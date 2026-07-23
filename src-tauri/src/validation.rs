use crate::clipboard::{BrowserBookmarkRequest, IdeSnippetRequest, TerminalCommandRequest};
use crate::database::{SearchRequest, TeamSharingPolicy, VaultRetentionSettings};
use crate::insight_trail::{
    InsightTrailNoteRequest, InsightTrailSearchRequest, InsightTrailSettings,
};
use crate::operations;
use crate::platform::{TeamSharingDeviceRequest, TeamSharingDeviceStatusRequest};
use crate::privacy::PrivacySettings;
use crate::workspace::{
    HandoffRecipientRevokeRequest, HandoffRecipientTrustRequest, HandoffSignerRevokeRequest,
    HandoffSignerTrustRequest, IncidentEvidenceLinkRequest, IncidentReopenRequest,
    IncidentResolutionRequest, ManualRunbookRequest, ManualRunbookReviewRequest,
    ManualRunbookRevisionRestoreRequest, ManualRunbookUpdateRequest, RunbookSearchRequest,
    WorkspaceContextUpdate, WorkspaceCreateRequest, WorkspaceDocumentImportRequest,
    WorkspaceHandoffRequest, WorkspaceReportRequest, WorkspaceSessionStartRequest,
};

pub const MAX_SEARCH_QUERY_CHARS: usize = 512;
pub const MAX_COLLECTION_NAME_CHARS: usize = 80;
pub const MAX_ASSISTANT_QUESTION_CHARS: usize = 2_000;
pub const MAX_AGENT_GOAL_CHARS: usize = 2_000;
pub const MAX_CLIPBOARD_TEXT_BYTES: usize = 1_048_576;
pub const MAX_CLIPBOARD_IMAGE_BYTES: usize = 64 * 1_024 * 1_024;
pub const MAX_CLIPBOARD_IMAGE_PIXELS: usize = 40_000_000;
pub const MAX_HANDOFF_RECIPIENT_CHARS: usize = 120;
pub const MAX_HANDOFF_PURPOSE_CHARS: usize = 240;
pub const MAX_HANDOFF_TRUST_NOTE_CHARS: usize = 240;
pub const MAX_HANDOFF_SIGNER_LABEL_CHARS: usize = 120;
pub const MAX_TEAM_DEVICE_NAME_CHARS: usize = 120;
pub const MAX_TEAM_DEVICE_PLATFORM_CHARS: usize = 80;
pub const MAX_ANALYSIS_CHARS: usize = 16_000;
pub const MAX_INSIGHT_NOTE_TITLE_CHARS: usize = 120;
pub const MAX_INSIGHT_NOTE_DETAILS_CHARS: usize = 1_500;
pub const MAX_INSIGHT_TAGS: usize = 12;
pub const MAX_WORKSPACE_CONTEXT_CHARS: usize = 120;
pub const MAX_BOOKMARK_TITLE_CHARS: usize = 160;
pub const MAX_IDE_PROJECT_CHARS: usize = 120;
pub const MAX_IDE_FILE_PATH_CHARS: usize = 512;
pub const MAX_TERMINAL_COMMAND_CHARS: usize = 4_000;
pub const MAX_TERMINAL_HOST_CHARS: usize = 253;

const IDE_LANGUAGES: &[&str] = &[
    "Auto",
    "Bash",
    "C/C++",
    "CSS",
    "HTML",
    "Java",
    "JavaScript",
    "JSON",
    "Python",
    "Rust",
    "SQL",
    "TypeScript",
    "YAML",
];

const CONTENT_TYPES: &[&str] = &[
    "All", "Text", "URL", "Code", "Image", "File", "Folder", "Color", "Table", "HTML",
];
const INSIGHT_EVENT_TYPES: &[&str] = &[
    "All",
    "Clipboard",
    "Terminal",
    "Screenshot",
    "Error",
    "Note",
];

pub fn item_id(value: i64) -> Result<(), String> {
    if value > 0 {
        Ok(())
    } else {
        Err("A valid memory id is required.".to_string())
    }
}

pub fn search(request: &SearchRequest) -> Result<(), String> {
    bounded_text("Search query", &request.query, MAX_SEARCH_QUERY_CHARS, true)?;
    bounded_text("Tag", &request.tag, MAX_COLLECTION_NAME_CHARS, true)?;
    bounded_text(
        "Category",
        &request.category,
        MAX_COLLECTION_NAME_CHARS,
        true,
    )?;

    if !CONTENT_TYPES.contains(&request.content_type.as_str()) {
        return Err("Unsupported memory type filter.".to_string());
    }
    if let Some(collection_id) = request.collection_id {
        item_id(collection_id)?;
    }
    Ok(())
}

pub fn collection(name: &str, color: &str) -> Result<(), String> {
    bounded_text("Collection name", name, MAX_COLLECTION_NAME_CHARS, false)?;
    if !is_hex_color(color) {
        return Err("Collection color must use the #RRGGBB format.".to_string());
    }
    Ok(())
}

pub fn browser_bookmark(request: &BrowserBookmarkRequest) -> Result<(), String> {
    bounded_text("Bookmark URL", &request.url, MAX_SEARCH_QUERY_CHARS, false)?;
    bounded_text(
        "Bookmark title",
        &request.title,
        MAX_BOOKMARK_TITLE_CHARS,
        true,
    )?;
    let url = request.url.trim();
    let host = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|value| value.split(['/', '?', '#']).next());
    if host.is_none_or(|value| value.is_empty()) || url.chars().any(char::is_whitespace) {
        return Err("Bookmark URL must be a valid HTTP or HTTPS address.".to_string());
    }
    if request.tags.len() > MAX_INSIGHT_TAGS {
        return Err("A bookmark can have up to 12 tags.".to_string());
    }
    for tag in &request.tags {
        bounded_text("Bookmark tag", tag, MAX_COLLECTION_NAME_CHARS, false)?;
    }
    Ok(())
}

pub fn ide_snippet(request: &IdeSnippetRequest) -> Result<(), String> {
    bounded_text(
        "IDE snippet",
        &request.content,
        MAX_CLIPBOARD_TEXT_BYTES,
        false,
    )?;
    clipboard_text(&request.content)?;
    bounded_text(
        "IDE snippet title",
        &request.title,
        MAX_BOOKMARK_TITLE_CHARS,
        true,
    )?;
    bounded_text("IDE project", &request.project, MAX_IDE_PROJECT_CHARS, true)?;
    bounded_text(
        "IDE file path",
        &request.file_path,
        MAX_IDE_FILE_PATH_CHARS,
        true,
    )?;
    if !IDE_LANGUAGES.contains(&request.language.as_str()) {
        return Err("Unsupported IDE snippet language.".to_string());
    }
    if request.tags.len() > MAX_INSIGHT_TAGS {
        return Err("An IDE snippet can have up to 12 tags.".to_string());
    }
    for tag in &request.tags {
        bounded_text("IDE snippet tag", tag, MAX_COLLECTION_NAME_CHARS, false)?;
    }
    Ok(())
}

pub fn terminal_command(request: &TerminalCommandRequest) -> Result<(), String> {
    bounded_text(
        "Terminal command",
        &request.command,
        MAX_TERMINAL_COMMAND_CHARS,
        false,
    )?;
    terminal_shell(&request.shell)?;
    bounded_text(
        "Terminal host",
        &request.host,
        MAX_TERMINAL_HOST_CHARS,
        true,
    )?;
    if request.host.chars().any(char::is_whitespace) {
        return Err("Terminal host cannot contain whitespace.".to_string());
    }
    bounded_text(
        "Terminal project",
        &request.project,
        MAX_IDE_PROJECT_CHARS,
        true,
    )?;
    if operations::is_sensitive_command(&request.command) {
        return Err(
            "Terminal command appears to contain a sensitive value and was not saved.".to_string(),
        );
    }
    if request.tags.len() > MAX_INSIGHT_TAGS {
        return Err("A terminal command can have up to 12 tags.".to_string());
    }
    for tag in &request.tags {
        bounded_text(
            "Terminal command tag",
            tag,
            MAX_COLLECTION_NAME_CHARS,
            false,
        )?;
    }
    Ok(())
}

pub fn assistant_question(value: &str) -> Result<(), String> {
    bounded_text("Question", value, MAX_ASSISTANT_QUESTION_CHARS, false)
}

pub fn agent_goal(value: &str) -> Result<(), String> {
    bounded_text("Agent goal", value, MAX_AGENT_GOAL_CHARS, false)
}

pub fn export_format(value: &str) -> Result<(), String> {
    if matches!(value, "JSON" | "CSV" | "Markdown" | "Text") {
        Ok(())
    } else {
        Err("Unsupported export format.".to_string())
    }
}

pub fn terminal_shell(value: &str) -> Result<(), String> {
    if matches!(value, "Bash" | "Zsh") {
        Ok(())
    } else {
        Err("Unsupported terminal history source.".to_string())
    }
}

pub fn terminal_history_limit(value: usize) -> Result<(), String> {
    if (1..=crate::terminal_history::MAX_HISTORY_COMMANDS).contains(&value) {
        Ok(())
    } else {
        Err("Terminal history import must include between 1 and 1000 commands.".to_string())
    }
}

pub fn handoff_package(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err("Handoff package cannot be empty.".to_string());
    }
    if value.as_bytes().len() > MAX_CLIPBOARD_TEXT_BYTES {
        return Err("Handoff package exceeds the 1 MB local verification limit.".to_string());
    }
    Ok(())
}

pub fn insight_trail_search(request: &InsightTrailSearchRequest) -> Result<(), String> {
    bounded_text(
        "Timeline search",
        &request.query,
        MAX_SEARCH_QUERY_CHARS,
        true,
    )?;
    if !INSIGHT_EVENT_TYPES.contains(&request.event_type.as_str()) {
        return Err("Unsupported timeline event filter.".to_string());
    }
    if !(1..=200).contains(&request.limit) {
        return Err("Timeline result limit must be between 1 and 200.".to_string());
    }
    Ok(())
}

pub fn insight_trail_note(request: &InsightTrailNoteRequest) -> Result<(), String> {
    bounded_text(
        "Timeline note title",
        &request.title,
        MAX_INSIGHT_NOTE_TITLE_CHARS,
        false,
    )?;
    bounded_text(
        "Timeline note",
        &request.details,
        MAX_INSIGHT_NOTE_DETAILS_CHARS,
        false,
    )?;
    if request.tags.len() > MAX_INSIGHT_TAGS {
        return Err("A timeline note can have up to 12 tags.".to_string());
    }
    for tag in &request.tags {
        bounded_text("Timeline tag", tag, MAX_COLLECTION_NAME_CHARS, false)?;
    }
    Ok(())
}

pub fn insight_trail_settings(settings: &InsightTrailSettings) -> Result<(), String> {
    if !(1..=3_650).contains(&settings.retention_days) {
        return Err("Retention must be between 1 and 3650 days.".to_string());
    }
    if !(64..=102_400).contains(&settings.max_storage_mb) {
        return Err("Storage limit must be between 64 MB and 102400 MB.".to_string());
    }
    if settings.excluded_applications.len() > MAX_INSIGHT_TAGS {
        return Err("You can exclude up to 12 applications.".to_string());
    }
    for application in &settings.excluded_applications {
        bounded_text(
            "Excluded application",
            application,
            MAX_COLLECTION_NAME_CHARS,
            false,
        )?;
    }
    Ok(())
}

pub fn workspace_context(update: &WorkspaceContextUpdate) -> Result<(), String> {
    bounded_text(
        "Workspace name",
        &update.name,
        MAX_WORKSPACE_CONTEXT_CHARS,
        false,
    )?;
    bounded_text(
        "Workspace project",
        &update.project,
        MAX_WORKSPACE_CONTEXT_CHARS,
        false,
    )
}

pub fn workspace_create(request: &WorkspaceCreateRequest) -> Result<(), String> {
    bounded_text(
        "Workspace name",
        &request.name,
        MAX_WORKSPACE_CONTEXT_CHARS,
        false,
    )?;
    bounded_text(
        "Workspace project",
        &request.project,
        MAX_WORKSPACE_CONTEXT_CHARS,
        false,
    )
}

pub fn workspace_session_start(request: &WorkspaceSessionStartRequest) -> Result<(), String> {
    item_id(request.workspace_id)?;
    bounded_text(
        "Session title",
        &request.title,
        MAX_WORKSPACE_CONTEXT_CHARS,
        false,
    )
}

pub fn incident_resolution(request: &IncidentResolutionRequest) -> Result<(), String> {
    item_id(request.workspace_id)?;
    item_id(request.incident_id)?;
    bounded_text(
        "Resolution title",
        &request.title,
        MAX_INSIGHT_NOTE_TITLE_CHARS,
        false,
    )?;
    bounded_text(
        "Resolution details",
        &request.details,
        MAX_INSIGHT_NOTE_DETAILS_CHARS,
        false,
    )
}

pub fn incident_reopen(request: &IncidentReopenRequest) -> Result<(), String> {
    item_id(request.workspace_id)?;
    item_id(request.incident_id)?;
    bounded_text(
        "Reopen reason",
        &request.reason,
        MAX_INSIGHT_NOTE_DETAILS_CHARS,
        false,
    )
}

pub fn incident_evidence_link(request: &IncidentEvidenceLinkRequest) -> Result<(), String> {
    item_id(request.workspace_id)?;
    item_id(request.incident_id)?;
    item_id(request.event_id)
}

pub fn workspace_report(request: &WorkspaceReportRequest) -> Result<(), String> {
    item_id(request.workspace_id)?;
    if let Some(session_id) = request.session_id {
        item_id(session_id)?;
    }
    Ok(())
}

pub fn workspace_handoff(request: &WorkspaceHandoffRequest) -> Result<(), String> {
    item_id(request.workspace_id)?;
    if let Some(session_id) = request.session_id {
        item_id(session_id)?;
    }
    if request.excluded_event_ids.len() > 200 {
        return Err("A handoff can exclude at most 200 timeline events.".to_string());
    }

    let mut event_ids = request.excluded_event_ids.clone();
    for event_id in &event_ids {
        item_id(*event_id)?;
    }
    event_ids.sort_unstable();
    if event_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("A handoff event can only be excluded once.".to_string());
    }
    bounded_text(
        "Handoff recipient",
        &request.recipient,
        MAX_HANDOFF_RECIPIENT_CHARS,
        false,
    )?;
    bounded_text(
        "Handoff purpose",
        &request.purpose,
        MAX_HANDOFF_PURPOSE_CHARS,
        false,
    )?;
    if !matches!(
        request.classification.as_str(),
        "Internal" | "Restricted" | "Confidential"
    ) {
        return Err("Choose a supported handoff classification.".to_string());
    }
    if let Some(expires_in_days) = request.expires_in_days {
        if !matches!(expires_in_days, 1 | 7 | 30) {
            return Err("Choose a supported handoff expiry period.".to_string());
        }
    }
    if request.classification == "Confidential" && request.expires_in_days.is_none() {
        return Err("Confidential handoffs require a 1, 7, or 30 day expiry.".to_string());
    }
    if crate::privacy::sensitive_reason(&format!("{}\n{}", request.recipient, request.purpose))
        .is_some()
    {
        return Err(
            "Handoff declaration cannot contain sensitive configuration values.".to_string(),
        );
    }
    Ok(())
}

pub fn handoff_recipient_trust(request: &HandoffRecipientTrustRequest) -> Result<(), String> {
    bounded_text(
        "Trusted handoff recipient",
        &request.recipient,
        MAX_HANDOFF_RECIPIENT_CHARS,
        false,
    )?;
    bounded_text(
        "Trusted recipient note",
        &request.note,
        MAX_HANDOFF_TRUST_NOTE_CHARS,
        true,
    )?;
    if !matches!(
        request.max_classification.as_str(),
        "Internal" | "Restricted" | "Confidential"
    ) {
        return Err("Choose a supported trusted recipient classification.".to_string());
    }
    if crate::privacy::sensitive_reason(&format!("{}\n{}", request.recipient, request.note))
        .is_some()
    {
        return Err("Trusted recipient details cannot contain sensitive values.".to_string());
    }
    Ok(())
}

pub fn handoff_recipient_revoke(request: &HandoffRecipientRevokeRequest) -> Result<(), String> {
    bounded_text(
        "Trusted handoff recipient",
        &request.recipient,
        MAX_HANDOFF_RECIPIENT_CHARS,
        false,
    )
}

pub fn handoff_signer_trust(request: &HandoffSignerTrustRequest) -> Result<(), String> {
    signer_fingerprint(&request.signer_fingerprint)?;
    bounded_text(
        "Trusted signer label",
        &request.label,
        MAX_HANDOFF_SIGNER_LABEL_CHARS,
        true,
    )?;
    if crate::privacy::sensitive_reason(&request.label).is_some() {
        return Err("Trusted signer label cannot contain sensitive values.".to_string());
    }
    Ok(())
}

pub fn handoff_signer_revoke(request: &HandoffSignerRevokeRequest) -> Result<(), String> {
    signer_fingerprint(&request.signer_fingerprint)
}

fn signer_fingerprint(value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.len() != 16
        || !trimmed
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err("Trusted signer fingerprint must be 16 hex characters.".to_string());
    }
    Ok(())
}

pub fn workspace_document_import(request: &WorkspaceDocumentImportRequest) -> Result<(), String> {
    item_id(request.workspace_id)?;
    bounded_text(
        "File name",
        &request.file_name,
        MAX_WORKSPACE_CONTEXT_CHARS,
        false,
    )?;
    if request.file_name.contains('/') || request.file_name.contains('\\') {
        return Err("Choose a file name without a path.".to_string());
    }
    clipboard_text(&request.content)
}

pub fn runbook_search(request: &RunbookSearchRequest) -> Result<(), String> {
    bounded_text(
        "Runbook search",
        &request.query,
        MAX_SEARCH_QUERY_CHARS,
        true,
    )?;
    if matches!(
        request.review_status.as_str(),
        "All" | "Needs review" | "Review due" | "Reviewed"
    ) {
        Ok(())
    } else {
        Err("Unsupported runbook review filter.".to_string())
    }
}

pub fn manual_runbook(request: &ManualRunbookRequest) -> Result<(), String> {
    bounded_text(
        "Runbook title",
        &request.title,
        MAX_INSIGHT_NOTE_TITLE_CHARS,
        false,
    )?;
    bounded_text(
        "Runbook details",
        &request.details,
        MAX_INSIGHT_NOTE_DETAILS_CHARS,
        false,
    )?;
    if request.tags.len() > MAX_INSIGHT_TAGS {
        return Err("A runbook can have up to 12 tags.".to_string());
    }
    for tag in &request.tags {
        bounded_text("Runbook tag", tag, MAX_COLLECTION_NAME_CHARS, false)?;
    }
    Ok(())
}

pub fn manual_runbook_update(request: &ManualRunbookUpdateRequest) -> Result<(), String> {
    if request.id >= 0 {
        return Err("Only standalone local runbooks can be edited.".to_string());
    }
    manual_runbook(&ManualRunbookRequest {
        title: request.title.clone(),
        details: request.details.clone(),
        tags: request.tags.clone(),
    })
}

pub fn manual_runbook_delete(id: i64) -> Result<(), String> {
    if id < 0 {
        Ok(())
    } else {
        Err("Only standalone local runbooks can be deleted.".to_string())
    }
}

pub fn manual_runbook_revision_restore(
    request: &ManualRunbookRevisionRestoreRequest,
) -> Result<(), String> {
    manual_runbook_delete(request.entry_id)?;
    if request.revision_id > 0 {
        Ok(())
    } else {
        Err("A valid runbook revision is required.".to_string())
    }
}

pub fn manual_runbook_review(request: &ManualRunbookReviewRequest) -> Result<(), String> {
    manual_runbook_delete(request.entry_id)?;
    bounded_text("Runbook review note", &request.note, 500, true)
}

pub fn runbook_entry_id(id: i64) -> Result<(), String> {
    if id == 0 || id == i64::MIN {
        Err("A valid runbook entry is required.".to_string())
    } else {
        Ok(())
    }
}

pub fn privacy_settings(settings: &PrivacySettings) -> Result<(), String> {
    let _ = settings;
    Ok(())
}

pub fn vault_retention_settings(settings: &VaultRetentionSettings) -> Result<(), String> {
    if !(1..=3_650).contains(&settings.retention_days) {
        return Err("Vault retention must be between 1 and 3650 days.".to_string());
    }
    if !(100..=1_000_000).contains(&settings.max_items) {
        return Err("Vault item limit must be between 100 and 1000000 items.".to_string());
    }
    if !(64..=102_400).contains(&settings.max_storage_mb) {
        return Err("Vault storage limit must be between 64 MB and 102400 MB.".to_string());
    }
    Ok(())
}

pub fn team_sharing_policy(policy: &TeamSharingPolicy) -> Result<(), String> {
    match policy.mode.as_str() {
        "LocalOnly" | "SelfHosted" | "EncryptedCloud" => {}
        _ => {
            return Err(
                "Team sharing mode must be LocalOnly, SelfHosted, or EncryptedCloud.".to_string(),
            )
        }
    }
    if !(1..=365).contains(&policy.retention_days) {
        return Err("Team sharing retention must be between 1 and 365 days.".to_string());
    }
    if policy.enabled && !policy.require_device_approval {
        return Err(
            "Team sharing requires explicit device approval before it can be enabled.".to_string(),
        );
    }
    if policy.enabled && !policy.require_recipient_trust {
        return Err(
            "Team sharing requires trusted recipients before it can be enabled.".to_string(),
        );
    }
    Ok(())
}

pub fn team_sharing_device(request: &TeamSharingDeviceRequest) -> Result<(), String> {
    bounded_text(
        "Device name",
        &request.device_name,
        MAX_TEAM_DEVICE_NAME_CHARS,
        false,
    )?;
    bounded_text(
        "Device platform",
        &request.platform,
        MAX_TEAM_DEVICE_PLATFORM_CHARS,
        false,
    )?;
    match request.sync_mode.as_str() {
        "Local-only" | "Self-hosted" | "Encrypted cloud" => Ok(()),
        _ => {
            Err("Device sync mode must be Local-only, Self-hosted, or Encrypted cloud.".to_string())
        }
    }
}

pub fn team_sharing_device_status(request: &TeamSharingDeviceStatusRequest) -> Result<(), String> {
    if request.device_id <= 0 {
        return Err("Device id must be positive.".to_string());
    }
    Ok(())
}

pub fn clipboard_text(value: &str) -> Result<(), String> {
    if value.as_bytes().len() > MAX_CLIPBOARD_TEXT_BYTES {
        return Err("Clipboard text exceeds the 1 MB capture limit.".to_string());
    }
    Ok(())
}

pub fn analysis_excerpt(value: &str) -> String {
    let mut characters = value.chars();
    let excerpt = characters
        .by_ref()
        .take(MAX_ANALYSIS_CHARS)
        .collect::<String>();
    if characters.next().is_some() {
        format!("{excerpt}\n[Content truncated for local analysis]")
    } else {
        excerpt
    }
}

pub fn clipboard_image(width: usize, height: usize, byte_len: usize) -> Result<(), String> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| "Clipboard image dimensions are invalid.".to_string())?;
    let expected_bytes = pixels
        .checked_mul(4)
        .ok_or_else(|| "Clipboard image dimensions are invalid.".to_string())?;

    if width == 0 || height == 0 || pixels > MAX_CLIPBOARD_IMAGE_PIXELS {
        return Err("Clipboard image exceeds the 40 megapixel capture limit.".to_string());
    }
    if byte_len != expected_bytes || byte_len > MAX_CLIPBOARD_IMAGE_BYTES {
        return Err(
            "Clipboard image payload is invalid or exceeds the 64 MB capture limit.".to_string(),
        );
    }
    Ok(())
}

fn bounded_text(
    label: &str,
    value: &str,
    max_chars: usize,
    allow_empty: bool,
) -> Result<(), String> {
    let trimmed = value.trim();
    if !allow_empty && trimmed.is_empty() {
        return Err(format!("{label} cannot be empty."));
    }
    if value.chars().count() > max_chars {
        return Err(format!("{label} is too long."));
    }
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(format!("{label} contains unsupported control characters."));
    }
    Ok(())
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value
            .chars()
            .skip(1)
            .all(|character| character.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{
        analysis_excerpt, browser_bookmark, clipboard_image, collection, handoff_recipient_revoke,
        handoff_recipient_trust, handoff_signer_revoke, handoff_signer_trust, ide_snippet,
        incident_evidence_link, item_id, team_sharing_device, team_sharing_device_status,
        team_sharing_policy, terminal_command, workspace_handoff, MAX_ANALYSIS_CHARS,
    };
    use crate::clipboard::{BrowserBookmarkRequest, IdeSnippetRequest, TerminalCommandRequest};
    use crate::platform::{
        TeamSharingDeviceRequest, TeamSharingDeviceStatusRequest, TeamSharingPolicy,
    };
    use crate::workspace::{
        HandoffRecipientRevokeRequest, HandoffRecipientTrustRequest, HandoffSignerRevokeRequest,
        HandoffSignerTrustRequest, IncidentEvidenceLinkRequest, WorkspaceHandoffRequest,
    };

    #[test]
    fn rejects_invalid_collection_color() {
        assert!(collection("Inbox", "blue").is_err());
    }

    #[test]
    fn rejects_zero_item_id() {
        assert!(item_id(0).is_err());
    }

    #[test]
    fn rejects_invalid_image_payload() {
        assert!(clipboard_image(2, 2, 15).is_err());
    }

    #[test]
    fn analysis_excerpt_respects_the_character_limit() {
        let content = "a".repeat(MAX_ANALYSIS_CHARS + 1);
        assert!(analysis_excerpt(&content).starts_with(&"a".repeat(MAX_ANALYSIS_CHARS)));
        assert!(analysis_excerpt(&content).contains("Content truncated"));
    }

    #[test]
    fn enabled_team_sharing_requires_local_approval_gates() {
        let mut policy = TeamSharingPolicy {
            enabled: true,
            ..TeamSharingPolicy::default()
        };
        assert!(team_sharing_policy(&policy).is_ok());

        policy.require_device_approval = false;
        assert!(team_sharing_policy(&policy).is_err());

        policy.require_device_approval = true;
        policy.require_recipient_trust = false;
        assert!(team_sharing_policy(&policy).is_err());
    }

    #[test]
    fn validates_team_sharing_device_records() {
        assert!(team_sharing_device(&TeamSharingDeviceRequest {
            device_name: "RHEL admin laptop".to_string(),
            platform: "RHEL 9".to_string(),
            sync_mode: "Local-only".to_string(),
        })
        .is_ok());
        assert!(team_sharing_device(&TeamSharingDeviceRequest {
            device_name: String::new(),
            platform: "macOS".to_string(),
            sync_mode: "Local-only".to_string(),
        })
        .is_err());
        assert!(
            team_sharing_device_status(&TeamSharingDeviceStatusRequest { device_id: 0 }).is_err()
        );
    }

    #[test]
    fn accepts_explicit_http_bookmarks_and_rejects_other_schemes() {
        let request = BrowserBookmarkRequest {
            url: "https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9".to_string(),
            title: "RHEL 9 documentation".to_string(),
            tags: vec!["RHEL".to_string(), "Linux".to_string()],
        };

        assert!(browser_bookmark(&request).is_ok());

        let invalid = BrowserBookmarkRequest {
            url: "file:///etc/passwd".to_string(),
            ..request
        };
        assert!(browser_bookmark(&invalid).is_err());
    }

    #[test]
    fn accepts_explicit_ide_snippets_and_rejects_unknown_languages() {
        let request = IdeSnippetRequest {
            content: "fn main() { println!(\"CYMOS\"); }".to_string(),
            title: "Application entry point".to_string(),
            language: "Rust".to_string(),
            project: "CtrlCove".to_string(),
            file_path: "src-tauri/src/main.rs".to_string(),
            tags: vec!["architecture".to_string()],
        };

        assert!(ide_snippet(&request).is_ok());

        let invalid = IdeSnippetRequest {
            language: "MadeUpLang".to_string(),
            ..request
        };
        assert!(ide_snippet(&invalid).is_err());
    }

    #[test]
    fn accepts_safe_terminal_commands_and_rejects_sensitive_ones() {
        let request = TerminalCommandRequest {
            command: "systemctl restart nginx".to_string(),
            shell: "Bash".to_string(),
            host: "web-01.internal".to_string(),
            project: "edge-platform".to_string(),
            tags: vec!["maintenance".to_string()],
        };

        assert!(terminal_command(&request).is_ok());

        let invalid = TerminalCommandRequest {
            command: "export API_TOKEN=not-for-storage".to_string(),
            ..request
        };
        assert!(terminal_command(&invalid).is_err());
    }

    #[test]
    fn rejects_invalid_incident_evidence_link_ids() {
        assert!(incident_evidence_link(&IncidentEvidenceLinkRequest {
            workspace_id: 1,
            incident_id: 2,
            event_id: 3,
        })
        .is_ok());
        assert!(incident_evidence_link(&IncidentEvidenceLinkRequest {
            workspace_id: 1,
            incident_id: 2,
            event_id: 0,
        })
        .is_err());
    }

    #[test]
    fn rejects_duplicate_handoff_exclusions() {
        assert!(workspace_handoff(&WorkspaceHandoffRequest {
            workspace_id: 1,
            session_id: Some(2),
            excluded_event_ids: vec![3, 4],
            recipient: "Platform operations".to_string(),
            purpose: "Incident escalation".to_string(),
            classification: "Restricted".to_string(),
            expires_in_days: Some(7),
        })
        .is_ok());
        assert!(workspace_handoff(&WorkspaceHandoffRequest {
            workspace_id: 1,
            session_id: Some(2),
            excluded_event_ids: vec![3, 3],
            recipient: "Platform operations".to_string(),
            purpose: "Incident escalation".to_string(),
            classification: "Restricted".to_string(),
            expires_in_days: Some(7),
        })
        .is_err());
        assert!(workspace_handoff(&WorkspaceHandoffRequest {
            workspace_id: 1,
            session_id: Some(2),
            excluded_event_ids: Vec::new(),
            recipient: "Platform operations".to_string(),
            purpose: "api_key=not-for-handoff".to_string(),
            classification: "Restricted".to_string(),
            expires_in_days: Some(7),
        })
        .is_err());
        assert!(workspace_handoff(&WorkspaceHandoffRequest {
            workspace_id: 1,
            session_id: Some(2),
            excluded_event_ids: Vec::new(),
            recipient: "Platform operations".to_string(),
            purpose: "Incident escalation".to_string(),
            classification: "Restricted".to_string(),
            expires_in_days: Some(14),
        })
        .is_err());
        assert!(workspace_handoff(&WorkspaceHandoffRequest {
            workspace_id: 1,
            session_id: Some(2),
            excluded_event_ids: Vec::new(),
            recipient: "Platform operations".to_string(),
            purpose: "Incident escalation".to_string(),
            classification: "Confidential".to_string(),
            expires_in_days: None,
        })
        .is_err());
        assert!(workspace_handoff(&WorkspaceHandoffRequest {
            workspace_id: 1,
            session_id: Some(2),
            excluded_event_ids: Vec::new(),
            recipient: "Platform operations".to_string(),
            purpose: "Incident escalation".to_string(),
            classification: "Confidential".to_string(),
            expires_in_days: Some(7),
        })
        .is_ok());
    }

    #[test]
    fn validates_trusted_handoff_recipient_declarations() {
        assert!(handoff_recipient_trust(&HandoffRecipientTrustRequest {
            recipient: "Platform operations".to_string(),
            max_classification: "Restricted".to_string(),
            note: "Approved for incident handoff review.".to_string(),
        })
        .is_ok());
        assert!(handoff_recipient_trust(&HandoffRecipientTrustRequest {
            recipient: "Platform operations".to_string(),
            max_classification: "Secret".to_string(),
            note: String::new(),
        })
        .is_err());
        assert!(handoff_recipient_trust(&HandoffRecipientTrustRequest {
            recipient: "api_key=not-for-storage".to_string(),
            max_classification: "Internal".to_string(),
            note: String::new(),
        })
        .is_err());
        assert!(handoff_recipient_revoke(&HandoffRecipientRevokeRequest {
            recipient: "Platform operations".to_string(),
        })
        .is_ok());
        assert!(handoff_recipient_revoke(&HandoffRecipientRevokeRequest {
            recipient: String::new(),
        })
        .is_err());
        assert!(handoff_signer_trust(&HandoffSignerTrustRequest {
            signer_fingerprint: "a1b2c3d4e5f60708".to_string(),
            label: "Remote Nginx".to_string(),
        })
        .is_ok());
        assert!(handoff_signer_trust(&HandoffSignerTrustRequest {
            signer_fingerprint: "not-a-fingerprint".to_string(),
            label: String::new(),
        })
        .is_err());
        assert!(handoff_signer_revoke(&HandoffSignerRevokeRequest {
            signer_fingerprint: "a1b2c3d4e5f60708".to_string(),
        })
        .is_ok());
    }
}
