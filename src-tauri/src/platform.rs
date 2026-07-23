use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct PlatformSummary {
    pub sync_mode: String,
    pub sync_status: String,
    pub device_count: i64,
    pub integration_count: i64,
    pub active_plugins: i64,
    pub api_clients: i64,
    pub audit_events: i64,
    pub encryption_status: String,
    pub retention_policy: String,
    pub performance_score: i64,
}

#[derive(Debug, Serialize)]
pub struct SyncDevice {
    pub id: i64,
    pub device_name: String,
    pub platform: String,
    pub sync_mode: String,
    pub status: String,
    pub last_seen_at: String,
}

#[derive(Debug, Deserialize)]
pub struct TeamSharingDeviceRequest {
    pub device_name: String,
    pub platform: String,
    pub sync_mode: String,
}

#[derive(Debug, Deserialize)]
pub struct TeamSharingDeviceStatusRequest {
    pub device_id: i64,
}

#[derive(Debug, Serialize)]
pub struct IntegrationConnector {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub status: String,
    pub capabilities: Vec<String>,
    pub last_activity_at: String,
}

#[derive(Debug, Serialize)]
pub struct PluginRecord {
    pub id: i64,
    pub name: String,
    pub version: String,
    pub status: String,
    pub permissions: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ApiClient {
    pub id: i64,
    pub name: String,
    pub scope: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct AuditLog {
    pub id: i64,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub severity: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct UniversalSyncResult {
    pub devices_checked: i64,
    pub integrations_checked: i64,
    pub events_recorded: i64,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TeamSharingPolicy {
    pub enabled: bool,
    pub mode: String,
    pub allow_workspace_handoffs: bool,
    pub allow_runbook_exports: bool,
    pub allow_imported_references: bool,
    pub require_device_approval: bool,
    pub require_recipient_trust: bool,
    pub retention_days: i64,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TeamSharingReadiness {
    pub ready: bool,
    pub status: String,
    pub mode: String,
    pub approved_devices: i64,
    pub trusted_recipients: i64,
    pub trusted_signers: i64,
    pub allowed_scopes: Vec<String>,
    pub blockers: Vec<String>,
    pub checked_at: String,
}

#[derive(Debug, Serialize)]
pub struct TeamSharingSyncDryRun {
    pub ready: bool,
    pub status: String,
    pub mode: String,
    pub eligible_devices: i64,
    pub eligible_scopes: Vec<String>,
    pub estimated_records: i64,
    pub estimated_bytes: i64,
    pub blockers: Vec<String>,
    pub generated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct TeamSharingManifestInspectionRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct TeamSharingManifestLedgerExportRequest {
    pub filter: String,
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct TeamSharingManifestLedgerChecksum {
    pub filter: String,
    pub event_count: i64,
    pub event_set_sha256: String,
    pub search_applied: bool,
}

#[derive(Debug, Serialize)]
pub struct TeamSharingManifestInspection {
    pub valid: bool,
    pub status: String,
    pub format: String,
    pub schema_version: i64,
    pub remote_sync_enabled: bool,
    pub ready: bool,
    pub mode: String,
    pub estimated_records: i64,
    pub estimated_bytes: i64,
    pub dry_run_sha256: String,
    pub signature_verified: bool,
    pub signer_fingerprint: Option<String>,
    pub signature_status: String,
    pub signer_trusted: bool,
    pub trust_status: String,
    pub blocker_count: i64,
    pub device_count: i64,
    pub failure_reason: Option<String>,
}

impl Default for TeamSharingPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "LocalOnly".to_string(),
            allow_workspace_handoffs: true,
            allow_runbook_exports: true,
            allow_imported_references: false,
            require_device_approval: true,
            require_recipient_trust: true,
            retention_days: 30,
            updated_at: String::new(),
        }
    }
}

pub fn performance_score(devices: i64, integrations: i64, plugins: i64, api_clients: i64) -> i64 {
    ((devices * 12) + (integrations * 5) + (plugins * 10) + (api_clients * 8)).clamp(0, 100)
}

pub fn sharing_allowed_scopes(policy: &TeamSharingPolicy) -> Vec<String> {
    let mut scopes = Vec::new();
    if policy.allow_workspace_handoffs {
        scopes.push("Workspace handoffs".to_string());
    }
    if policy.allow_runbook_exports {
        scopes.push("Runbook exports".to_string());
    }
    if policy.allow_imported_references {
        scopes.push("Imported references".to_string());
    }
    scopes
}

pub fn sharing_blockers(
    policy: &TeamSharingPolicy,
    approved_devices: i64,
    trusted_recipients: i64,
    allowed_scopes: &[String],
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !policy.enabled {
        blockers.push("Team sharing policy is disabled.".to_string());
    }
    if allowed_scopes.is_empty() {
        blockers.push("No sharing scopes are enabled.".to_string());
    }
    if policy.require_device_approval && approved_devices == 0 {
        blockers.push("No approved sharing device is registered.".to_string());
    }
    if policy.require_recipient_trust && trusted_recipients == 0 {
        blockers.push("No active trusted handoff recipient is registered.".to_string());
    }
    blockers
}

#[cfg(test)]
mod tests {
    use super::{performance_score, sharing_allowed_scopes, sharing_blockers, TeamSharingPolicy};

    #[test]
    fn performance_score_is_capped() {
        assert_eq!(performance_score(20, 20, 20, 20), 100);
    }

    #[test]
    fn sharing_readiness_requires_policy_scope_device_and_recipient() {
        let mut policy = TeamSharingPolicy {
            enabled: true,
            allow_workspace_handoffs: false,
            allow_runbook_exports: false,
            ..TeamSharingPolicy::default()
        };
        let scopes = sharing_allowed_scopes(&policy);
        assert!(sharing_blockers(&policy, 0, 0, &scopes)
            .iter()
            .any(|blocker| blocker.contains("No sharing scopes")));

        policy.allow_workspace_handoffs = true;
        let scopes = sharing_allowed_scopes(&policy);
        let blockers = sharing_blockers(&policy, 0, 0, &scopes);
        assert_eq!(blockers.len(), 2);
        assert!(sharing_blockers(&policy, 1, 1, &scopes).is_empty());
    }
}
