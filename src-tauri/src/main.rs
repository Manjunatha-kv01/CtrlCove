mod agent;
mod ai;
mod assistant;
mod autonomous;
mod clipboard;
mod database;
mod error;
mod graph;
mod insight_trail;
mod operations;
mod pcos;
mod platform;
mod privacy;
mod semantic;
mod terminal_history;
mod validation;
mod workspace;

use arboard::{Clipboard, ImageData};
use database::{
    AgentWorkflowRecord, ApiClient, AuditLog, AutomationRunResult, AutomationTask, ClipboardItem,
    ClipboardStats, CognitiveModule, CognitiveOverview, CognitiveReleaseResult, CognitiveUseCase,
    Collection, Database, DatabaseBackup, DatabaseBackupSnapshot,
    DatabaseBackupVerificationRequest, DatabaseReliabilityChecksum,
    DatabaseReliabilityReportExport, DatabaseReliabilityReportSnapshot, DatabaseReliabilityStatus,
    EnterpriseControl, IntegrationConnector, IntelligenceReport, KnowledgeGraph, KnowledgeHealth,
    PlatformSummary, PluginRecord, SearchRequest, SmartNotification, SyncDevice,
    TeamSharingDeviceRequest, TeamSharingDeviceStatusRequest, TeamSharingManifestInspection,
    TeamSharingManifestInspectionRequest, TeamSharingManifestLedgerChecksum,
    TeamSharingManifestLedgerExportRequest, TeamSharingPolicy, TeamSharingReadiness,
    TeamSharingSyncDryRun, UniversalSyncResult, VaultRetentionResult, VaultRetentionSettings,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use error::AppError;
use image::ImageReader;
use insight_trail::{
    InsightIncident, InsightTrailEvent, InsightTrailNoteRequest, InsightTrailOverview,
    InsightTrailSearchRequest, InsightTrailSettings,
};
use privacy::{PrivacySettings, PrivacyStatus, TextCaptureDecision};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use terminal_history::TerminalHistoryImport;
use tokio::sync::{Mutex, Semaphore};
use workspace::{
    CognitiveWorkspace, HandoffRecipientRevokeRequest, HandoffRecipientTrustRecord,
    HandoffRecipientTrustRequest, HandoffSignerRevokeRequest, HandoffSignerTrustRecord,
    HandoffSignerTrustRequest, IncidentEvidenceLinkRequest, IncidentReopenRequest,
    IncidentResolutionRequest, ManualRunbookRequest, ManualRunbookReviewRequest,
    ManualRunbookRevisionRestoreRequest, ManualRunbookUpdateRequest, RunbookEntry, RunbookRevision,
    RunbookSearchRequest, WorkspaceContextUpdate, WorkspaceCreateRequest,
    WorkspaceDocumentImportRequest, WorkspaceDocumentImportResult, WorkspaceHandoffExportRecord,
    WorkspaceHandoffImportRequest, WorkspaceHandoffInspectionRecord, WorkspaceHandoffRequest,
    WorkspaceReportRequest, WorkspaceSessionStartRequest, WorkspaceSnapshot,
};

pub struct AppState {
    pub(crate) database: Database,
    pub(crate) maintenance_lock: Arc<Mutex<()>>,
    pub(crate) graph_lock: Arc<Mutex<()>>,
    pub(crate) ingestion_limiter: Arc<Semaphore>,
}

#[derive(Serialize)]
struct ExportResult {
    path: String,
}

#[derive(serde::Deserialize)]
struct AssistantQuestion {
    question: String,
}

#[derive(serde::Deserialize)]
struct TerminalHistoryRequest {
    shell: String,
    max_entries: usize,
}

#[derive(Serialize)]
struct TerminalHistoryImportResult {
    shell: String,
    available: usize,
    selected: usize,
    imported: usize,
    skipped_sensitive: usize,
    skipped_irrelevant: usize,
}

#[derive(Deserialize)]
struct HandoffInspectionRequest {
    content: String,
}

#[derive(Debug, Serialize)]
struct HandoffInspection {
    workspace_name: String,
    project: String,
    scope: String,
    recipient: String,
    purpose: String,
    classification: String,
    expires_at_unix: Option<u64>,
    is_expired: bool,
    generated_locally_at: String,
    event_count: usize,
    incident_count: usize,
    resolution_count: usize,
    checksum: String,
    signature_verified: bool,
    signer_fingerprint: Option<String>,
    signature_status: String,
}

struct HandoffSigner {
    signing_key: SigningKey,
    fingerprint: String,
}

#[derive(Serialize)]
struct WorkspaceHandoffReadiness {
    safe: bool,
    scope: String,
    event_count: usize,
    excluded_event_count: usize,
    incident_count: usize,
    resolution_count: usize,
    estimated_bytes: usize,
    blocking_findings: usize,
    blockers: Vec<String>,
}

fn storage_error(action: &str, error: impl std::fmt::Display) -> String {
    let error = AppError::storage(action, error.to_string());
    error.log();
    error.user_message()
}

#[tauri::command]
async fn search_clipboard_items(
    request: SearchRequest,
    state: State<'_, AppState>,
) -> Result<Vec<ClipboardItem>, String> {
    validation::search(&request)?;
    state
        .database
        .search_items(&request)
        .await
        .map_err(|err| storage_error("Memory search", err))
}

#[tauri::command]
async fn capture_browser_bookmark(
    request: clipboard::BrowserBookmarkRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<i64>, String> {
    validation::browser_bookmark(&request)?;
    let settings = state
        .database
        .privacy_settings()
        .await
        .map_err(|err| storage_error("Browser bookmark privacy check", err))?;
    let privacy_candidate = format!(
        "{}\n{}\n{}",
        request.url,
        request.title,
        request.tags.join(" ")
    );
    match privacy::text_capture_decision(&settings, &privacy_candidate) {
        TextCaptureDecision::Allow => {}
        TextCaptureDecision::Skip => {
            return Err(
                "Capture privacy settings do not allow browser bookmark capture.".to_string(),
            )
        }
        TextCaptureDecision::Block(reason) => {
            state
                .database
                .record_privacy_block("Browser bookmark", &reason)
                .await
                .map_err(|err| storage_error("Browser bookmark privacy audit", err))?;
            return Err("Browser bookmark was blocked by local capture privacy.".to_string());
        }
    }
    let item = clipboard::browser_bookmark_item(&request).await;
    let _graph_guard = state.graph_lock.lock().await;
    let memory_id = state
        .database
        .insert_item(item)
        .await
        .map_err(|err| storage_error("Browser bookmark capture", err))?;
    if memory_id.is_some() {
        let _ = app.emit("clipboard-item-created", ());
    }
    Ok(memory_id)
}

#[tauri::command]
async fn capture_ide_snippet(
    request: clipboard::IdeSnippetRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<i64>, String> {
    validation::ide_snippet(&request)?;
    let settings = state
        .database
        .privacy_settings()
        .await
        .map_err(|err| storage_error("IDE snippet privacy check", err))?;
    let privacy_candidate = format!(
        "{}\n{}\n{}\n{}\n{}",
        request.content,
        request.title,
        request.project,
        request.file_path,
        request.tags.join(" ")
    );
    match privacy::text_capture_decision(&settings, &privacy_candidate) {
        TextCaptureDecision::Allow => {}
        TextCaptureDecision::Skip => {
            return Err("Capture privacy settings do not allow IDE snippet capture.".to_string())
        }
        TextCaptureDecision::Block(reason) => {
            state
                .database
                .record_privacy_block("IDE snippet", &reason)
                .await
                .map_err(|err| storage_error("IDE snippet privacy audit", err))?;
            return Err("IDE snippet was blocked by local capture privacy.".to_string());
        }
    }
    let item = clipboard::ide_snippet_item(&request).await;
    let _graph_guard = state.graph_lock.lock().await;
    let memory_id = state
        .database
        .insert_item(item)
        .await
        .map_err(|err| storage_error("IDE snippet capture", err))?;
    if memory_id.is_some() {
        let _ = app.emit("clipboard-item-created", ());
    }
    Ok(memory_id)
}

#[tauri::command]
async fn capture_terminal_command(
    request: clipboard::TerminalCommandRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<i64>, String> {
    validation::terminal_command(&request)?;
    let settings = state
        .database
        .privacy_settings()
        .await
        .map_err(|err| storage_error("Terminal command privacy check", err))?;
    let privacy_candidate = format!(
        "{}\n{}\n{}\n{}",
        request.command,
        request.host,
        request.project,
        request.tags.join(" ")
    );
    match privacy::text_capture_decision(&settings, &privacy_candidate) {
        TextCaptureDecision::Allow => {}
        TextCaptureDecision::Skip => {
            return Err(
                "Capture privacy settings do not allow terminal command capture.".to_string(),
            )
        }
        TextCaptureDecision::Block(reason) => {
            state
                .database
                .record_privacy_block("Terminal command", &reason)
                .await
                .map_err(|err| storage_error("Terminal command privacy audit", err))?;
            return Err("Terminal command was blocked by local capture privacy.".to_string());
        }
    }
    let item = clipboard::terminal_command_item(&request);
    let _graph_guard = state.graph_lock.lock().await;
    let memory_id = state
        .database
        .insert_item(item)
        .await
        .map_err(|err| storage_error("Terminal command capture", err))?;
    if memory_id.is_some() {
        let _ = app.emit("clipboard-item-created", ());
    }
    Ok(memory_id)
}

#[tauri::command]
async fn get_collections(state: State<'_, AppState>) -> Result<Vec<Collection>, String> {
    state
        .database
        .collections()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn create_collection(
    name: String,
    color: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::collection(&name, &color)?;
    state
        .database
        .create_collection(&name, &color)
        .await
        .map_err(|err| storage_error("Collection creation", err))
}

#[tauri::command]
async fn move_item_to_collection(
    item_id: i64,
    collection_id: Option<i64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::item_id(item_id)?;
    if let Some(collection_id) = collection_id {
        validation::item_id(collection_id)?;
    }
    state
        .database
        .move_to_collection(item_id, collection_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn toggle_favorite(item_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    validation::item_id(item_id)?;
    state
        .database
        .toggle_favorite(item_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn delete_clipboard_item(item_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    validation::item_id(item_id)?;
    state
        .database
        .delete_item(item_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_clipboard_stats(state: State<'_, AppState>) -> Result<ClipboardStats, String> {
    state.database.stats().await.map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_insight_trail_events(
    request: InsightTrailSearchRequest,
    state: State<'_, AppState>,
) -> Result<Vec<InsightTrailEvent>, String> {
    validation::insight_trail_search(&request)?;
    state
        .database
        .insight_trail_events(&request)
        .await
        .map_err(|err| storage_error("InsightTrail timeline", err))
}

#[tauri::command]
async fn get_insight_trail_overview(
    state: State<'_, AppState>,
) -> Result<InsightTrailOverview, String> {
    state
        .database
        .insight_trail_overview()
        .await
        .map_err(|err| storage_error("InsightTrail overview", err))
}

#[tauri::command]
async fn get_insight_trail_settings(
    state: State<'_, AppState>,
) -> Result<InsightTrailSettings, String> {
    state
        .database
        .insight_trail_settings()
        .await
        .map_err(|err| storage_error("InsightTrail settings", err))
}

#[tauri::command]
async fn update_insight_trail_settings(
    settings: InsightTrailSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::insight_trail_settings(&settings)?;
    state
        .database
        .update_insight_trail_settings(&settings)
        .await
        .map_err(|err| storage_error("InsightTrail settings update", err))
}

#[tauri::command]
async fn record_insight_trail_note(
    request: InsightTrailNoteRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::insight_trail_note(&request)?;
    state
        .database
        .record_insight_trail_note(&request.title, &request.details, &request.tags)
        .await
        .map_err(|err| storage_error("InsightTrail note", err))
}

#[tauri::command]
async fn get_insight_incidents(state: State<'_, AppState>) -> Result<Vec<InsightIncident>, String> {
    state
        .database
        .insight_incidents()
        .await
        .map_err(|err| storage_error("InsightTrail incidents", err))
}

#[tauri::command]
async fn resolve_insight_incident(
    incident_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::item_id(incident_id)?;
    state
        .database
        .resolve_insight_incident(incident_id)
        .await
        .map_err(|err| storage_error("InsightTrail incident resolution", err))
}

#[tauri::command]
async fn apply_insight_trail_retention(state: State<'_, AppState>) -> Result<i64, String> {
    state
        .database
        .prune_insight_trail()
        .await
        .map_err(|err| storage_error("InsightTrail retention", err))
}

#[tauri::command]
async fn get_cognitive_workspace(state: State<'_, AppState>) -> Result<CognitiveWorkspace, String> {
    state
        .database
        .cognitive_workspace(None)
        .await
        .map_err(|err| storage_error("Cognitive workspace", err))
}

#[tauri::command]
async fn update_cognitive_workspace(
    workspace_id: i64,
    update: WorkspaceContextUpdate,
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, String> {
    validation::item_id(workspace_id)?;
    validation::workspace_context(&update)?;
    state
        .database
        .update_cognitive_workspace(workspace_id, &update)
        .await
        .map_err(|err| storage_error("Cognitive workspace update", err))
}

#[tauri::command]
async fn get_cognitive_workspaces(
    state: State<'_, AppState>,
) -> Result<Vec<CognitiveWorkspace>, String> {
    state
        .database
        .cognitive_workspaces()
        .await
        .map_err(|err| storage_error("Cognitive workspaces", err))
}

#[tauri::command]
async fn get_workspace_snapshot(
    workspace_id: Option<i64>,
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, String> {
    if let Some(workspace_id) = workspace_id {
        validation::item_id(workspace_id)?;
    }
    state
        .database
        .workspace_snapshot(workspace_id)
        .await
        .map_err(|err| storage_error("Workspace timeline", err))
}

#[tauri::command]
async fn create_cognitive_workspace(
    request: WorkspaceCreateRequest,
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, String> {
    validation::workspace_create(&request)?;
    state
        .database
        .create_cognitive_workspace(&request)
        .await
        .map_err(|err| storage_error("Workspace creation", err))
}

#[tauri::command]
async fn start_workspace_session(
    request: WorkspaceSessionStartRequest,
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, String> {
    validation::workspace_session_start(&request)?;
    state
        .database
        .start_workspace_session(&request)
        .await
        .map_err(|err| storage_error("Workspace session", err))
}

#[tauri::command]
async fn end_workspace_session(
    session_id: i64,
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, String> {
    validation::item_id(session_id)?;
    state
        .database
        .end_workspace_session(session_id)
        .await
        .map_err(|err| storage_error("Workspace session", err))
}

#[tauri::command]
async fn archive_cognitive_workspace(
    workspace_id: i64,
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, String> {
    validation::item_id(workspace_id)?;
    state
        .database
        .archive_cognitive_workspace(workspace_id)
        .await
        .map_err(|err| storage_error("Workspace archive", err))
}

#[tauri::command]
async fn restore_cognitive_workspace(
    workspace_id: i64,
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, String> {
    validation::item_id(workspace_id)?;
    state
        .database
        .restore_cognitive_workspace(workspace_id)
        .await
        .map_err(|err| storage_error("Workspace restore", err))
}

#[tauri::command]
async fn import_workspace_document(
    request: WorkspaceDocumentImportRequest,
    state: State<'_, AppState>,
) -> Result<WorkspaceDocumentImportResult, String> {
    validation::workspace_document_import(&request)?;
    let _permit = state
        .ingestion_limiter
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| "Document import is unavailable. Please retry.".to_string())?;
    let _graph_guard = state.graph_lock.lock().await;
    state
        .database
        .import_workspace_document(&request)
        .await
        .map_err(|err| storage_error("Document import", err))
}

#[tauri::command]
async fn record_workspace_incident_resolution(
    request: IncidentResolutionRequest,
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, String> {
    validation::incident_resolution(&request)?;
    state
        .database
        .record_incident_resolution(&request)
        .await
        .map_err(|err| storage_error("Incident resolution", err))
}

#[tauri::command]
async fn reopen_workspace_incident(
    request: IncidentReopenRequest,
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, String> {
    validation::incident_reopen(&request)?;
    state
        .database
        .reopen_incident(&request)
        .await
        .map_err(|err| storage_error("Incident reopening", err))
}

#[tauri::command]
async fn link_workspace_incident_evidence(
    request: IncidentEvidenceLinkRequest,
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, String> {
    validation::incident_evidence_link(&request)?;
    state
        .database
        .link_workspace_event_to_incident(&request)
        .await
        .map_err(|err| storage_error("Incident evidence link", err))
}

#[tauri::command]
async fn export_workspace_report(
    request: WorkspaceReportRequest,
    state: State<'_, AppState>,
) -> Result<ExportResult, String> {
    validation::workspace_report(&request)?;
    let snapshot = state
        .database
        .workspace_snapshot(Some(request.workspace_id))
        .await
        .map_err(|err| storage_error("Workspace report", err))?;
    if let Some(session_id) = request.session_id {
        if !snapshot
            .sessions
            .iter()
            .any(|session| session.id == session_id)
        {
            return Err("The selected session does not belong to this workspace.".to_string());
        }
    }

    let export_dir = state.database.exports_dir();
    fs::create_dir_all(&export_dir).map_err(|err| err.to_string())?;
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_secs();
    let path = write_export(
        export_dir.join(format!(
            "workspace-{}-report-{}.md",
            request.workspace_id, created_at
        )),
        workspace_report_markdown(&snapshot, request.session_id),
    )?;
    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn export_workspace_handoff(
    request: WorkspaceHandoffRequest,
    state: State<'_, AppState>,
) -> Result<ExportResult, String> {
    validation::workspace_handoff(&request)?;
    let snapshot = state
        .database
        .workspace_snapshot(Some(request.workspace_id))
        .await
        .map_err(|err| storage_error("Workspace handoff export", err))?;
    validate_trusted_handoff_recipient(&state.database, &request).await?;
    validate_workspace_handoff_scope(&snapshot, &request)?;
    let readiness = workspace_handoff_readiness(&snapshot, &request)?;
    if !readiness.safe {
        let reason = readiness.blockers.join("; ");
        state
            .database
            .record_privacy_block("Workspace handoff export", &reason)
            .await
            .map_err(|err| storage_error("Workspace handoff safety audit", err))?;
        return Err(
            "Handoff safety review blocked export. Remove the flagged sensitive content or reduce the selected scope."
                .to_string(),
        );
    }

    let export_dir = state.database.exports_dir();
    fs::create_dir_all(&export_dir).map_err(|err| err.to_string())?;
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_secs();
    let (expires_at_unix, _) = handoff_expiry(&created_at.to_string(), request.expires_in_days)?;
    let signer = handoff_signer(&state.database.handoff_signing_key_path())?;
    let handoff = workspace_handoff_json(&snapshot, &request, &created_at.to_string(), &signer)?;
    let package_sha256 = crate::clipboard::hash_bytes(handoff.as_bytes());
    let package_bytes = handoff.len() as i64;
    let path = write_export(
        export_dir.join(format!(
            "workspace-{}-handoff-{}.json",
            request.workspace_id, created_at
        )),
        handoff,
    )?;
    if let Err(error) = state
        .database
        .record_workspace_handoff_export(
            request.workspace_id,
            request.session_id,
            &readiness.scope,
            request.recipient.trim(),
            request.purpose.trim(),
            &request.classification,
            expires_at_unix.map(|value| value as i64),
            &signer.fingerprint,
            &package_sha256,
            package_bytes,
            readiness.event_count as i64,
            readiness.excluded_event_count as i64,
            readiness.incident_count as i64,
            readiness.resolution_count as i64,
        )
        .await
    {
        let _ = fs::remove_file(&path);
        return Err(storage_error("Workspace handoff export audit", error));
    }
    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn get_workspace_handoff_exports(
    workspace_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceHandoffExportRecord>, String> {
    validation::item_id(workspace_id)?;
    state
        .database
        .workspace_handoff_exports(workspace_id)
        .await
        .map_err(|err| storage_error("Workspace handoff audit", err))
}

#[tauri::command]
async fn get_handoff_recipient_trust_records(
    state: State<'_, AppState>,
) -> Result<Vec<HandoffRecipientTrustRecord>, String> {
    state
        .database
        .handoff_recipient_trust_records()
        .await
        .map_err(|err| storage_error("Handoff recipient trust", err))
}

#[tauri::command]
async fn trust_handoff_recipient(
    request: HandoffRecipientTrustRequest,
    state: State<'_, AppState>,
) -> Result<HandoffRecipientTrustRecord, String> {
    validation::handoff_recipient_trust(&request)?;
    state
        .database
        .trust_handoff_recipient(
            request.recipient.trim(),
            &request.max_classification,
            request.note.trim(),
        )
        .await
        .map_err(|err| storage_error("Handoff recipient trust", err))
}

#[tauri::command]
async fn revoke_handoff_recipient(
    request: HandoffRecipientRevokeRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::handoff_recipient_revoke(&request)?;
    state
        .database
        .revoke_handoff_recipient(request.recipient.trim())
        .await
        .map_err(|err| storage_error("Handoff recipient revocation", err))
}

#[tauri::command]
async fn get_handoff_signer_trust_records(
    state: State<'_, AppState>,
) -> Result<Vec<HandoffSignerTrustRecord>, String> {
    state
        .database
        .handoff_signer_trust_records()
        .await
        .map_err(|err| storage_error("Handoff signer trust", err))
}

#[tauri::command]
async fn trust_handoff_signer(
    request: HandoffSignerTrustRequest,
    state: State<'_, AppState>,
) -> Result<HandoffSignerTrustRecord, String> {
    validation::handoff_signer_trust(&request)?;
    state
        .database
        .trust_handoff_signer(request.signer_fingerprint.trim(), request.label.trim())
        .await
        .map_err(|err| storage_error("Handoff signer trust", err))
}

#[tauri::command]
async fn revoke_handoff_signer(
    request: HandoffSignerRevokeRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::handoff_signer_revoke(&request)?;
    state
        .database
        .revoke_handoff_signer(request.signer_fingerprint.trim())
        .await
        .map_err(|err| storage_error("Handoff signer revocation", err))
}

#[tauri::command]
async fn trust_current_device_team_sharing_signer(
    state: State<'_, AppState>,
) -> Result<HandoffSignerTrustRecord, String> {
    let signer = handoff_signer(&state.database.handoff_signing_key_path())?;
    state
        .database
        .trust_handoff_signer(&signer.fingerprint, "Current CYMOS device")
        .await
        .map_err(|err| storage_error("Current device signer trust", err))
}

#[tauri::command]
async fn get_workspace_handoff_readiness(
    request: WorkspaceHandoffRequest,
    state: State<'_, AppState>,
) -> Result<WorkspaceHandoffReadiness, String> {
    validation::workspace_handoff(&request)?;
    let snapshot = state
        .database
        .workspace_snapshot(Some(request.workspace_id))
        .await
        .map_err(|err| storage_error("Workspace handoff readiness", err))?;
    validate_workspace_handoff_scope(&snapshot, &request)?;
    let mut readiness = workspace_handoff_readiness(&snapshot, &request)?;
    if let Err(blocker) = validate_trusted_handoff_recipient(&state.database, &request).await {
        readiness.safe = false;
        readiness.blocking_findings += 1;
        readiness.blockers.push(blocker);
    }
    Ok(readiness)
}

#[tauri::command]
async fn inspect_workspace_handoff(
    request: HandoffInspectionRequest,
    state: State<'_, AppState>,
) -> Result<HandoffInspection, String> {
    let package_sha256 = crate::clipboard::hash_bytes(request.content.as_bytes());
    let package_bytes = request.content.len() as i64;
    let result = validation::handoff_package(&request.content)
        .and_then(|_| inspect_handoff_package(&request.content));
    match result {
        Ok(inspection) => {
            let status = if inspection.is_expired {
                "Expired"
            } else {
                "Verified"
            };
            state
                .database
                .record_workspace_handoff_inspection(
                    status,
                    Some(&inspection.workspace_name),
                    Some(&inspection.classification),
                    inspection.signer_fingerprint.as_deref(),
                    &package_sha256,
                    Some(&inspection.checksum),
                    None,
                    package_bytes,
                )
                .await
                .map_err(|err| storage_error("Handoff inspection audit", err))?;
            Ok(inspection)
        }
        Err(error) => {
            state
                .database
                .record_workspace_handoff_inspection(
                    "Rejected",
                    None,
                    None,
                    None,
                    &package_sha256,
                    None,
                    Some(&error),
                    package_bytes,
                )
                .await
                .map_err(|err| storage_error("Handoff inspection audit", err))?;
            Err(error)
        }
    }
}

#[tauri::command]
async fn get_workspace_handoff_inspections(
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceHandoffInspectionRecord>, String> {
    state
        .database
        .workspace_handoff_inspection_records()
        .await
        .map_err(|err| storage_error("Handoff inspection audit", err))
}

#[tauri::command]
async fn import_workspace_handoff(
    request: WorkspaceHandoffImportRequest,
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, String> {
    validation::handoff_package(&request.content)?;
    let inspection = inspect_handoff_package(&request.content)?;
    if inspection.is_expired {
        return Err("This handoff package has expired and cannot be imported.".to_string());
    }
    if inspection.classification == "Confidential" {
        let Some(fingerprint) = inspection.signer_fingerprint.as_deref() else {
            return Err("Confidential handoff packages must be signed.".to_string());
        };
        if state
            .database
            .trusted_handoff_signer(fingerprint)
            .await
            .map_err(|err| storage_error("Handoff signer trust", err))?
            .is_none()
        {
            return Err(
                "Confidential handoff signer is not trusted locally. Trust the signer before import."
                    .to_string(),
            );
        }
    }
    let settings = state
        .database
        .privacy_settings()
        .await
        .map_err(|err| storage_error("Handoff privacy check", err))?;
    match privacy::text_capture_decision(&settings, &request.content) {
        TextCaptureDecision::Allow => {}
        TextCaptureDecision::Skip => {
            return Err("Capture privacy settings do not allow handoff import.".to_string())
        }
        TextCaptureDecision::Block(reason) => {
            state
                .database
                .record_privacy_block("Workspace handoff import", &reason)
                .await
                .map_err(|err| storage_error("Handoff privacy audit", err))?;
            return Err("Handoff import was blocked by local capture privacy.".to_string());
        }
    }
    let package: serde_json::Value = serde_json::from_str(&request.content)
        .map_err(|_| "The selected file is not a valid CYMOS handoff package.".to_string())?;
    state
        .database
        .import_workspace_handoff(&package)
        .await
        .map_err(|err| storage_error("Workspace handoff import", err))
}

#[tauri::command]
async fn search_runbook_entries(
    request: RunbookSearchRequest,
    state: State<'_, AppState>,
) -> Result<Vec<RunbookEntry>, String> {
    validation::runbook_search(&request)?;
    state
        .database
        .runbook_entries(&request)
        .await
        .map_err(|err| storage_error("Runbook search", err))
}

#[tauri::command]
async fn get_runbook_audit_logs(state: State<'_, AppState>) -> Result<Vec<AuditLog>, String> {
    state
        .database
        .runbook_audit_logs()
        .await
        .map_err(|err| storage_error("Runbook audit trail", err))
}

#[tauri::command]
async fn create_manual_runbook(
    request: ManualRunbookRequest,
    state: State<'_, AppState>,
) -> Result<RunbookEntry, String> {
    validation::manual_runbook(&request)?;
    state
        .database
        .create_manual_runbook(&request)
        .await
        .map_err(|err| storage_error("Runbook creation", err))
}

#[tauri::command]
async fn update_manual_runbook(
    request: ManualRunbookUpdateRequest,
    state: State<'_, AppState>,
) -> Result<RunbookEntry, String> {
    validation::manual_runbook_update(&request)?;
    state
        .database
        .update_manual_runbook(&request)
        .await
        .map_err(|err| storage_error("Runbook update", err))
}

#[tauri::command]
async fn delete_manual_runbook(entry_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    validation::manual_runbook_delete(entry_id)?;
    state
        .database
        .delete_manual_runbook(entry_id)
        .await
        .map_err(|err| storage_error("Runbook deletion", err))
}

#[tauri::command]
async fn export_runbook_entry(
    entry_id: i64,
    state: State<'_, AppState>,
) -> Result<ExportResult, String> {
    validation::runbook_entry_id(entry_id)?;
    let entry = state
        .database
        .runbook_entry(entry_id)
        .await
        .map_err(|err| storage_error("Runbook export", err))?;
    let export_dir = state.database.exports_dir();
    fs::create_dir_all(&export_dir).map_err(|err| err.to_string())?;
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_secs();
    let path = write_export(
        export_dir.join(format!("runbook-{}-{}.md", entry.id, created_at)),
        runbook_entry_markdown(&entry),
    )?;
    state
        .database
        .record_runbook_export(&entry)
        .await
        .map_err(|err| storage_error("Runbook export audit", err))?;
    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn copy_runbook_entry(entry_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    validation::runbook_entry_id(entry_id)?;
    let entry = state
        .database
        .runbook_entry(entry_id)
        .await
        .map_err(|err| storage_error("Runbook copy", err))?;
    let mut clipboard = Clipboard::new().map_err(|err| err.to_string())?;
    clipboard
        .set_text(&entry.details)
        .map_err(|err| err.to_string())?;
    state
        .database
        .record_runbook_copy(&entry)
        .await
        .map_err(|err| storage_error("Runbook copy audit", err))
}

#[tauri::command]
async fn get_manual_runbook_revisions(
    entry_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<RunbookRevision>, String> {
    validation::manual_runbook_delete(entry_id)?;
    state
        .database
        .manual_runbook_revisions(entry_id)
        .await
        .map_err(|err| storage_error("Runbook revision history", err))
}

#[tauri::command]
async fn restore_manual_runbook_revision(
    request: ManualRunbookRevisionRestoreRequest,
    state: State<'_, AppState>,
) -> Result<RunbookEntry, String> {
    validation::manual_runbook_revision_restore(&request)?;
    state
        .database
        .restore_manual_runbook_revision(&request)
        .await
        .map_err(|err| storage_error("Runbook revision restore", err))
}

#[tauri::command]
async fn review_manual_runbook(
    request: ManualRunbookReviewRequest,
    state: State<'_, AppState>,
) -> Result<RunbookEntry, String> {
    validation::manual_runbook_review(&request)?;
    state
        .database
        .review_manual_runbook(&request)
        .await
        .map_err(|err| storage_error("Runbook review", err))
}

#[tauri::command]
async fn get_similar_memories(
    item_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<ClipboardItem>, String> {
    validation::item_id(item_id)?;
    state
        .database
        .similar_items(item_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn import_terminal_history(
    request: TerminalHistoryRequest,
    state: State<'_, AppState>,
) -> Result<TerminalHistoryImportResult, String> {
    validation::terminal_shell(&request.shell)?;
    validation::terminal_history_limit(request.max_entries)?;
    let history: TerminalHistoryImport = terminal_history::read_history(&request.shell)?;
    let available = history.commands.len();
    let commands = terminal_history::newest_commands(&history.commands, request.max_entries);
    let _graph_guard = state.graph_lock.lock().await;
    let mut imported = 0;

    for command in &commands {
        let inserted = state
            .database
            .insert_item(clipboard::terminal_history_item(command, &history.shell))
            .await
            .map_err(|error| storage_error("Terminal history import", error))?;
        if inserted.is_some() {
            imported += 1;
        }
    }

    Ok(TerminalHistoryImportResult {
        shell: history.shell,
        available,
        selected: commands.len(),
        imported,
        skipped_sensitive: history.skipped_sensitive,
        skipped_irrelevant: history.skipped_irrelevant,
    })
}

#[tauri::command]
async fn rebuild_semantic_index(state: State<'_, AppState>) -> Result<(), String> {
    state
        .database
        .rebuild_semantic_index()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_knowledge_graph(state: State<'_, AppState>) -> Result<KnowledgeGraph, String> {
    state
        .database
        .knowledge_graph()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn rebuild_knowledge_graph(state: State<'_, AppState>) -> Result<(), String> {
    let _graph_guard = state.graph_lock.lock().await;
    state
        .database
        .rebuild_knowledge_graph()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn ask_memory_assistant(
    request: AssistantQuestion,
    state: State<'_, AppState>,
) -> Result<assistant::AssistantResponse, String> {
    validation::assistant_question(&request.question)?;
    let search = SearchRequest {
        query: request.question.clone(),
        content_type: "All".to_string(),
        favorite_only: false,
        collection_id: None,
        tag: "All".to_string(),
        category: "All".to_string(),
        semantic: true,
    };
    let memories = state
        .database
        .search_items(&search)
        .await
        .map_err(|err| err.to_string())?;
    let graph = state
        .database
        .knowledge_graph()
        .await
        .map_err(|err| err.to_string())?;

    Ok(assistant::answer_question(&request.question, &memories, &graph).await)
}

#[tauri::command]
async fn get_daily_knowledge_summary(
    state: State<'_, AppState>,
) -> Result<assistant::KnowledgeDigest, String> {
    let search = SearchRequest {
        query: String::new(),
        content_type: "All".to_string(),
        favorite_only: false,
        collection_id: None,
        tag: "All".to_string(),
        category: "All".to_string(),
        semantic: true,
    };
    let memories = state
        .database
        .search_items(&search)
        .await
        .map_err(|err| err.to_string())?;
    let graph = state
        .database
        .knowledge_graph()
        .await
        .map_err(|err| err.to_string())?;
    Ok(assistant::daily_summary(&memories, &graph))
}

#[tauri::command]
async fn get_weekly_learning_report(
    state: State<'_, AppState>,
) -> Result<assistant::KnowledgeDigest, String> {
    let search = SearchRequest {
        query: String::new(),
        content_type: "All".to_string(),
        favorite_only: false,
        collection_id: None,
        tag: "All".to_string(),
        category: "All".to_string(),
        semantic: true,
    };
    let memories = state
        .database
        .search_items(&search)
        .await
        .map_err(|err| err.to_string())?;
    let graph = state
        .database
        .knowledge_graph()
        .await
        .map_err(|err| err.to_string())?;
    Ok(assistant::weekly_report(&memories, &graph))
}

#[tauri::command]
async fn run_agent_workflow(
    request: agent::AgentRequest,
    state: State<'_, AppState>,
) -> Result<agent::AgentWorkflow, String> {
    validation::agent_goal(&request.goal)?;
    let search = SearchRequest {
        query: request.goal.clone(),
        content_type: "All".to_string(),
        favorite_only: false,
        collection_id: None,
        tag: "All".to_string(),
        category: "All".to_string(),
        semantic: true,
    };
    let memories = state
        .database
        .search_items(&search)
        .await
        .map_err(|err| err.to_string())?;
    let graph = state
        .database
        .knowledge_graph()
        .await
        .map_err(|err| err.to_string())?;
    let mut workflow = agent::run_workflow(&request.goal, &memories, &graph);
    let workflow_id = state
        .database
        .save_agent_workflow(&workflow)
        .await
        .map_err(|err| err.to_string())?;
    workflow.id = Some(workflow_id);
    Ok(workflow)
}

#[tauri::command]
async fn get_agent_workflows(
    state: State<'_, AppState>,
) -> Result<Vec<AgentWorkflowRecord>, String> {
    state
        .database
        .agent_workflows()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn run_autonomous_cycle(state: State<'_, AppState>) -> Result<AutomationRunResult, String> {
    let _maintenance_guard = state.maintenance_lock.lock().await;
    let _graph_guard = state.graph_lock.lock().await;
    state
        .database
        .run_autonomous_cycle()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_knowledge_health(state: State<'_, AppState>) -> Result<KnowledgeHealth, String> {
    state
        .database
        .knowledge_health()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_database_reliability(
    state: State<'_, AppState>,
) -> Result<DatabaseReliabilityStatus, String> {
    let _maintenance_guard = state.maintenance_lock.lock().await;
    state
        .database
        .database_reliability()
        .await
        .map_err(|err| storage_error("Database reliability check", err))
}

#[tauri::command]
async fn get_database_reliability_checksum(
    state: State<'_, AppState>,
) -> Result<DatabaseReliabilityChecksum, String> {
    let _maintenance_guard = state.maintenance_lock.lock().await;
    let reliability = state
        .database
        .database_reliability()
        .await
        .map_err(|err| storage_error("Database reliability checksum", err))?;
    let snapshots = state
        .database
        .recent_backup_snapshots()
        .map_err(|err| storage_error("Recent database backups", err))?;

    Ok(DatabaseReliabilityChecksum {
        integrity_status: reliability.integrity_status.clone(),
        snapshot_count: snapshots.len() as i64,
        report_data_sha256: database_reliability_checksum(&reliability, &snapshots),
    })
}

#[tauri::command]
async fn create_verified_backup(state: State<'_, AppState>) -> Result<DatabaseBackup, String> {
    let _maintenance_guard = state.maintenance_lock.lock().await;
    state
        .database
        .create_verified_backup()
        .await
        .map_err(|err| storage_error("Verified database backup", err))
}

#[tauri::command]
async fn verify_latest_backup(state: State<'_, AppState>) -> Result<DatabaseBackup, String> {
    let _maintenance_guard = state.maintenance_lock.lock().await;
    state
        .database
        .verify_latest_backup()
        .await
        .map_err(|err| storage_error("Latest database backup verification", err))
}

#[tauri::command]
async fn verify_database_backup_snapshot(
    request: DatabaseBackupVerificationRequest,
    state: State<'_, AppState>,
) -> Result<DatabaseBackup, String> {
    let _maintenance_guard = state.maintenance_lock.lock().await;
    state
        .database
        .verify_backup_snapshot(&request.file_name)
        .await
        .map_err(|err| storage_error("Selected database backup verification", err))
}

#[tauri::command]
async fn get_recent_database_backups(
    state: State<'_, AppState>,
) -> Result<Vec<DatabaseBackupSnapshot>, String> {
    let _maintenance_guard = state.maintenance_lock.lock().await;
    state
        .database
        .recent_backup_snapshots()
        .map_err(|err| storage_error("Recent database backups", err))
}

#[tauri::command]
async fn get_recent_database_reliability_reports(
    state: State<'_, AppState>,
) -> Result<Vec<DatabaseReliabilityReportSnapshot>, String> {
    let _maintenance_guard = state.maintenance_lock.lock().await;
    state
        .database
        .recent_database_reliability_reports()
        .map_err(|err| storage_error("Recent vault reliability reports", err))
}

#[tauri::command]
async fn export_database_reliability_report(
    state: State<'_, AppState>,
) -> Result<DatabaseReliabilityReportExport, String> {
    let _maintenance_guard = state.maintenance_lock.lock().await;
    let reliability = state
        .database
        .database_reliability()
        .await
        .map_err(|err| storage_error("Database reliability report", err))?;
    let snapshots = state
        .database
        .recent_backup_snapshots()
        .map_err(|err| storage_error("Recent database backups", err))?;
    let report_data_sha256 = database_reliability_checksum(&reliability, &snapshots);
    let integrity_status = reliability.integrity_status.clone();
    let snapshot_count = snapshots.len() as i64;
    let export_dir = state.database.exports_dir();
    fs::create_dir_all(&export_dir).map_err(|err| err.to_string())?;
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_micros();
    let path = write_export(
        export_dir.join(format!("vault-reliability-{created_at}.md")),
        database_reliability_markdown(&reliability, &snapshots),
    )?;
    let path_string = path.to_string_lossy().to_string();
    state
        .database
        .record_database_reliability_report_export(&path_string)
        .await
        .map_err(|err| storage_error("Vault reliability report audit", err))?;

    Ok(DatabaseReliabilityReportExport {
        path: path_string,
        integrity_status,
        snapshot_count,
        report_data_sha256,
    })
}

#[tauri::command]
async fn get_privacy_status(state: State<'_, AppState>) -> Result<PrivacyStatus, String> {
    state
        .database
        .privacy_status()
        .await
        .map_err(|err| storage_error("Capture privacy status", err))
}

#[tauri::command]
async fn update_privacy_settings(
    settings: PrivacySettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::privacy_settings(&settings)?;
    state
        .database
        .update_privacy_settings(&settings)
        .await
        .map_err(|err| storage_error("Capture privacy update", err))
}

#[tauri::command]
async fn get_vault_retention_settings(
    state: State<'_, AppState>,
) -> Result<VaultRetentionSettings, String> {
    state
        .database
        .vault_retention_settings()
        .await
        .map_err(|err| storage_error("Vault retention settings", err))
}

#[tauri::command]
async fn update_vault_retention_settings(
    settings: VaultRetentionSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::vault_retention_settings(&settings)?;
    state
        .database
        .update_vault_retention_settings(&settings)
        .await
        .map_err(|err| storage_error("Vault retention update", err))
}

#[tauri::command]
async fn apply_vault_retention(state: State<'_, AppState>) -> Result<VaultRetentionResult, String> {
    let _maintenance_guard = state.maintenance_lock.lock().await;
    let _graph_guard = state.graph_lock.lock().await;
    let result = state
        .database
        .apply_vault_retention()
        .await
        .map_err(|err| storage_error("Vault retention", err))?;
    if result.removed_items > 0 {
        state
            .database
            .rebuild_knowledge_graph()
            .await
            .map_err(|err| storage_error("Vault graph refresh", err))?;
    }
    Ok(result)
}

#[tauri::command]
async fn get_automation_tasks(state: State<'_, AppState>) -> Result<Vec<AutomationTask>, String> {
    state
        .database
        .automation_tasks()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_smart_notifications(
    state: State<'_, AppState>,
) -> Result<Vec<SmartNotification>, String> {
    state
        .database
        .smart_notifications()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_intelligence_reports(
    state: State<'_, AppState>,
) -> Result<Vec<IntelligenceReport>, String> {
    state
        .database
        .intelligence_reports()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn run_universal_sync_cycle(
    state: State<'_, AppState>,
) -> Result<UniversalSyncResult, String> {
    state
        .database
        .run_universal_sync_cycle()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_platform_summary(state: State<'_, AppState>) -> Result<PlatformSummary, String> {
    state
        .database
        .platform_summary()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_team_sharing_policy(state: State<'_, AppState>) -> Result<TeamSharingPolicy, String> {
    state
        .database
        .team_sharing_policy()
        .await
        .map_err(|err| storage_error("Team sharing policy", err))
}

#[tauri::command]
async fn update_team_sharing_policy(
    policy: TeamSharingPolicy,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::team_sharing_policy(&policy)?;
    state
        .database
        .update_team_sharing_policy(&policy)
        .await
        .map_err(|err| storage_error("Team sharing policy update", err))
}

#[tauri::command]
async fn get_team_sharing_readiness(
    state: State<'_, AppState>,
) -> Result<TeamSharingReadiness, String> {
    state
        .database
        .team_sharing_readiness()
        .await
        .map_err(|err| storage_error("Team sharing readiness", err))
}

#[tauri::command]
async fn run_team_sharing_sync_dry_run(
    state: State<'_, AppState>,
) -> Result<TeamSharingSyncDryRun, String> {
    state
        .database
        .team_sharing_sync_dry_run()
        .await
        .map_err(|err| storage_error("Team sharing sync dry run", err))
}

#[tauri::command]
async fn get_sync_devices(state: State<'_, AppState>) -> Result<Vec<SyncDevice>, String> {
    state
        .database
        .sync_devices()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn register_team_sharing_device(
    request: TeamSharingDeviceRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::team_sharing_device(&request)?;
    state
        .database
        .register_team_sharing_device(&request)
        .await
        .map_err(|err| storage_error("Team sharing device registration", err))
}

#[tauri::command]
async fn approve_team_sharing_device(
    request: TeamSharingDeviceStatusRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::team_sharing_device_status(&request)?;
    state
        .database
        .approve_team_sharing_device(&request)
        .await
        .map_err(|err| storage_error("Team sharing device approval", err))
}

#[tauri::command]
async fn revoke_team_sharing_device(
    request: TeamSharingDeviceStatusRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::team_sharing_device_status(&request)?;
    state
        .database
        .revoke_team_sharing_device(&request)
        .await
        .map_err(|err| storage_error("Team sharing device revocation", err))
}

#[tauri::command]
async fn get_integration_connectors(
    state: State<'_, AppState>,
) -> Result<Vec<IntegrationConnector>, String> {
    state
        .database
        .integration_connectors()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_plugin_records(state: State<'_, AppState>) -> Result<Vec<PluginRecord>, String> {
    state
        .database
        .plugin_records()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_api_clients(state: State<'_, AppState>) -> Result<Vec<ApiClient>, String> {
    state
        .database
        .api_clients()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_audit_logs(state: State<'_, AppState>) -> Result<Vec<AuditLog>, String> {
    state
        .database
        .audit_logs()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_team_sharing_audit_logs(state: State<'_, AppState>) -> Result<Vec<AuditLog>, String> {
    state
        .database
        .team_sharing_audit_logs()
        .await
        .map_err(|err| storage_error("Team sharing audit logs", err))
}

#[tauri::command]
async fn get_team_sharing_manifest_ledger_audit_logs(
    state: State<'_, AppState>,
) -> Result<Vec<AuditLog>, String> {
    state
        .database
        .team_sharing_manifest_ledger_audit_logs()
        .await
        .map_err(|err| storage_error("Team sharing manifest ledger audit logs", err))
}

#[tauri::command]
async fn export_team_sharing_readiness_report(
    state: State<'_, AppState>,
) -> Result<ExportResult, String> {
    let policy = state
        .database
        .team_sharing_policy()
        .await
        .map_err(|err| storage_error("Team sharing policy", err))?;
    let readiness = state
        .database
        .team_sharing_readiness()
        .await
        .map_err(|err| storage_error("Team sharing readiness", err))?;
    let devices = state
        .database
        .sync_devices()
        .await
        .map_err(|err| storage_error("Team sharing devices", err))?;
    let audit_logs = state
        .database
        .team_sharing_audit_logs()
        .await
        .map_err(|err| storage_error("Team sharing audit logs", err))?;

    let export_dir = state.database.exports_dir();
    fs::create_dir_all(&export_dir).map_err(|err| err.to_string())?;
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_secs();
    let path = write_export(
        export_dir.join(format!("team-sharing-readiness-{created_at}.md")),
        team_sharing_readiness_markdown(&policy, &readiness, &devices, &audit_logs),
    )?;
    let path_string = path.to_string_lossy().to_string();
    state
        .database
        .record_team_sharing_report_export(&path_string)
        .await
        .map_err(|err| storage_error("Team sharing report audit", err))?;

    Ok(ExportResult { path: path_string })
}

#[tauri::command]
async fn export_team_sharing_manifest_ledger(
    state: State<'_, AppState>,
) -> Result<ExportResult, String> {
    let audit_logs = state
        .database
        .team_sharing_manifest_ledger_audit_logs()
        .await
        .map_err(|err| storage_error("Team sharing audit logs", err))?;
    let manifest_logs = audit_logs
        .into_iter()
        .filter(|log| log.action.contains("manifest"))
        .collect::<Vec<_>>();

    let export_dir = state.database.exports_dir();
    fs::create_dir_all(&export_dir).map_err(|err| err.to_string())?;
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_secs();
    let path = write_export(
        export_dir.join(format!("team-sharing-manifest-ledger-{created_at}.md")),
        team_sharing_manifest_ledger_markdown(&manifest_logs),
    )?;
    let path_string = path.to_string_lossy().to_string();
    state
        .database
        .record_team_sharing_manifest_ledger_export(&path_string)
        .await
        .map_err(|err| storage_error("Team sharing manifest ledger audit", err))?;

    Ok(ExportResult { path: path_string })
}

#[tauri::command]
async fn export_filtered_team_sharing_manifest_ledger(
    request: TeamSharingManifestLedgerExportRequest,
    state: State<'_, AppState>,
) -> Result<ExportResult, String> {
    let filter = validated_manifest_ledger_filter(&request.filter)?;
    let filter_label = manifest_ledger_filter_label(filter);
    let audit_logs = state
        .database
        .team_sharing_manifest_ledger_audit_logs()
        .await
        .map_err(|err| storage_error("Team sharing audit logs", err))?;
    let manifest_logs = filter_team_sharing_manifest_ledger(audit_logs, filter, &request.query)?;

    let export_dir = state.database.exports_dir();
    fs::create_dir_all(&export_dir).map_err(|err| err.to_string())?;
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_secs();
    let path = write_export(
        export_dir.join(format!(
            "team-sharing-manifest-ledger-{}-{created_at}.md",
            manifest_ledger_filter_slug(filter)
        )),
        team_sharing_manifest_ledger_markdown_with_context(
            &manifest_logs,
            filter_label,
            !request.query.trim().is_empty(),
        ),
    )?;
    let path_string = path.to_string_lossy().to_string();
    state
        .database
        .record_team_sharing_filtered_manifest_ledger_export(
            &path_string,
            filter_label,
            manifest_logs.len(),
        )
        .await
        .map_err(|err| storage_error("Team sharing manifest ledger audit", err))?;

    Ok(ExportResult { path: path_string })
}

#[tauri::command]
async fn get_team_sharing_manifest_ledger_checksum(
    request: TeamSharingManifestLedgerExportRequest,
    state: State<'_, AppState>,
) -> Result<TeamSharingManifestLedgerChecksum, String> {
    let filter = validated_manifest_ledger_filter(&request.filter)?;
    let audit_logs = state
        .database
        .team_sharing_manifest_ledger_audit_logs()
        .await
        .map_err(|err| storage_error("Team sharing manifest ledger audit logs", err))?;
    let manifest_logs = filter_team_sharing_manifest_ledger(audit_logs, filter, &request.query)?;

    Ok(TeamSharingManifestLedgerChecksum {
        filter: manifest_ledger_filter_label(filter).to_string(),
        event_count: manifest_logs.len() as i64,
        event_set_sha256: team_sharing_manifest_ledger_checksum(&manifest_logs),
        search_applied: !request.query.trim().is_empty(),
    })
}

#[tauri::command]
async fn export_team_sharing_sync_dry_run_manifest(
    state: State<'_, AppState>,
) -> Result<ExportResult, String> {
    let policy = state
        .database
        .team_sharing_policy()
        .await
        .map_err(|err| storage_error("Team sharing policy", err))?;
    let readiness = state
        .database
        .team_sharing_readiness()
        .await
        .map_err(|err| storage_error("Team sharing readiness", err))?;
    let dry_run = state
        .database
        .team_sharing_sync_dry_run()
        .await
        .map_err(|err| storage_error("Team sharing sync dry run", err))?;
    let devices = state
        .database
        .sync_devices()
        .await
        .map_err(|err| storage_error("Team sharing devices", err))?;

    let export_dir = state.database.exports_dir();
    fs::create_dir_all(&export_dir).map_err(|err| err.to_string())?;
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_secs();
    let dry_run_payload = serde_json::to_value(&dry_run).map_err(|err| err.to_string())?;
    let dry_run_sha256 = crate::clipboard::hash_bytes(
        serde_json::to_string(&dry_run_payload)
            .map_err(|err| err.to_string())?
            .as_bytes(),
    );
    let generated_locally_at = created_at.to_string();
    let signer = handoff_signer(&state.database.handoff_signing_key_path())?;
    let verifying_key = signer.signing_key.verifying_key();
    let signature = signer.signing_key.sign(
        team_sharing_manifest_signature_message(&generated_locally_at, &dry_run_sha256).as_bytes(),
    );
    let manifest = serde_json::to_string_pretty(&serde_json::json!({
        "format": "cymos.team_sharing_sync_dry_run",
        "schema_version": 1,
        "generated_locally_at": generated_locally_at,
        "remote_sync_enabled": false,
        "policy": policy,
        "readiness": readiness,
        "dry_run": dry_run_payload,
        "integrity": {
            "algorithm": "SHA-256",
            "dry_run_sha256": dry_run_sha256,
        },
        "authenticity": {
            "algorithm": "Ed25519",
            "public_key_hex": hex::encode(verifying_key.to_bytes()),
            "signature_hex": hex::encode(signature.to_bytes()),
            "signer_fingerprint": &signer.fingerprint,
        },
        "devices": devices,
    }))
    .map_err(|err| err.to_string())?;
    let path = write_export(
        export_dir.join(format!("team-sharing-sync-dry-run-{created_at}.json")),
        manifest,
    )?;
    let path_string = path.to_string_lossy().to_string();
    state
        .database
        .record_team_sharing_manifest_export(&path_string)
        .await
        .map_err(|err| storage_error("Team sharing manifest audit", err))?;

    Ok(ExportResult { path: path_string })
}

#[tauri::command]
async fn inspect_team_sharing_sync_dry_run_manifest(
    request: TeamSharingManifestInspectionRequest,
    state: State<'_, AppState>,
) -> Result<TeamSharingManifestInspection, String> {
    let mut inspection = inspect_team_sharing_manifest(&request.content);
    if inspection.valid {
        if let Some(fingerprint) = inspection.signer_fingerprint.as_deref() {
            if state
                .database
                .trusted_handoff_signer(fingerprint)
                .await
                .map_err(|err| storage_error("Team sharing signer trust", err))?
                .is_some()
            {
                inspection.signer_trusted = true;
                inspection.trust_status = "Trusted signer".to_string();
            } else {
                inspection.signer_trusted = false;
                inspection.trust_status = "Signer not trusted locally".to_string();
            }
        }
    }
    let status = if inspection.valid {
        "Verified"
    } else {
        "Rejected"
    };
    let resource = if inspection.valid {
        let signer = inspection
            .signer_fingerprint
            .as_deref()
            .unwrap_or("unknown signer");
        format!(
            "{} v{} - {} records - signer {} - {}",
            inspection.format,
            inspection.schema_version,
            inspection.estimated_records,
            signer,
            inspection.trust_status
        )
    } else {
        inspection
            .failure_reason
            .as_deref()
            .unwrap_or("Invalid team sharing manifest")
            .to_string()
    };
    state
        .database
        .record_team_sharing_manifest_inspection(status, &resource)
        .await
        .map_err(|err| storage_error("Team sharing manifest inspection audit", err))?;
    Ok(inspection)
}

#[tauri::command]
async fn run_cognitive_release_check(
    state: State<'_, AppState>,
) -> Result<CognitiveReleaseResult, String> {
    state
        .database
        .run_cognitive_release_check()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_cognitive_overview(state: State<'_, AppState>) -> Result<CognitiveOverview, String> {
    state
        .database
        .cognitive_overview()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_cognitive_modules(state: State<'_, AppState>) -> Result<Vec<CognitiveModule>, String> {
    state
        .database
        .cognitive_modules()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_enterprise_controls(
    state: State<'_, AppState>,
) -> Result<Vec<EnterpriseControl>, String> {
    state
        .database
        .enterprise_controls()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_cognitive_use_cases(
    state: State<'_, AppState>,
) -> Result<Vec<CognitiveUseCase>, String> {
    state
        .database
        .cognitive_use_cases()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn copy_clipboard_item(item_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    validation::item_id(item_id)?;
    let item = state
        .database
        .item(item_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Clipboard item not found".to_string())?;

    let mut clipboard = Clipboard::new().map_err(|err| err.to_string())?;

    if item.content_type == "Image" {
        let image = ImageReader::open(&item.content)
            .map_err(|err| err.to_string())?
            .decode()
            .map_err(|err| err.to_string())?
            .to_rgba8();
        let (width, height) = image.dimensions();
        clipboard
            .set_image(ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::Owned(image.into_raw()),
            })
            .map_err(|err| err.to_string())?;
    } else {
        clipboard
            .set_text(item.content)
            .map_err(|err| err.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn export_clipboard_item(
    item_id: i64,
    format: String,
    state: State<'_, AppState>,
) -> Result<ExportResult, String> {
    validation::item_id(item_id)?;
    validation::export_format(&format)?;
    let item = state
        .database
        .item(item_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Clipboard item not found".to_string())?;

    let export_dir = state.database.exports_dir();
    fs::create_dir_all(&export_dir).map_err(|err| err.to_string())?;

    let path = match format.as_str() {
        "JSON" => write_export(
            export_dir.join(format!("clipboard-{}.json", item.id)),
            serde_json::to_string_pretty(&item).map_err(|err| err.to_string())?,
        )?,
        "CSV" => write_export(
            export_dir.join(format!("clipboard-{}.csv", item.id)),
            format!(
                "id,content_type,created_at,content\n{},\"{}\",\"{}\",\"{}\"\n",
                item.id,
                item.content_type,
                item.created_at,
                item.content.replace('"', "\"\"")
            ),
        )?,
        "Markdown" => write_export(
            export_dir.join(format!("clipboard-{}.md", item.id)),
            format!(
                "# Clipboard Item {}\n\n- Type: {}\n- Created: {}\n- Tags: {}\n\n```text\n{}\n```\n",
                item.id,
                item.content_type,
                item.created_at,
                item.tags.join(", "),
                item.content
            ),
        )?,
        _ if item.content_type == "Image" => {
            let destination = export_dir.join(format!("clipboard-{}.png", item.id));
            fs::copy(&item.content, &destination).map_err(|err| err.to_string())?;
            destination
        }
        _ => write_export(
            export_dir.join(format!("clipboard-{}.txt", item.id)),
            item.content,
        )?,
    };

    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
    })
}

fn write_export(path: PathBuf, content: String) -> Result<PathBuf, String> {
    fs::write(&path, content).map_err(|err| err.to_string())?;
    Ok(path)
}

fn database_reliability_markdown(
    reliability: &DatabaseReliabilityStatus,
    snapshots: &[DatabaseBackupSnapshot],
) -> String {
    let mut report = String::new();
    let _ = writeln!(report, "# CYMOS Vault Reliability Report");
    let _ = writeln!(
        report,
        "\nThis local report contains database health and backup metadata only. Clipboard and memory contents are not included."
    );
    let _ = writeln!(
        report,
        "\n## Database Health\n\n- **Integrity:** {}\n- **Foreign-key issues:** {}\n- **Journal mode:** {}\n- **Database size:** {} bytes\n- **Schema migrations:** {}\n- **Retained backups:** {}\n- **Report data SHA-256:** `{}`",
        reliability.integrity_status,
        reliability.foreign_key_issues,
        reliability.journal_mode,
        reliability.database_bytes,
        reliability.migration_count,
        reliability.backup_count,
        database_reliability_checksum(reliability, snapshots),
    );
    let _ = writeln!(report, "\n## Recent Snapshots\n");
    if snapshots.is_empty() {
        let _ = writeln!(report, "No local backup snapshots are available.");
    } else {
        for snapshot in snapshots {
            let _ = writeln!(
                report,
                "- `{}` - {} bytes - modified Unix time {}",
                snapshot.file_name, snapshot.bytes, snapshot.modified_at_unix
            );
        }
    }
    report
}

fn database_reliability_checksum(
    reliability: &DatabaseReliabilityStatus,
    snapshots: &[DatabaseBackupSnapshot],
) -> String {
    let snapshot_metadata = snapshots
        .iter()
        .map(|snapshot| {
            serde_json::json!({
                "file_name": snapshot.file_name,
                "bytes": snapshot.bytes,
                "modified_at_unix": snapshot.modified_at_unix,
            })
        })
        .collect::<Vec<_>>();
    let canonical_metadata = serde_json::to_vec(&serde_json::json!({
        "reliability": reliability,
        "snapshots": snapshot_metadata,
    }))
    .unwrap_or_default();
    crate::clipboard::hash_bytes(&canonical_metadata)
}

fn runbook_entry_markdown(entry: &RunbookEntry) -> String {
    let mut markdown = format!(
        "# {}\n\n- Source: {}\n- Workspace: {}\n- Created: {}\n",
        entry.title, entry.incident_title, entry.workspace_name, entry.created_at
    );
    if entry.incident_id.is_none() {
        let _ = writeln!(markdown, "- Revision: {}", entry.latest_revision);
        let _ = writeln!(markdown, "- Review status: {}", entry.review_status);
        if let Some(reviewed_at) = &entry.last_reviewed_at {
            let _ = writeln!(markdown, "- Last reviewed: {reviewed_at}");
        }
        if let Some(note) = &entry.last_review_note {
            if !note.is_empty() {
                let _ = writeln!(markdown, "- Review evidence: {note}");
            }
        }
    } else {
        markdown.push_str("- Review status: Incident-derived immutable evidence\n");
    }
    if !entry.tags.is_empty() {
        let _ = writeln!(markdown, "- Tags: {}", entry.tags.join(", "));
    }
    markdown.push_str("\n## Procedure\n\n");
    markdown.push_str(&entry.details);
    markdown.push('\n');
    markdown
}

fn team_sharing_readiness_markdown(
    policy: &TeamSharingPolicy,
    readiness: &TeamSharingReadiness,
    devices: &[SyncDevice],
    audit_logs: &[AuditLog],
) -> String {
    let mut report = String::new();
    let _ = writeln!(report, "# CYMOS Team Sharing Readiness Report");
    let _ = writeln!(
        report,
        "\n## Status\n\n- **Readiness:** {}\n- **Mode:** {}\n- **Checked:** {}\n- **Remote sync enabled:** no",
        readiness.status, readiness.mode, readiness.checked_at
    );
    let _ = writeln!(
        report,
        "\n## Policy\n\n- **Enabled:** {}\n- **Workspace handoffs:** {}\n- **Runbook exports:** {}\n- **Imported references:** {}\n- **Device approval required:** {}\n- **Recipient trust required:** {}\n- **Shared-data retention:** {} days",
        policy.enabled,
        policy.allow_workspace_handoffs,
        policy.allow_runbook_exports,
        policy.allow_imported_references,
        policy.require_device_approval,
        policy.require_recipient_trust,
        policy.retention_days
    );
    let _ = writeln!(
        report,
        "\n## Readiness Counts\n\n- **Approved devices:** {}\n- **Trusted recipients:** {}\n- **Trusted signers:** {}",
        readiness.approved_devices, readiness.trusted_recipients, readiness.trusted_signers
    );
    let _ = writeln!(report, "\n## Allowed Scopes\n");
    if readiness.allowed_scopes.is_empty() {
        let _ = writeln!(report, "No sharing scopes are enabled.");
    } else {
        for scope in &readiness.allowed_scopes {
            let _ = writeln!(report, "- {scope}");
        }
    }
    let _ = writeln!(report, "\n## Blockers\n");
    if readiness.blockers.is_empty() {
        let _ = writeln!(report, "No readiness blockers are currently reported.");
    } else {
        for blocker in &readiness.blockers {
            let _ = writeln!(report, "- {blocker}");
        }
    }
    let _ = writeln!(report, "\n## Devices\n");
    if devices.is_empty() {
        let _ = writeln!(report, "No local sharing devices are registered.");
    } else {
        for device in devices {
            let _ = writeln!(
                report,
                "- **{}** - {} - {} - {} - last seen {}",
                device.device_name,
                device.platform,
                device.sync_mode,
                device.status,
                device.last_seen_at
            );
        }
    }
    let _ = writeln!(report, "\n## Recent Sharing Audit\n");
    if audit_logs.is_empty() {
        let _ = writeln!(report, "No team sharing audit events are recorded.");
    } else {
        for log in audit_logs.iter().take(12) {
            let _ = writeln!(
                report,
                "- **{}** - {} - {} - {}",
                log.created_at, log.action, log.resource, log.severity
            );
        }
    }
    report
}

fn team_sharing_manifest_ledger_markdown(audit_logs: &[AuditLog]) -> String {
    team_sharing_manifest_ledger_markdown_with_context(audit_logs, "All", false)
}

fn team_sharing_manifest_ledger_markdown_with_context(
    audit_logs: &[AuditLog],
    filter: &str,
    search_applied: bool,
) -> String {
    let mut report = String::new();
    let _ = writeln!(report, "# CYMOS Team Sharing Manifest Ledger");
    let _ = writeln!(
        report,
        "\nThis report contains metadata-only local manifest export and inspection events. Manifest payloads are not embedded."
    );
    let _ = writeln!(
        report,
        "\n## Summary\n\n- **View filter:** {}\n- **Search applied:** {}\n- **Manifest events:** {}\n- **Rejected or warning events:** {}\n- **Event-set SHA-256:** `{}`",
        filter,
        if search_applied { "Yes (raw query not recorded as report context)" } else { "No" },
        audit_logs.len(),
        audit_logs
            .iter()
            .filter(|log| log.severity == "Warning")
            .count(),
        team_sharing_manifest_ledger_checksum(audit_logs)
    );
    let _ = writeln!(report, "\n## Events\n");
    if audit_logs.is_empty() {
        let _ = writeln!(report, "No local manifest events are recorded.");
    } else {
        for log in audit_logs {
            let _ = writeln!(
                report,
                "- **Event #{}** - {} - {} - {} - {} - {}",
                log.id, log.created_at, log.action, log.severity, log.actor, log.resource
            );
        }
    }
    report
}

fn team_sharing_manifest_ledger_checksum(audit_logs: &[AuditLog]) -> String {
    let canonical_metadata = serde_json::to_vec(audit_logs).unwrap_or_default();
    crate::clipboard::hash_bytes(&canonical_metadata)
}

fn validated_manifest_ledger_filter(filter: &str) -> Result<&str, String> {
    match filter.trim() {
        "All" | "Verified" | "Warnings" | "Exports" | "FilteredExports" => Ok(filter.trim()),
        _ => Err(
            "Manifest ledger filter must be All, Verified, Warnings, Exports, or FilteredExports."
                .to_string(),
        ),
    }
}

fn manifest_ledger_filter_label(filter: &str) -> &str {
    match filter {
        "FilteredExports" => "Filtered exports",
        _ => filter,
    }
}

fn manifest_ledger_filter_slug(filter: &str) -> &str {
    match filter {
        "All" => "all",
        "Verified" => "verified",
        "Warnings" => "warnings",
        "Exports" => "exports",
        "FilteredExports" => "filtered-exports",
        _ => "all",
    }
}

fn filter_team_sharing_manifest_ledger(
    mut audit_logs: Vec<AuditLog>,
    filter: &str,
    query: &str,
) -> Result<Vec<AuditLog>, String> {
    let normalized_query = query.trim();
    if normalized_query.chars().count() > 200 {
        return Err("Manifest ledger search must not exceed 200 characters.".to_string());
    }
    let normalized_query = normalized_query.to_ascii_lowercase();
    audit_logs.retain(|log| {
        if !log.action.contains("manifest") {
            return false;
        }
        let matches_filter = match filter {
            "All" => true,
            "Verified" => log.action == "team_sharing.manifest.inspected" && log.severity == "Info",
            "Warnings" => log.severity == "Warning",
            "Exports" => log.action.contains(".exported"),
            "FilteredExports" => log.action == "team_sharing.manifest_ledger.exported_filtered",
            _ => false,
        };
        matches_filter
            && (normalized_query.is_empty()
                || format!(
                    "{} {} {} {} {}",
                    log.action, log.actor, log.severity, log.created_at, log.resource
                )
                .to_ascii_lowercase()
                .contains(&normalized_query))
    });
    Ok(audit_logs)
}

fn inspect_team_sharing_manifest(content: &str) -> TeamSharingManifestInspection {
    let manifest = match serde_json::from_str::<serde_json::Value>(content) {
        Ok(value) => value,
        Err(err) => {
            return rejected_team_sharing_manifest(format!("Manifest JSON is invalid: {err}"));
        }
    };
    let format = manifest
        .get("format")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    if format != "cymos.team_sharing_sync_dry_run" {
        return rejected_team_sharing_manifest(
            "Manifest format must be cymos.team_sharing_sync_dry_run.".to_string(),
        );
    }
    let schema_version = manifest
        .get("schema_version")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    if schema_version != 1 {
        return rejected_team_sharing_manifest("Manifest schema version must be 1.".to_string());
    }
    let generated_locally_at = manifest
        .get("generated_locally_at")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if generated_locally_at.is_empty() {
        return rejected_team_sharing_manifest(
            "Manifest generation timestamp is missing.".to_string(),
        );
    }
    let remote_sync_enabled = manifest
        .get("remote_sync_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if remote_sync_enabled {
        return rejected_team_sharing_manifest(
            "Manifest must declare remote_sync_enabled as false.".to_string(),
        );
    }
    let dry_run = match manifest.get("dry_run") {
        Some(value) if value.is_object() => value,
        _ => {
            return rejected_team_sharing_manifest(
                "Manifest dry_run object is missing.".to_string(),
            )
        }
    };
    let algorithm = manifest
        .pointer("/integrity/algorithm")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if algorithm != "SHA-256" {
        return rejected_team_sharing_manifest(
            "Manifest integrity algorithm must be SHA-256.".to_string(),
        );
    }
    let declared_dry_run_sha256 = manifest
        .pointer("/integrity/dry_run_sha256")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if declared_dry_run_sha256.len() != 64
        || !declared_dry_run_sha256
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return rejected_team_sharing_manifest(
            "Manifest dry-run checksum is missing or invalid.".to_string(),
        );
    }
    let computed_dry_run_sha256 = match serde_json::to_string(dry_run) {
        Ok(payload) => crate::clipboard::hash_bytes(payload.as_bytes()),
        Err(err) => {
            return rejected_team_sharing_manifest(format!(
                "Manifest dry_run payload cannot be checksummed: {err}"
            ));
        }
    };
    if computed_dry_run_sha256 != declared_dry_run_sha256 {
        return rejected_team_sharing_manifest(
            "Manifest dry-run checksum verification failed.".to_string(),
        );
    }
    let signature = match inspect_team_sharing_manifest_signature(
        &manifest,
        generated_locally_at,
        declared_dry_run_sha256,
    ) {
        Ok(signature) => signature,
        Err(reason) => return rejected_team_sharing_manifest(reason),
    };
    let estimated_records = dry_run
        .get("estimated_records")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(-1);
    let estimated_bytes = dry_run
        .get("estimated_bytes")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(-1);
    if estimated_records < 0 || estimated_bytes < 0 {
        return rejected_team_sharing_manifest(
            "Manifest estimates must be zero or greater.".to_string(),
        );
    }
    let blockers = dry_run
        .get("blockers")
        .and_then(serde_json::Value::as_array)
        .map_or(0, |items| items.len() as i64);
    let device_count = manifest
        .get("devices")
        .and_then(serde_json::Value::as_array)
        .map_or(0, |items| items.len() as i64);

    TeamSharingManifestInspection {
        valid: true,
        status: "Verified".to_string(),
        format,
        schema_version,
        remote_sync_enabled,
        ready: dry_run
            .get("ready")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        mode: dry_run
            .get("mode")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Unknown")
            .to_string(),
        estimated_records,
        estimated_bytes,
        dry_run_sha256: computed_dry_run_sha256,
        signature_verified: signature.verified,
        signer_fingerprint: signature.fingerprint,
        signature_status: signature.status,
        signer_trusted: false,
        trust_status: "Trust not checked locally".to_string(),
        blocker_count: blockers,
        device_count,
        failure_reason: None,
    }
}

fn rejected_team_sharing_manifest(reason: String) -> TeamSharingManifestInspection {
    TeamSharingManifestInspection {
        valid: false,
        status: "Rejected".to_string(),
        format: "Unknown".to_string(),
        schema_version: 0,
        remote_sync_enabled: false,
        ready: false,
        mode: "Unknown".to_string(),
        estimated_records: 0,
        estimated_bytes: 0,
        dry_run_sha256: String::new(),
        signature_verified: false,
        signer_fingerprint: None,
        signature_status: "Not verified".to_string(),
        signer_trusted: false,
        trust_status: "Not checked".to_string(),
        blocker_count: 0,
        device_count: 0,
        failure_reason: Some(reason),
    }
}

fn team_sharing_manifest_signature_message(generated_at: &str, checksum: &str) -> String {
    format!("cymos.team_sharing_sync_dry_run:v1:{generated_at}:{checksum}")
}

fn inspect_team_sharing_manifest_signature(
    manifest: &serde_json::Value,
    generated_at: &str,
    checksum: &str,
) -> Result<HandoffSignatureInspection, String> {
    let authenticity = manifest
        .get("authenticity")
        .ok_or_else(|| "Manifest authenticity signature is missing.".to_string())?;
    if authenticity
        .get("algorithm")
        .and_then(serde_json::Value::as_str)
        != Some("Ed25519")
    {
        return Err("Manifest signature algorithm must be Ed25519.".to_string());
    }
    let public_key_hex = authenticity
        .get("public_key_hex")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "Manifest signer public key is missing.".to_string())?;
    let signature_hex = authenticity
        .get("signature_hex")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "Manifest signature is missing.".to_string())?;
    let declared_fingerprint = authenticity
        .get("signer_fingerprint")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "Manifest signer fingerprint is missing.".to_string())?;
    let public_key_bytes = hex::decode(public_key_hex)
        .map_err(|_| "Manifest signer public key is invalid.".to_string())?;
    let signature_bytes =
        hex::decode(signature_hex).map_err(|_| "Manifest signature is invalid.".to_string())?;
    let public_key_bytes: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| "Manifest signer public key is invalid.".to_string())?;
    let signature_bytes: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| "Manifest signature is invalid.".to_string())?;
    let fingerprint = signer_fingerprint(&public_key_bytes);
    if fingerprint != declared_fingerprint {
        return Err("Manifest signer fingerprint verification failed.".to_string());
    }
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|_| "Manifest signer public key is invalid.".to_string())?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify(
            team_sharing_manifest_signature_message(generated_at, checksum).as_bytes(),
            &signature,
        )
        .map_err(|_| {
            "Manifest signature verification failed. Do not trust this manifest.".to_string()
        })?;
    Ok(HandoffSignatureInspection {
        verified: true,
        fingerprint: Some(fingerprint),
        status: "Signature verified".to_string(),
    })
}

fn workspace_report_markdown(snapshot: &WorkspaceSnapshot, session_id: Option<i64>) -> String {
    let selected_session =
        session_id.and_then(|id| snapshot.sessions.iter().find(|session| session.id == id));
    let events = snapshot
        .events
        .iter()
        .filter(|event| session_id.is_none_or(|id| event.session_id == Some(id)))
        .collect::<Vec<_>>();
    let incident_ids = events
        .iter()
        .filter_map(|event| event.incident_id)
        .collect::<Vec<_>>();
    let incidents = snapshot
        .incidents
        .iter()
        .filter(|incident| incident_ids.contains(&incident.id))
        .collect::<Vec<_>>();
    let resolutions = snapshot
        .resolutions
        .iter()
        .filter(|resolution| incident_ids.contains(&resolution.incident_id))
        .collect::<Vec<_>>();

    let mut report = String::new();
    let _ = writeln!(report, "# CYMOS Operational Report");
    let _ = writeln!(
        report,
        "\n## Workspace\n\n- **Name:** {}\n- **Project:** {}\n- **Generated locally:** yes",
        snapshot.workspace.name, snapshot.workspace.project
    );
    if let Some(session) = selected_session {
        let _ = writeln!(
            report,
            "\n## Session\n\n- **Title:** {}\n- **Status:** {}\n- **Started:** {}\n- **Ended:** {}",
            session.title,
            session.status,
            session.started_at,
            session.ended_at.as_deref().unwrap_or("In progress")
        );
    } else {
        let _ = writeln!(report, "\n## Scope\n\nAll workspace sessions");
    }

    let _ = writeln!(report, "\n## Timeline\n");
    if events.is_empty() {
        let _ = writeln!(report, "No captured events matched this report scope.");
    }
    for event in events {
        let _ = writeln!(
            report,
            "### {}: {}\n\n- **When:** {}\n- **Source:** {}\n- **Severity:** {}\n\n{}\n",
            event.event_type,
            event.title,
            event.created_at,
            event.source_application,
            event.severity,
            event.details
        );
    }

    let _ = writeln!(report, "## Incident Memory\n");
    if incidents.is_empty() {
        let _ = writeln!(report, "No incident signals matched this report scope.");
    }
    for incident in incidents {
        let _ = writeln!(
            report,
            "### {}\n\n- **Status:** {}\n- **Signals:** {}\n\n{}\n",
            incident.title, incident.status, incident.event_count, incident.summary
        );
    }

    let _ = writeln!(report, "## Recorded Fixes\n");
    if resolutions.is_empty() {
        let _ = writeln!(report, "No remediation records matched this report scope.");
    }
    for resolution in resolutions {
        let _ = writeln!(
            report,
            "### {}\n\n- **Recorded:** {}\n- **Source workspace:** {}\n\n{}\n",
            resolution.title, resolution.created_at, resolution.workspace_name, resolution.details
        );
    }
    report
}

fn validate_workspace_handoff_scope(
    snapshot: &WorkspaceSnapshot,
    request: &WorkspaceHandoffRequest,
) -> Result<(), String> {
    if let Some(session_id) = request.session_id {
        if !snapshot
            .sessions
            .iter()
            .any(|session| session.id == session_id)
        {
            return Err("The selected session does not belong to this workspace.".to_string());
        }
    }

    if request.excluded_event_ids.iter().any(|event_id| {
        !snapshot.events.iter().any(|event| {
            event.id == *event_id
                && request
                    .session_id
                    .is_none_or(|id| event.session_id == Some(id))
        })
    }) {
        return Err("A selected handoff exclusion is not in this workspace scope.".to_string());
    }
    Ok(())
}

fn handoff_classification_rank(classification: &str) -> i64 {
    match classification {
        "Internal" => 1,
        "Restricted" => 2,
        "Confidential" => 3,
        _ => 0,
    }
}

async fn validate_trusted_handoff_recipient(
    database: &Database,
    request: &WorkspaceHandoffRequest,
) -> Result<HandoffRecipientTrustRecord, String> {
    let Some(record) = database
        .trusted_handoff_recipient(request.recipient.trim())
        .await
        .map_err(|err| storage_error("Handoff recipient trust", err))?
    else {
        return Err(format!(
            "Recipient '{}' is not in the local trusted handoff registry.",
            request.recipient.trim()
        ));
    };
    if handoff_classification_rank(&request.classification)
        > handoff_classification_rank(&record.max_classification)
    {
        return Err(format!(
            "Recipient '{}' is trusted only up to {} handoffs.",
            record.recipient, record.max_classification
        ));
    }
    Ok(record)
}

fn workspace_handoff_json(
    snapshot: &WorkspaceSnapshot,
    request: &WorkspaceHandoffRequest,
    generated_at: &str,
    signer: &HandoffSigner,
) -> Result<String, String> {
    let payload = workspace_handoff_payload(snapshot, request);
    let canonical_payload = serde_json::to_string(&payload).map_err(|err| err.to_string())?;
    let checksum = crate::clipboard::hash_bytes(canonical_payload.as_bytes());
    let verifying_key = signer.signing_key.verifying_key();
    let signature = signer
        .signing_key
        .sign(handoff_signature_message(generated_at, &checksum).as_bytes());
    serde_json::to_string_pretty(&serde_json::json!({
        "format": "cymos.workspace_handoff",
        "schema_version": 1,
        "generated_locally_at": generated_at,
        "payload": &payload,
        "integrity": {
            "algorithm": "SHA-256",
            "payload_sha256": checksum,
        },
        "authenticity": {
            "algorithm": "Ed25519",
            "public_key_hex": hex::encode(verifying_key.to_bytes()),
            "signature_hex": hex::encode(signature.to_bytes()),
            "signer_fingerprint": &signer.fingerprint,
        },
    }))
    .map_err(|err| err.to_string())
}

fn handoff_signer(path: &Path) -> Result<HandoffSigner, String> {
    let signing_key = match fs::read(path) {
        Ok(bytes) => {
            let key_bytes: [u8; 32] = bytes.try_into().map_err(|_| {
                "Local handoff signing key is invalid. Remove it only if you intend to rotate this device identity."
                    .to_string()
            })?;
            SigningKey::from_bytes(&key_bytes)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("Could not create local signer directory: {err}"))?;
            }
            let signing_key = SigningKey::generate(&mut OsRng);
            let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|err| format!("Could not create local handoff signing key: {err}"))?;
            file.write_all(&signing_key.to_bytes())
                .map_err(|err| format!("Could not write local handoff signing key: {err}"))?;
            file.sync_all()
                .map_err(|err| format!("Could not persist local handoff signing key: {err}"))?;
            #[cfg(unix)]
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
                .map_err(|err| format!("Could not protect local handoff signing key: {err}"))?;
            fs::rename(&temporary, path)
                .map_err(|err| format!("Could not activate local handoff signing key: {err}"))?;
            signing_key
        }
        Err(err) => return Err(format!("Could not read local handoff signing key: {err}")),
    };
    let public_key = signing_key.verifying_key().to_bytes();
    Ok(HandoffSigner {
        signing_key,
        fingerprint: signer_fingerprint(&public_key),
    })
}

fn signer_fingerprint(public_key: &[u8]) -> String {
    crate::clipboard::hash_bytes(public_key)
        .chars()
        .take(16)
        .collect()
}

fn handoff_signature_message(generated_at: &str, checksum: &str) -> String {
    format!("cymos.workspace_handoff:v1:{generated_at}:{checksum}")
}

fn workspace_handoff_payload(
    snapshot: &WorkspaceSnapshot,
    request: &WorkspaceHandoffRequest,
) -> serde_json::Value {
    let selected_session = request
        .session_id
        .and_then(|id| snapshot.sessions.iter().find(|session| session.id == id));
    let scoped_events = snapshot
        .events
        .iter()
        .filter(|event| {
            request
                .session_id
                .is_none_or(|id| event.session_id == Some(id))
        })
        .collect::<Vec<_>>();
    let excluded_event_count = scoped_events
        .iter()
        .filter(|event| request.excluded_event_ids.contains(&event.id))
        .count();
    let events = scoped_events
        .into_iter()
        .filter(|event| !request.excluded_event_ids.contains(&event.id))
        .collect::<Vec<_>>();
    let incident_ids = events
        .iter()
        .filter_map(|event| event.incident_id)
        .collect::<Vec<_>>();
    let incidents = snapshot
        .incidents
        .iter()
        .filter(|incident| incident_ids.contains(&incident.id))
        .collect::<Vec<_>>();
    let resolutions = snapshot
        .resolutions
        .iter()
        .filter(|resolution| incident_ids.contains(&resolution.incident_id))
        .collect::<Vec<_>>();

    serde_json::json!({
        "scope": {
            "workspace_id": snapshot.workspace.id,
            "session_id": request.session_id,
            "session_title": selected_session.map(|session| &session.title),
            "excluded_event_count": excluded_event_count,
        },
        "handoff_intent": {
            "recipient": request.recipient.trim(),
            "purpose": request.purpose.trim(),
            "classification": &request.classification,
            "expires_in_days": request.expires_in_days,
        },
        "workspace": &snapshot.workspace,
        "events": events,
        "incidents": incidents,
        "resolutions": resolutions,
    })
}

fn workspace_handoff_readiness(
    snapshot: &WorkspaceSnapshot,
    request: &WorkspaceHandoffRequest,
) -> Result<WorkspaceHandoffReadiness, String> {
    let payload = workspace_handoff_payload(snapshot, request);
    let payload_text = serde_json::to_string(&payload).map_err(|err| err.to_string())?;
    let mut blockers = Vec::new();
    if let Some(reason) = privacy::sensitive_reason(&payload_text) {
        blockers.push(reason);
    }
    let estimated_bytes = serde_json::to_vec(&serde_json::json!({
        "format": "cymos.workspace_handoff",
        "schema_version": 1,
        "generated_locally_at": "readiness-check",
        "payload": &payload,
        "integrity": {
            "algorithm": "SHA-256",
            "payload_sha256": "readiness-check",
        },
    }))
    .map_err(|err| err.to_string())?
    .len();
    if estimated_bytes > validation::MAX_CLIPBOARD_TEXT_BYTES {
        blockers.push("Handoff payload exceeds the 1 MB local import limit.".to_string());
    }
    let scope = if let Some(session) = request
        .session_id
        .and_then(|id| snapshot.sessions.iter().find(|session| session.id == id))
    {
        format!("Session: {}", session.title)
    } else {
        "All workspace sessions".to_string()
    };
    let count = |field: &str| {
        payload
            .get(field)
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .unwrap_or_default()
    };
    let excluded_event_count = payload
        .pointer("/scope/excluded_event_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default() as usize;
    let scope = if excluded_event_count == 0 {
        scope
    } else {
        format!("{scope} - {excluded_event_count} event(s) excluded")
    };
    Ok(WorkspaceHandoffReadiness {
        safe: blockers.is_empty(),
        scope,
        event_count: count("events"),
        excluded_event_count,
        incident_count: count("incidents"),
        resolution_count: count("resolutions"),
        estimated_bytes,
        blocking_findings: blockers.len(),
        blockers,
    })
}

fn handoff_expiry_days(payload: &serde_json::Value) -> Result<Option<i64>, String> {
    let Some(value) = payload.pointer("/handoff_intent/expires_in_days") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let expires_in_days = value
        .as_i64()
        .ok_or_else(|| "This handoff package has an invalid expiry policy.".to_string())?;
    if !matches!(expires_in_days, 1 | 7 | 30) {
        return Err("This handoff package has an unsupported expiry policy.".to_string());
    }
    Ok(Some(expires_in_days))
}

fn handoff_expiry(
    generated_locally_at: &str,
    expires_in_days: Option<i64>,
) -> Result<(Option<u64>, bool), String> {
    let Some(expires_in_days) = expires_in_days else {
        return Ok((None, false));
    };
    let generated_at = generated_locally_at
        .parse::<u64>()
        .map_err(|_| "This handoff package has an invalid generation time.".to_string())?;
    let expiry_seconds = (expires_in_days as u64)
        .checked_mul(86_400)
        .ok_or_else(|| "This handoff package has an invalid expiry policy.".to_string())?;
    let expires_at_unix = generated_at
        .checked_add(expiry_seconds)
        .ok_or_else(|| "This handoff package has an invalid expiry policy.".to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_secs();
    Ok((Some(expires_at_unix), now >= expires_at_unix))
}

struct HandoffSignatureInspection {
    verified: bool,
    fingerprint: Option<String>,
    status: String,
}

fn inspect_handoff_signature(
    package: &serde_json::Value,
    generated_at: &str,
    checksum: &str,
) -> Result<HandoffSignatureInspection, String> {
    let Some(authenticity) = package.get("authenticity") else {
        return Ok(HandoffSignatureInspection {
            verified: false,
            fingerprint: None,
            status: "Unsigned legacy package".to_string(),
        });
    };
    if authenticity
        .get("algorithm")
        .and_then(serde_json::Value::as_str)
        != Some("Ed25519")
    {
        return Err("This handoff package uses an unsupported signature algorithm.".to_string());
    }
    let public_key_hex = authenticity
        .get("public_key_hex")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "This signed handoff package is missing its public key.".to_string())?;
    let signature_hex = authenticity
        .get("signature_hex")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "This signed handoff package is missing its signature.".to_string())?;
    let declared_fingerprint = authenticity
        .get("signer_fingerprint")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            "This signed handoff package is missing its signer fingerprint.".to_string()
        })?;
    let public_key_bytes = hex::decode(public_key_hex)
        .map_err(|_| "This handoff package has an invalid signer public key.".to_string())?;
    let signature_bytes = hex::decode(signature_hex)
        .map_err(|_| "This handoff package has an invalid signature.".to_string())?;
    let public_key_bytes: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| "This handoff package has an invalid signer public key.".to_string())?;
    let signature_bytes: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| "This handoff package has an invalid signature.".to_string())?;
    let fingerprint = signer_fingerprint(&public_key_bytes);
    if fingerprint != declared_fingerprint {
        return Err("Handoff signer fingerprint verification failed.".to_string());
    }
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|_| "This handoff package has an invalid signer public key.".to_string())?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify(
            handoff_signature_message(generated_at, checksum).as_bytes(),
            &signature,
        )
        .map_err(|_| {
            "Handoff signature verification failed. Do not import this package.".to_string()
        })?;
    Ok(HandoffSignatureInspection {
        verified: true,
        fingerprint: Some(fingerprint),
        status: "Signature verified".to_string(),
    })
}

fn inspect_handoff_package(content: &str) -> Result<HandoffInspection, String> {
    let package: serde_json::Value = serde_json::from_str(content)
        .map_err(|_| "The selected file is not a valid CYMOS handoff package.".to_string())?;
    if package.get("format").and_then(serde_json::Value::as_str) != Some("cymos.workspace_handoff")
        || package
            .get("schema_version")
            .and_then(serde_json::Value::as_i64)
            != Some(1)
    {
        return Err("This handoff package uses an unsupported CYMOS format.".to_string());
    }
    let payload = package
        .get("payload")
        .ok_or_else(|| "This handoff package is missing its payload.".to_string())?;
    let algorithm = package
        .pointer("/integrity/algorithm")
        .and_then(serde_json::Value::as_str);
    let checksum = package
        .pointer("/integrity/payload_sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "This handoff package is missing its integrity checksum.".to_string())?;
    if algorithm != Some("SHA-256")
        || crate::clipboard::hash_bytes(
            serde_json::to_string(payload)
                .map_err(|err| err.to_string())?
                .as_bytes(),
        ) != checksum
    {
        return Err(
            "Handoff integrity verification failed. Do not import this package.".to_string(),
        );
    }
    let workspace = payload
        .get("workspace")
        .ok_or_else(|| "This handoff package is missing workspace information.".to_string())?;
    let workspace_name = workspace
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "This handoff package has an invalid workspace name.".to_string())?;
    let project = workspace
        .get("project")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "This handoff package has an invalid project name.".to_string())?;
    let scope = payload
        .pointer("/scope/session_title")
        .and_then(serde_json::Value::as_str)
        .map(|title| format!("Session: {title}"))
        .unwrap_or_else(|| "All workspace sessions".to_string());
    let recipient = payload
        .pointer("/handoff_intent/recipient")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Unspecified")
        .to_string();
    let purpose = payload
        .pointer("/handoff_intent/purpose")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Unspecified")
        .to_string();
    let classification = payload
        .pointer("/handoff_intent/classification")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Unspecified")
        .to_string();
    let generated_locally_at = package
        .get("generated_locally_at")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "This handoff package is missing its generation time.".to_string())?;
    let signature = inspect_handoff_signature(&package, generated_locally_at, checksum)?;
    if classification == "Confidential" && !signature.verified {
        return Err("Confidential handoff packages must be signed.".to_string());
    }
    let (expires_at_unix, is_expired) =
        handoff_expiry(generated_locally_at, handoff_expiry_days(payload)?)?;
    let count = |field: &str| {
        payload
            .get(field)
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .ok_or_else(|| format!("This handoff package has invalid {field} data."))
    };
    Ok(HandoffInspection {
        workspace_name: workspace_name.to_string(),
        project: project.to_string(),
        scope,
        recipient,
        purpose,
        classification,
        expires_at_unix,
        is_expired,
        generated_locally_at: generated_locally_at.to_string(),
        event_count: count("events")?,
        incident_count: count("incidents")?,
        resolution_count: count("resolutions")?,
        checksum: checksum.to_string(),
        signature_verified: signature.verified,
        signer_fingerprint: signature.fingerprint,
        signature_status: signature.status,
    })
}

#[cfg(test)]
mod report_tests {
    use super::{
        database_reliability_checksum, database_reliability_markdown,
        filter_team_sharing_manifest_ledger, inspect_handoff_package,
        inspect_team_sharing_manifest, runbook_entry_markdown, signer_fingerprint,
        team_sharing_manifest_ledger_checksum, team_sharing_manifest_ledger_markdown,
        team_sharing_manifest_ledger_markdown_with_context,
        team_sharing_manifest_signature_message, workspace_handoff_json,
        workspace_handoff_readiness, workspace_report_markdown, AuditLog, DatabaseBackupSnapshot,
        DatabaseReliabilityStatus, HandoffSigner,
    };
    use crate::insight_trail::{InsightIncident, InsightTrailEvent};
    use crate::platform::TeamSharingManifestLedgerExportRequest;
    use crate::workspace::{
        CognitiveWorkspace, IncidentResolution, RunbookEntry, WorkspaceHandoffRequest,
        WorkspaceSession, WorkspaceSnapshot,
    };
    use ed25519_dalek::{Signer as _, SigningKey};

    fn handoff_request(
        session_id: Option<i64>,
        excluded_event_ids: Vec<i64>,
    ) -> WorkspaceHandoffRequest {
        WorkspaceHandoffRequest {
            workspace_id: 1,
            session_id,
            excluded_event_ids,
            recipient: "Platform operations".to_string(),
            purpose: "Incident escalation".to_string(),
            classification: "Restricted".to_string(),
            expires_in_days: None,
        }
    }

    fn test_handoff_signer() -> HandoffSigner {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let fingerprint = signer_fingerprint(&signing_key.verifying_key().to_bytes());
        HandoffSigner {
            signing_key,
            fingerprint,
        }
    }

    #[test]
    fn session_report_keeps_only_the_selected_timeline() {
        let snapshot = WorkspaceSnapshot {
            workspace: CognitiveWorkspace {
                id: 1,
                name: "Nginx rollout".to_string(),
                project: "RHEL operations".to_string(),
                status: "Active".to_string(),
                is_imported: false,
                created_at: String::new(),
                updated_at: String::new(),
                last_event_at: None,
                event_count: 2,
                memory_count: 2,
                error_count: 1,
                sources: Vec::new(),
                top_topics: Vec::new(),
                summary: String::new(),
                next_signal: String::new(),
            },
            sessions: vec![
                WorkspaceSession {
                    id: 10,
                    workspace_id: 1,
                    title: "Investigate outage".to_string(),
                    status: "Completed".to_string(),
                    started_at: "2026-07-17 10:00".to_string(),
                    ended_at: Some("2026-07-17 10:30".to_string()),
                    event_count: 1,
                },
                WorkspaceSession {
                    id: 11,
                    workspace_id: 1,
                    title: "Follow-up".to_string(),
                    status: "Active".to_string(),
                    started_at: "2026-07-17 11:00".to_string(),
                    ended_at: None,
                    event_count: 1,
                },
            ],
            active_session: None,
            events: vec![
                event(1, 10, "SELinux denied access", Some(1)),
                event(2, 11, "Unrelated follow-up", None),
            ],
            incidents: vec![InsightIncident {
                id: 1,
                title: "Nginx SELinux incident".to_string(),
                status: "Resolved".to_string(),
                summary: "Nginx could not read the configured path.".to_string(),
                first_seen_at: "2026-07-17 10:00".to_string(),
                last_seen_at: "2026-07-17 10:30".to_string(),
                event_count: 1,
                recommended_steps: Vec::new(),
            }],
            resolutions: vec![IncidentResolution {
                id: 1,
                incident_id: 1,
                workspace_id: 1,
                workspace_name: "Nginx rollout".to_string(),
                session_id: Some(10),
                title: "restorecon -Rv /var/www".to_string(),
                details: "Restored the correct SELinux context.".to_string(),
                created_at: "2026-07-17 10:30".to_string(),
            }],
            import_provenance: None,
        };

        let report = workspace_report_markdown(&snapshot, Some(10));
        assert!(report.contains("SELinux denied access"));
        assert!(report.contains("restorecon -Rv /var/www"));
        assert!(!report.contains("Unrelated follow-up"));

        let request = handoff_request(Some(10), Vec::new());
        let signer = test_handoff_signer();
        let handoff = workspace_handoff_json(&snapshot, &request, "1780000000", &signer)
            .expect("handoff should serialize");
        assert!(handoff.contains("cymos.workspace_handoff"));
        assert!(handoff.contains("SELinux denied access"));
        assert!(handoff.contains("restorecon -Rv /var/www"));
        assert!(!handoff.contains("Unrelated follow-up"));
        let inspection = inspect_handoff_package(&handoff).expect("handoff should verify");
        assert_eq!(inspection.workspace_name, "Nginx rollout");
        assert_eq!(inspection.recipient, "Platform operations");
        assert_eq!(inspection.classification, "Restricted");
        assert_eq!(inspection.event_count, 1);
        assert_eq!(inspection.incident_count, 1);
        assert!(inspection.signature_verified);
        assert_eq!(inspection.signer_fingerprint, Some(signer.fingerprint));
        assert_eq!(inspection.signature_status, "Signature verified");
        let readiness = workspace_handoff_readiness(&snapshot, &request)
            .expect("handoff readiness should load");
        assert!(readiness.safe);
        assert_eq!(readiness.event_count, 1);
    }

    #[test]
    fn handoff_readiness_blocks_sensitive_workspace_data() {
        let snapshot = WorkspaceSnapshot {
            workspace: CognitiveWorkspace {
                id: 1,
                name: "Private review".to_string(),
                project: "Secure migration".to_string(),
                status: "Active".to_string(),
                is_imported: false,
                created_at: String::new(),
                updated_at: String::new(),
                last_event_at: None,
                event_count: 1,
                memory_count: 1,
                error_count: 0,
                sources: Vec::new(),
                top_topics: Vec::new(),
                summary: String::new(),
                next_signal: String::new(),
            },
            sessions: vec![WorkspaceSession {
                id: 10,
                workspace_id: 1,
                title: "Sensitive review".to_string(),
                status: "Active".to_string(),
                started_at: String::new(),
                ended_at: None,
                event_count: 1,
            }],
            active_session: None,
            events: vec![InsightTrailEvent {
                details: "api_key=very-long-test-secret-value".to_string(),
                ..event(1, 10, "Legacy config", None)
            }],
            incidents: Vec::new(),
            resolutions: Vec::new(),
            import_provenance: None,
        };

        let request = handoff_request(Some(10), Vec::new());
        let readiness = workspace_handoff_readiness(&snapshot, &request)
            .expect("handoff readiness should load");
        assert!(!readiness.safe);
        assert_eq!(readiness.blocking_findings, 1);
        assert!(readiness
            .blockers
            .iter()
            .any(|reason| reason.contains("Sensitive configuration")));

        let selective_request = handoff_request(Some(10), vec![1]);
        let selective_readiness = workspace_handoff_readiness(&snapshot, &selective_request)
            .expect("selective handoff readiness should load");
        assert!(selective_readiness.safe);
        assert_eq!(selective_readiness.event_count, 0);
        assert_eq!(selective_readiness.excluded_event_count, 1);
        let signer = test_handoff_signer();
        let handoff = workspace_handoff_json(&snapshot, &selective_request, "1780000000", &signer)
            .expect("selective handoff should serialize");
        assert!(!handoff.contains("api_key="));
    }

    #[test]
    fn signed_handoff_rejects_rechecksummed_payload_tampering() {
        let snapshot = WorkspaceSnapshot {
            workspace: CognitiveWorkspace {
                id: 1,
                name: "Trusted handoff".to_string(),
                project: "Operations".to_string(),
                status: "Active".to_string(),
                is_imported: false,
                created_at: String::new(),
                updated_at: String::new(),
                last_event_at: None,
                event_count: 0,
                memory_count: 0,
                error_count: 0,
                sources: Vec::new(),
                top_topics: Vec::new(),
                summary: String::new(),
                next_signal: String::new(),
            },
            sessions: Vec::new(),
            active_session: None,
            events: Vec::new(),
            incidents: Vec::new(),
            resolutions: Vec::new(),
            import_provenance: None,
        };
        let request = handoff_request(None, Vec::new());
        let handoff =
            workspace_handoff_json(&snapshot, &request, "1780000000", &test_handoff_signer())
                .expect("handoff should serialize");
        let mut package: serde_json::Value =
            serde_json::from_str(&handoff).expect("handoff should be json");
        package["payload"]["workspace"]["name"] = serde_json::json!("Tampered handoff");
        let checksum = crate::clipboard::hash_bytes(
            serde_json::to_string(&package["payload"])
                .expect("payload should serialize")
                .as_bytes(),
        );
        package["integrity"]["payload_sha256"] = serde_json::json!(checksum);

        let error = inspect_handoff_package(&package.to_string())
            .expect_err("tampered signed package should fail verification");
        assert!(error.contains("signature verification failed"));
    }

    #[test]
    fn confidential_handoff_rejects_unsigned_legacy_package() {
        let payload = serde_json::json!({
            "scope": { "session_title": "Confidential incident" },
            "handoff_intent": {
                "recipient": "Security operations",
                "purpose": "Confidential incident escalation",
                "classification": "Confidential",
                "expires_in_days": 7,
            },
            "workspace": { "name": "Confidential handoff", "project": "Operations" },
            "events": [],
            "incidents": [],
            "resolutions": [],
        });
        let checksum = crate::clipboard::hash_bytes(
            serde_json::to_string(&payload)
                .expect("payload should serialize")
                .as_bytes(),
        );
        let package = serde_json::json!({
            "format": "cymos.workspace_handoff",
            "schema_version": 1,
            "generated_locally_at": "1780000000",
            "payload": payload,
            "integrity": { "algorithm": "SHA-256", "payload_sha256": checksum },
        });

        let error = inspect_handoff_package(&package.to_string())
            .expect_err("unsigned confidential package should be rejected");
        assert!(error.contains("Confidential handoff packages must be signed"));
    }

    #[test]
    fn handoff_inspection_marks_expired_packages_as_non_importable() {
        let payload = serde_json::json!({
            "scope": { "session_title": "Expired incident" },
            "handoff_intent": {
                "recipient": "Platform operations",
                "purpose": "Incident escalation",
                "classification": "Restricted",
                "expires_in_days": 1,
            },
            "workspace": { "name": "Expired handoff", "project": "Operations" },
            "events": [],
            "incidents": [],
            "resolutions": [],
        });
        let checksum = crate::clipboard::hash_bytes(
            serde_json::to_string(&payload)
                .expect("payload should serialize")
                .as_bytes(),
        );
        let package = serde_json::json!({
            "format": "cymos.workspace_handoff",
            "schema_version": 1,
            "generated_locally_at": "1",
            "payload": payload,
            "integrity": { "algorithm": "SHA-256", "payload_sha256": checksum },
        });

        let inspection = inspect_handoff_package(&package.to_string())
            .expect("expired package should remain inspectable");
        assert!(inspection.is_expired);
        assert_eq!(inspection.expires_at_unix, Some(86_401));
        assert!(!inspection.signature_verified);
        assert_eq!(inspection.signature_status, "Unsigned legacy package");
    }

    #[test]
    fn team_sharing_manifest_inspection_rejects_remote_sync_enabled_payloads() {
        let dry_run = serde_json::json!({
            "ready": true,
            "status": "Ready",
            "mode": "LocalOnly",
            "eligible_devices": 1,
            "eligible_scopes": ["Workspace handoffs"],
            "estimated_records": 2,
            "estimated_bytes": 2048,
            "blockers": [],
            "generated_at": "2026-07-19 09:00"
        });
        let dry_run_sha256 = crate::clipboard::hash_bytes(
            serde_json::to_string(&dry_run)
                .expect("dry run should serialize")
                .as_bytes(),
        );
        let signer = test_handoff_signer();
        let signature = signer
            .signing_key
            .sign(team_sharing_manifest_signature_message("123", &dry_run_sha256).as_bytes());
        let manifest = serde_json::json!({
            "format": "cymos.team_sharing_sync_dry_run",
            "schema_version": 1,
            "generated_locally_at": "123",
            "remote_sync_enabled": false,
            "dry_run": dry_run,
            "integrity": {
                "algorithm": "SHA-256",
                "dry_run_sha256": dry_run_sha256,
            },
            "authenticity": {
                "algorithm": "Ed25519",
                "public_key_hex": hex::encode(signer.signing_key.verifying_key().to_bytes()),
                "signature_hex": hex::encode(signature.to_bytes()),
                "signer_fingerprint": &signer.fingerprint,
            },
            "devices": []
        });
        let inspection = inspect_team_sharing_manifest(&manifest.to_string());
        assert!(inspection.valid);
        assert_eq!(inspection.estimated_records, 2);
        assert_eq!(inspection.estimated_bytes, 2048);
        assert_eq!(inspection.dry_run_sha256, dry_run_sha256);
        assert!(inspection.signature_verified);
        assert_eq!(
            inspection.signer_fingerprint,
            Some(signer.fingerprint.clone())
        );

        let mut tampered_manifest = manifest.clone();
        tampered_manifest["dry_run"]["estimated_records"] = serde_json::Value::from(3);
        let tampered = inspect_team_sharing_manifest(&tampered_manifest.to_string());
        assert!(!tampered.valid);
        assert_eq!(tampered.status, "Rejected");

        let mut unsigned_manifest = manifest.clone();
        unsigned_manifest
            .as_object_mut()
            .expect("manifest should be an object")
            .remove("authenticity");
        let unsigned = inspect_team_sharing_manifest(&unsigned_manifest.to_string());
        assert!(!unsigned.valid);
        assert_eq!(unsigned.status, "Rejected");

        let mut unsafe_manifest = manifest.clone();
        unsafe_manifest["remote_sync_enabled"] = serde_json::Value::Bool(true);
        let rejected = inspect_team_sharing_manifest(&unsafe_manifest.to_string());
        assert!(!rejected.valid);
        assert_eq!(rejected.status, "Rejected");
    }

    #[test]
    fn manifest_ledger_report_is_metadata_only() {
        let markdown = team_sharing_manifest_ledger_markdown(&[AuditLog {
            id: 1,
            actor: "system".to_string(),
            action: "team_sharing.manifest.inspected".to_string(),
            resource: "cymos.team_sharing_sync_dry_run v1 - 2 records - signer abc".to_string(),
            severity: "Info".to_string(),
            created_at: "2026-07-19 10:30".to_string(),
        }]);

        assert!(markdown.contains("# CYMOS Team Sharing Manifest Ledger"));
        assert!(markdown.contains("team_sharing.manifest.inspected"));
        assert!(markdown.contains("Event #1"));
        assert!(markdown.contains("Event-set SHA-256"));
        assert!(markdown.contains("2 records"));
        assert!(markdown.contains("Manifest payloads are not embedded"));
        assert!(!markdown.contains("\"dry_run\""));
    }

    #[test]
    fn filtered_manifest_ledger_export_keeps_the_query_private() {
        let logs = vec![
            AuditLog {
                id: 1,
                actor: "system".to_string(),
                action: "team_sharing.manifest.inspected".to_string(),
                resource: "Verified signer abc".to_string(),
                severity: "Info".to_string(),
                created_at: "2026-07-19 10:30".to_string(),
            },
            AuditLog {
                id: 2,
                actor: "system".to_string(),
                action: "team_sharing.manifest.inspected".to_string(),
                resource: "Rejected signer def".to_string(),
                severity: "Warning".to_string(),
                created_at: "2026-07-19 10:31".to_string(),
            },
        ];
        let request = TeamSharingManifestLedgerExportRequest {
            filter: "Warnings".to_string(),
            query: "signer def".to_string(),
        };
        let filtered = filter_team_sharing_manifest_ledger(logs, &request.filter, &request.query)
            .expect("filter should be valid");
        let markdown =
            team_sharing_manifest_ledger_markdown_with_context(&filtered, &request.filter, true);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].severity, "Warning");
        assert!(markdown.contains("**View filter:** Warnings"));
        assert!(markdown.contains("raw query not recorded as report context"));
        assert!(!markdown.contains("**Search query:**"));
    }

    #[test]
    fn filtered_export_view_excludes_full_ledger_exports() {
        let logs = vec![
            AuditLog {
                id: 1,
                actor: "system".to_string(),
                action: "team_sharing.manifest_ledger.exported".to_string(),
                resource: "/tmp/full.md".to_string(),
                severity: "Info".to_string(),
                created_at: "2026-07-19 10:30".to_string(),
            },
            AuditLog {
                id: 2,
                actor: "system".to_string(),
                action: "team_sharing.manifest_ledger.exported_filtered".to_string(),
                resource: "/tmp/warnings.md - filter=Warnings - matching_events=2".to_string(),
                severity: "Info".to_string(),
                created_at: "2026-07-19 10:31".to_string(),
            },
        ];
        let filtered = filter_team_sharing_manifest_ledger(logs, "FilteredExports", "")
            .expect("filtered export view should be valid");

        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].action,
            "team_sharing.manifest_ledger.exported_filtered"
        );
    }

    #[test]
    fn manifest_ledger_checksum_changes_with_event_metadata() {
        let original = [AuditLog {
            id: 1,
            actor: "system".to_string(),
            action: "team_sharing.manifest.inspected".to_string(),
            resource: "Verified signer abc".to_string(),
            severity: "Info".to_string(),
            created_at: "2026-07-19 10:30".to_string(),
        }];
        let changed = [AuditLog {
            id: 1,
            actor: "system".to_string(),
            action: "team_sharing.manifest.inspected".to_string(),
            resource: "Verified signer abc".to_string(),
            severity: "Warning".to_string(),
            created_at: "2026-07-19 10:30".to_string(),
        }];

        assert_ne!(
            team_sharing_manifest_ledger_checksum(&original),
            team_sharing_manifest_ledger_checksum(&changed)
        );
    }

    #[test]
    fn reliability_report_includes_only_health_and_snapshot_metadata() {
        let markdown = database_reliability_markdown(
            &DatabaseReliabilityStatus {
                integrity_status: "Healthy".to_string(),
                foreign_key_issues: 0,
                journal_mode: "WAL".to_string(),
                database_bytes: 4096,
                migration_count: 12,
                backup_count: 2,
                last_backup: Some("cymos-manual-backup-1.db".to_string()),
            },
            &[DatabaseBackupSnapshot {
                path: "/private/cymos/backups/cymos-manual-backup-1.db".to_string(),
                file_name: "cymos-manual-backup-1.db".to_string(),
                bytes: 2048,
                modified_at_unix: 1_784_282_400,
            }],
        );

        assert!(markdown.contains("# CYMOS Vault Reliability Report"));
        assert!(markdown.contains("**Integrity:** Healthy"));
        assert!(markdown.contains("cymos-manual-backup-1.db"));
        assert!(markdown.contains("Report data SHA-256"));
        assert!(!markdown.contains("/private/cymos/backups"));
    }

    #[test]
    fn reliability_report_checksum_changes_with_snapshot_metadata() {
        let reliability = DatabaseReliabilityStatus {
            integrity_status: "Healthy".to_string(),
            foreign_key_issues: 0,
            journal_mode: "WAL".to_string(),
            database_bytes: 4096,
            migration_count: 12,
            backup_count: 1,
            last_backup: Some("cymos-manual-backup-1.db".to_string()),
        };
        let original = [DatabaseBackupSnapshot {
            path: "/private/cymos/backups/cymos-manual-backup-1.db".to_string(),
            file_name: "cymos-manual-backup-1.db".to_string(),
            bytes: 2048,
            modified_at_unix: 1_784_282_400,
        }];
        let changed = [DatabaseBackupSnapshot {
            path: "/private/cymos/backups/cymos-manual-backup-1.db".to_string(),
            file_name: "cymos-manual-backup-1.db".to_string(),
            bytes: 4096,
            modified_at_unix: 1_784_282_400,
        }];
        let relocated = [DatabaseBackupSnapshot {
            path: "/another-machine/backups/cymos-manual-backup-1.db".to_string(),
            file_name: "cymos-manual-backup-1.db".to_string(),
            bytes: 2048,
            modified_at_unix: 1_784_282_400,
        }];

        assert_ne!(
            database_reliability_checksum(&reliability, &original),
            database_reliability_checksum(&reliability, &changed)
        );
        assert_eq!(
            database_reliability_checksum(&reliability, &original),
            database_reliability_checksum(&reliability, &relocated)
        );
    }

    #[test]
    fn runbook_export_includes_operational_provenance() {
        let markdown = runbook_entry_markdown(&RunbookEntry {
            id: -7,
            incident_id: None,
            incident_title: "Manual runbook".to_string(),
            workspace_name: "Local vault".to_string(),
            title: "Restart PostgreSQL safely".to_string(),
            details: "Check replicas, then restart the service.".to_string(),
            tags: vec!["postgresql".to_string(), "maintenance".to_string()],
            created_at: "2026-07-17 10:30".to_string(),
            latest_revision: 3,
            last_reviewed_revision: Some(3),
            last_reviewed_at: Some("2026-07-17 10:45".to_string()),
            last_review_note: Some("Validated against the replica failover checklist.".to_string()),
            review_status: "Reviewed".to_string(),
        });

        assert!(markdown.contains("# Restart PostgreSQL safely"));
        assert!(markdown.contains("- Source: Manual runbook"));
        assert!(markdown.contains("- Tags: postgresql, maintenance"));
        assert!(markdown.contains("- Revision: 3"));
        assert!(markdown.contains("- Review status: Reviewed"));
        assert!(markdown.contains("- Last reviewed: 2026-07-17 10:45"));
        assert!(markdown
            .contains("- Review evidence: Validated against the replica failover checklist."));
        assert!(markdown.contains("## Procedure"));
    }

    fn event(id: i64, session_id: i64, title: &str, incident_id: Option<i64>) -> InsightTrailEvent {
        InsightTrailEvent {
            id,
            event_type: "Error".to_string(),
            title: title.to_string(),
            details: "Captured event details.".to_string(),
            source_application: "Terminal".to_string(),
            severity: "Warning".to_string(),
            created_at: "2026-07-17 10:00".to_string(),
            memory_id: None,
            screenshot_path: None,
            incident_id,
            session_id: Some(session_id),
            tags: Vec::new(),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let database = tauri::async_runtime::block_on(Database::connect(app_data_dir))?;
            app.manage(AppState {
                database,
                maintenance_lock: Arc::new(Mutex::new(())),
                graph_lock: Arc::new(Mutex::new(())),
                ingestion_limiter: Arc::new(Semaphore::new(2)),
            });
            clipboard::start_monitor(app.handle().clone());
            autonomous::start_scheduler(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_clipboard_items,
            capture_browser_bookmark,
            capture_ide_snippet,
            capture_terminal_command,
            get_collections,
            create_collection,
            move_item_to_collection,
            toggle_favorite,
            delete_clipboard_item,
            get_clipboard_stats,
            get_insight_trail_events,
            get_insight_trail_overview,
            get_insight_trail_settings,
            update_insight_trail_settings,
            record_insight_trail_note,
            get_insight_incidents,
            resolve_insight_incident,
            apply_insight_trail_retention,
            get_cognitive_workspace,
            update_cognitive_workspace,
            get_cognitive_workspaces,
            get_workspace_snapshot,
            create_cognitive_workspace,
            start_workspace_session,
            end_workspace_session,
            archive_cognitive_workspace,
            restore_cognitive_workspace,
            import_workspace_document,
            record_workspace_incident_resolution,
            reopen_workspace_incident,
            link_workspace_incident_evidence,
            export_workspace_report,
            export_workspace_handoff,
            get_workspace_handoff_readiness,
            get_workspace_handoff_exports,
            get_handoff_recipient_trust_records,
            trust_handoff_recipient,
            revoke_handoff_recipient,
            get_handoff_signer_trust_records,
            trust_handoff_signer,
            revoke_handoff_signer,
            trust_current_device_team_sharing_signer,
            inspect_workspace_handoff,
            get_workspace_handoff_inspections,
            import_workspace_handoff,
            search_runbook_entries,
            get_runbook_audit_logs,
            create_manual_runbook,
            update_manual_runbook,
            delete_manual_runbook,
            export_runbook_entry,
            copy_runbook_entry,
            get_manual_runbook_revisions,
            restore_manual_runbook_revision,
            review_manual_runbook,
            get_similar_memories,
            import_terminal_history,
            rebuild_semantic_index,
            get_knowledge_graph,
            rebuild_knowledge_graph,
            ask_memory_assistant,
            get_daily_knowledge_summary,
            get_weekly_learning_report,
            run_agent_workflow,
            get_agent_workflows,
            run_autonomous_cycle,
            get_knowledge_health,
            get_database_reliability,
            get_database_reliability_checksum,
            create_verified_backup,
            verify_latest_backup,
            verify_database_backup_snapshot,
            get_recent_database_backups,
            get_recent_database_reliability_reports,
            export_database_reliability_report,
            get_privacy_status,
            update_privacy_settings,
            get_vault_retention_settings,
            update_vault_retention_settings,
            apply_vault_retention,
            get_automation_tasks,
            get_smart_notifications,
            get_intelligence_reports,
            run_universal_sync_cycle,
            get_platform_summary,
            get_team_sharing_policy,
            update_team_sharing_policy,
            get_team_sharing_readiness,
            run_team_sharing_sync_dry_run,
            get_sync_devices,
            register_team_sharing_device,
            approve_team_sharing_device,
            revoke_team_sharing_device,
            get_integration_connectors,
            get_plugin_records,
            get_api_clients,
            get_audit_logs,
            get_team_sharing_audit_logs,
            get_team_sharing_manifest_ledger_audit_logs,
            export_team_sharing_readiness_report,
            export_team_sharing_manifest_ledger,
            export_filtered_team_sharing_manifest_ledger,
            get_team_sharing_manifest_ledger_checksum,
            export_team_sharing_sync_dry_run_manifest,
            inspect_team_sharing_sync_dry_run_manifest,
            run_cognitive_release_check,
            get_cognitive_overview,
            get_cognitive_modules,
            get_enterprise_controls,
            get_cognitive_use_cases,
            copy_clipboard_item,
            export_clipboard_item
        ])
        .run(tauri::generate_context!())
        .expect("failed to run CYMOS");
}

fn main() {
    run();
}
