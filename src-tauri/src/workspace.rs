use serde::{Deserialize, Serialize};

pub const DEFAULT_WORKSPACE_NAME: &str = "Local operations";
pub const DEFAULT_WORKSPACE_PROJECT: &str = "Personal memory";

#[derive(Debug, Serialize)]
pub struct CognitiveWorkspace {
    pub id: i64,
    pub name: String,
    pub project: String,
    pub status: String,
    pub is_imported: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_event_at: Option<String>,
    pub event_count: i64,
    pub memory_count: i64,
    pub error_count: i64,
    pub sources: Vec<String>,
    pub top_topics: Vec<String>,
    pub summary: String,
    pub next_signal: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSession {
    pub id: i64,
    pub workspace_id: i64,
    pub title: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub event_count: i64,
}

#[derive(Debug, Serialize)]
pub struct IncidentResolution {
    pub id: i64,
    pub incident_id: i64,
    pub workspace_id: i64,
    pub workspace_name: String,
    pub session_id: Option<i64>,
    pub title: String,
    pub details: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct RunbookEntry {
    pub id: i64,
    pub incident_id: Option<i64>,
    pub incident_title: String,
    pub workspace_name: String,
    pub title: String,
    pub details: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub latest_revision: i64,
    pub last_reviewed_revision: Option<i64>,
    pub last_reviewed_at: Option<String>,
    pub last_review_note: Option<String>,
    pub review_status: String,
}

#[derive(Debug, Serialize)]
pub struct RunbookRevision {
    pub id: i64,
    pub runbook_id: i64,
    pub revision: i64,
    pub title: String,
    pub details: String,
    pub tags: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceImportProvenance {
    pub source_workspace: String,
    pub source_project: String,
    pub source_scope: String,
    pub source_recipient: String,
    pub source_purpose: String,
    pub source_classification: String,
    pub source_expires_at_unix: Option<i64>,
    pub source_signer_fingerprint: Option<String>,
    pub source_generated_at: String,
    pub checksum: String,
    pub imported_at: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceHandoffExportRecord {
    pub id: i64,
    pub workspace_id: i64,
    pub session_id: Option<i64>,
    pub scope: String,
    pub recipient: String,
    pub purpose: String,
    pub classification: String,
    pub expires_at_unix: Option<i64>,
    pub signer_fingerprint: String,
    pub package_sha256: String,
    pub package_bytes: i64,
    pub event_count: i64,
    pub excluded_event_count: i64,
    pub incident_count: i64,
    pub resolution_count: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct HandoffRecipientTrustRecord {
    pub id: i64,
    pub recipient: String,
    pub max_classification: String,
    pub note: String,
    pub is_active: bool,
    pub export_count: i64,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceHandoffInspectionRecord {
    pub id: i64,
    pub status: String,
    pub workspace_name: Option<String>,
    pub classification: Option<String>,
    pub signer_fingerprint: Option<String>,
    pub package_sha256: String,
    pub payload_sha256: Option<String>,
    pub failure_reason: Option<String>,
    pub package_bytes: i64,
    pub inspected_at: String,
}

#[derive(Debug, Serialize)]
pub struct HandoffSignerTrustRecord {
    pub id: i64,
    pub signer_fingerprint: String,
    pub label: String,
    pub is_active: bool,
    pub import_count: i64,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceSnapshot {
    pub workspace: CognitiveWorkspace,
    pub sessions: Vec<WorkspaceSession>,
    pub active_session: Option<WorkspaceSession>,
    pub events: Vec<crate::insight_trail::InsightTrailEvent>,
    pub incidents: Vec<crate::insight_trail::InsightIncident>,
    pub resolutions: Vec<IncidentResolution>,
    pub import_provenance: Option<WorkspaceImportProvenance>,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceContextUpdate {
    pub name: String,
    pub project: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceCreateRequest {
    pub name: String,
    pub project: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceSessionStartRequest {
    pub workspace_id: i64,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct IncidentResolutionRequest {
    pub workspace_id: i64,
    pub incident_id: i64,
    pub title: String,
    pub details: String,
}

#[derive(Debug, Deserialize)]
pub struct IncidentReopenRequest {
    pub workspace_id: i64,
    pub incident_id: i64,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct IncidentEvidenceLinkRequest {
    pub workspace_id: i64,
    pub incident_id: i64,
    pub event_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceReportRequest {
    pub workspace_id: i64,
    pub session_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceHandoffRequest {
    pub workspace_id: i64,
    pub session_id: Option<i64>,
    #[serde(default)]
    pub excluded_event_ids: Vec<i64>,
    #[serde(default)]
    pub recipient: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default = "default_handoff_classification")]
    pub classification: String,
    #[serde(default)]
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct HandoffRecipientTrustRequest {
    pub recipient: String,
    pub max_classification: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Deserialize)]
pub struct HandoffRecipientRevokeRequest {
    pub recipient: String,
}

#[derive(Debug, Deserialize)]
pub struct HandoffSignerTrustRequest {
    pub signer_fingerprint: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct HandoffSignerRevokeRequest {
    pub signer_fingerprint: String,
}

fn default_handoff_classification() -> String {
    "Internal".to_string()
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceDocumentImportRequest {
    pub workspace_id: i64,
    pub file_name: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceHandoffImportRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceDocumentImportResult {
    pub stored: bool,
    pub snapshot: WorkspaceSnapshot,
}

#[derive(Debug, Deserialize)]
pub struct RunbookSearchRequest {
    pub query: String,
    pub review_status: String,
}

#[derive(Debug, Deserialize)]
pub struct ManualRunbookRequest {
    pub title: String,
    pub details: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ManualRunbookUpdateRequest {
    pub id: i64,
    pub title: String,
    pub details: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ManualRunbookRevisionRestoreRequest {
    pub entry_id: i64,
    pub revision_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct ManualRunbookReviewRequest {
    pub entry_id: i64,
    pub note: String,
}

pub fn summary(event_count: i64, memory_count: i64, error_count: i64, project: &str) -> String {
    if event_count == 0 {
        return format!("{project} is ready for its first captured work session.");
    }

    let event_label = if event_count == 1 { "event" } else { "events" };
    let memory_label = if memory_count == 1 {
        "memory"
    } else {
        "memories"
    };
    if error_count > 0 {
        format!(
            "{project} has {event_count} {event_label} connected to {memory_count} {memory_label}, including {error_count} error signal(s) to review."
        )
    } else {
        format!(
            "{project} has {event_count} {event_label} connected to {memory_count} {memory_label}."
        )
    }
}

pub fn next_signal(error_count: i64, event_count: i64, top_topics: &[String]) -> String {
    if error_count > 0 {
        return "Review the linked incident memory before repeating the workflow.".to_string();
    }
    if let Some(topic) = top_topics.first() {
        return format!("Keep the {topic} context together as this workspace grows.");
    }
    if event_count > 0 {
        return "Continue capturing the decisions and outcomes around this work.".to_string();
    }
    "Start a session note to give this workspace a durable point of reference.".to_string()
}

#[cfg(test)]
mod tests {
    use super::{next_signal, summary};

    #[test]
    fn summarizes_error_signals_as_reviewable_context() {
        assert_eq!(
            summary(3, 2, 1, "Nginx rollout"),
            "Nginx rollout has 3 events connected to 2 memories, including 1 error signal(s) to review."
        );
        assert_eq!(
            next_signal(1, 3, &["Nginx".to_string()]),
            "Review the linked incident memory before repeating the workflow."
        );
    }
}
