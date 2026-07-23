export type ClipboardType =
  | "Text"
  | "URL"
  | "Code"
  | "Image"
  | "File"
  | "Folder"
  | "Color"
  | "Table"
  | "HTML";

export type ClipboardItem = {
  id: number;
  content: string;
  content_type: ClipboardType;
  source_application: string;
  created_at: string;
  updated_at: string;
  content_hash: string;
  character_count: number;
  word_count: number;
  file_size: number | null;
  image_width: number | null;
  image_height: number | null;
  language: string | null;
  is_favorite: boolean;
  collection_id: number | null;
  collection_name: string | null;
  collection_color: string | null;
  ai_summary: string;
  category: string;
  keywords: string[];
  reading_time_minutes: number;
  copy_count: number;
  last_copied_at: string;
  semantic_score: number;
  rank_reason: string;
  embedding_source: string;
  operational_context: OperationalContext;
  tags: string[];
};

export type BrowserBookmarkRequest = {
  url: string;
  title: string;
  tags: string[];
};

export type IdeSnippetRequest = {
  content: string;
  title: string;
  language: string;
  project: string;
  file_path: string;
  tags: string[];
};

export type TerminalCommandRequest = {
  command: string;
  shell: "Bash" | "Zsh";
  host: string;
  project: string;
  tags: string[];
};

export type OperationalContext = {
  kind: string;
  environment: string;
  shell: string | null;
  hostnames: string[];
  ip_addresses: string[];
  services: string[];
  technologies: string[];
};

export type Collection = {
  id: number;
  name: string;
  color: string;
  created_at: string;
};

export type ClipboardStats = {
  total_items: number;
  text_items: number;
  image_items: number;
  code_items: number;
  url_items: number;
  file_items: number;
  storage_used: number;
  favorite_items: number;
  most_used_application: string;
};

export type InsightTrailSettings = {
  enabled: boolean;
  capture_clipboard: boolean;
  capture_terminal_history: boolean;
  capture_copied_images: boolean;
  create_incidents: boolean;
  retention_days: number;
  max_storage_mb: number;
  excluded_applications: string[];
  updated_at: string;
};

export type InsightTrailEvent = {
  id: number;
  event_type: "Clipboard" | "Terminal" | "Screenshot" | "Error" | "Note";
  title: string;
  details: string;
  source_application: string;
  severity: "Info" | "Warning" | "Critical";
  created_at: string;
  memory_id: number | null;
  screenshot_path: string | null;
  incident_id: number | null;
  session_id: number | null;
  tags: string[];
};

export type InsightIncident = {
  id: number;
  title: string;
  status: "Open" | "Resolved";
  summary: string;
  first_seen_at: string;
  last_seen_at: string;
  event_count: number;
  recommended_steps: string[];
};

export type InsightTrailOverview = {
  event_count: number;
  active_incident_count: number;
  screenshot_count: number;
  error_signal_count: number;
  capture_state: "Active" | "Paused";
  retention_days: number;
};

export type InsightTrailNoteRequest = {
  title: string;
  details: string;
  tags: string[];
};

export type CognitiveWorkspace = {
  id: number;
  name: string;
  project: string;
  status: string;
  is_imported: boolean;
  created_at: string;
  updated_at: string;
  last_event_at: string | null;
  event_count: number;
  memory_count: number;
  error_count: number;
  sources: string[];
  top_topics: string[];
  summary: string;
  next_signal: string;
};

export type WorkspaceContextUpdate = {
  name: string;
  project: string;
};

export type WorkspaceCreateRequest = {
  name: string;
  project: string;
};

export type WorkspaceSession = {
  id: number;
  workspace_id: number;
  title: string;
  status: "Active" | "Completed";
  started_at: string;
  ended_at: string | null;
  event_count: number;
};

export type WorkspaceSessionStartRequest = {
  workspace_id: number;
  title: string;
};

export type IncidentResolution = {
  id: number;
  incident_id: number;
  workspace_id: number;
  workspace_name: string;
  session_id: number | null;
  title: string;
  details: string;
  created_at: string;
};

export type IncidentResolutionRequest = {
  workspace_id: number;
  incident_id: number;
  title: string;
  details: string;
};

export type IncidentReopenRequest = {
  workspace_id: number;
  incident_id: number;
  reason: string;
};

export type IncidentEvidenceLinkRequest = {
  workspace_id: number;
  incident_id: number;
  event_id: number;
};

export type WorkspaceReportRequest = {
  workspace_id: number;
  session_id: number | null;
};

export type WorkspaceHandoffRequest = {
  workspace_id: number;
  session_id: number | null;
  excluded_event_ids: number[];
  recipient: string;
  purpose: string;
  classification: "Internal" | "Restricted" | "Confidential";
  expires_in_days: number | null;
};

export type HandoffRecipientTrustRequest = {
  recipient: string;
  max_classification: "Internal" | "Restricted" | "Confidential";
  note: string;
};

export type HandoffRecipientTrustRecord = HandoffRecipientTrustRequest & {
  id: number;
  is_active: boolean;
  export_count: number;
  last_used_at: string | null;
  revoked_at: string | null;
  created_at: string;
};

export type HandoffSignerTrustRequest = {
  signer_fingerprint: string;
  label: string;
};

export type HandoffSignerTrustRecord = HandoffSignerTrustRequest & {
  id: number;
  is_active: boolean;
  import_count: number;
  last_used_at: string | null;
  revoked_at: string | null;
  created_at: string;
};

export type WorkspaceDocumentImportRequest = {
  workspace_id: number;
  file_name: string;
  content: string;
};

export type WorkspaceDocumentImportResult = {
  stored: boolean;
  snapshot: WorkspaceSnapshot;
};

export type LocalExport = {
  path: string;
};

export type HandoffInspection = {
  workspace_name: string;
  project: string;
  scope: string;
  recipient: string;
  purpose: string;
  classification: string;
  expires_at_unix: number | null;
  is_expired: boolean;
  generated_locally_at: string;
  event_count: number;
  incident_count: number;
  resolution_count: number;
  checksum: string;
  signature_verified: boolean;
  signer_fingerprint: string | null;
  signature_status: string;
};

export type WorkspaceHandoffInspectionRecord = {
  id: number;
  status: "Verified" | "Expired" | "Rejected";
  workspace_name: string | null;
  classification: string | null;
  signer_fingerprint: string | null;
  package_sha256: string;
  payload_sha256: string | null;
  failure_reason: string | null;
  package_bytes: number;
  inspected_at: string;
};

export type WorkspaceHandoffReadiness = {
  safe: boolean;
  scope: string;
  event_count: number;
  excluded_event_count: number;
  incident_count: number;
  resolution_count: number;
  estimated_bytes: number;
  blocking_findings: number;
  blockers: string[];
};

export type WorkspaceHandoffExportRecord = {
  id: number;
  workspace_id: number;
  session_id: number | null;
  scope: string;
  recipient: string;
  purpose: string;
  classification: string;
  expires_at_unix: number | null;
  signer_fingerprint: string;
  package_sha256: string;
  package_bytes: number;
  event_count: number;
  excluded_event_count: number;
  incident_count: number;
  resolution_count: number;
  created_at: string;
};

export type WorkspaceHandoffImportRequest = {
  content: string;
};

export type RunbookEntry = {
  id: number;
  incident_id: number | null;
  incident_title: string;
  workspace_name: string;
  title: string;
  details: string;
  tags: string[];
  created_at: string;
  latest_revision: number;
  last_reviewed_revision: number | null;
  last_reviewed_at: string | null;
  last_review_note: string | null;
  review_status: "Incident evidence" | "Needs review" | "Review due" | "Reviewed";
};

export type RunbookSearchRequest = {
  query: string;
  review_status: "All" | "Needs review" | "Review due" | "Reviewed";
};

export type ManualRunbookRequest = {
  title: string;
  details: string;
  tags: string[];
};

export type ManualRunbookUpdateRequest = ManualRunbookRequest & {
  id: number;
};

export type RunbookRevision = {
  id: number;
  runbook_id: number;
  revision: number;
  title: string;
  details: string;
  tags: string[];
  created_at: string;
};

export type ManualRunbookRevisionRestoreRequest = {
  entry_id: number;
  revision_id: number;
};

export type ManualRunbookReviewRequest = {
  entry_id: number;
  note: string;
};

export type WorkspaceImportProvenance = {
  source_workspace: string;
  source_project: string;
  source_scope: string;
  source_recipient: string;
  source_purpose: string;
  source_classification: string;
  source_expires_at_unix: number | null;
  source_signer_fingerprint: string | null;
  source_generated_at: string;
  checksum: string;
  imported_at: string;
};

export type WorkspaceSnapshot = {
  workspace: CognitiveWorkspace;
  sessions: WorkspaceSession[];
  active_session: WorkspaceSession | null;
  events: InsightTrailEvent[];
  incidents: InsightIncident[];
  resolutions: IncidentResolution[];
  import_provenance: WorkspaceImportProvenance | null;
};

export type GraphNode = {
  id: number;
  name: string;
  entity_type: string;
  weight: number;
  cluster: string;
};

export type GraphEdge = {
  source: number;
  target: number;
  relationship: string;
  weight: number;
};

export type TopicCluster = {
  name: string;
  count: number;
  entities: string[];
};

export type KnowledgeGraph = {
  nodes: GraphNode[];
  edges: GraphEdge[];
  clusters: TopicCluster[];
  recommendations: string[];
};

export type AssistantSource = {
  id: number;
  title: string;
  content_type: string;
  category: string;
  created_at: string;
  score: number;
};

export type AssistantResponse = {
  answer: string;
  sources: AssistantSource[];
  related_topics: string[];
  retrieval_summary: string;
  model: string;
};

export type KnowledgeDigest = {
  title: string;
  bullets: string[];
  active_topics: string[];
  recommendations: string[];
};

export type AgentStep = {
  agent: string;
  action: string;
  output: string;
};

export type AgentLog = {
  agent: string;
  message: string;
};

export type AgentWorkflow = {
  id: number | null;
  goal: string;
  status: string;
  agents: string[];
  plan: AgentStep[];
  answer: string;
  recommendations: string[];
  logs: AgentLog[];
  context_memory_ids: number[];
};

export type AgentWorkflowRecord = {
  id: number;
  goal: string;
  status: string;
  agents: string[];
  answer: string;
  recommendations: string[];
  created_at: string;
};

export type AutomationTask = {
  id: number;
  service: string;
  status: string;
  details: string;
  created_at: string;
};

export type SmartNotification = {
  id: number;
  message: string;
  severity: string;
  is_read: boolean;
  created_at: string;
};

export type IntelligenceReport = {
  id: number;
  report_type: string;
  title: string;
  summary: string;
  bullets: string[];
  created_at: string;
};

export type KnowledgeHealth = {
  total_memories: number;
  connected_entities: number;
  graph_relationships: number;
  active_projects: number;
  ai_activity: number;
  background_tasks: number;
  unread_notifications: number;
  storage_bytes: number;
  storage_health: string;
  productivity_score: number;
};

export type AutomationRunResult = {
  tasks_run: number;
  reports_created: number;
  notifications_created: number;
  backup_path: string;
};

export type DatabaseReliabilityStatus = {
  integrity_status: string;
  foreign_key_issues: number;
  journal_mode: string;
  database_bytes: number;
  migration_count: number;
  backup_count: number;
  last_backup: string | null;
};

export type DatabaseReliabilityChecksum = {
  integrity_status: string;
  snapshot_count: number;
  report_data_sha256: string;
};

export type DatabaseReliabilityReportExport = DatabaseReliabilityChecksum & {
  path: string;
};

export type DatabaseReliabilityReportSnapshot = {
  path: string;
  file_name: string;
  bytes: number;
  modified_at_unix: number;
};

export type DatabaseBackup = {
  path: string;
  verified: boolean;
  backup_count: number;
};

export type DatabaseBackupSnapshot = {
  path: string;
  file_name: string;
  bytes: number;
  modified_at_unix: number;
};

export type DatabaseBackupVerificationRequest = {
  file_name: string;
};

export type PrivacySettings = {
  protection_enabled: boolean;
  capture_text: boolean;
  capture_images: boolean;
  block_sensitive_text: boolean;
  updated_at: string;
};

export type PrivacyStatus = {
  settings: PrivacySettings;
  blocked_capture_count: number;
  last_blocked_at: string | null;
};

export type VaultRetentionSettings = {
  retention_days: number;
  max_items: number;
  max_storage_mb: number;
  preserve_favorites: boolean;
  updated_at: string;
};

export type VaultRetentionResult = {
  removed_items: number;
  removed_images: number;
  remaining_items: number;
  remaining_storage_bytes: number;
  protected_favorites: number;
  limits_met: boolean;
};

export type PlatformSummary = {
  sync_mode: string;
  sync_status: string;
  device_count: number;
  integration_count: number;
  active_plugins: number;
  api_clients: number;
  audit_events: number;
  encryption_status: string;
  retention_policy: string;
  performance_score: number;
};

export type SyncDevice = {
  id: number;
  device_name: string;
  platform: string;
  sync_mode: string;
  status: string;
  last_seen_at: string;
};

export type TeamSharingDeviceRequest = {
  device_name: string;
  platform: string;
  sync_mode: "Local-only" | "Self-hosted" | "Encrypted cloud";
};

export type TeamSharingDeviceStatusRequest = {
  device_id: number;
};

export type IntegrationConnector = {
  id: number;
  name: string;
  category: string;
  status: string;
  capabilities: string[];
  last_activity_at: string;
};

export type PluginRecord = {
  id: number;
  name: string;
  version: string;
  status: string;
  permissions: string;
  created_at: string;
};

export type ApiClient = {
  id: number;
  name: string;
  scope: string;
  status: string;
  created_at: string;
};

export type AuditLog = {
  id: number;
  actor: string;
  action: string;
  resource: string;
  severity: string;
  created_at: string;
};

export type UniversalSyncResult = {
  devices_checked: number;
  integrations_checked: number;
  events_recorded: number;
  status: string;
};

export type TeamSharingPolicy = {
  enabled: boolean;
  mode: "LocalOnly" | "SelfHosted" | "EncryptedCloud";
  allow_workspace_handoffs: boolean;
  allow_runbook_exports: boolean;
  allow_imported_references: boolean;
  require_device_approval: boolean;
  require_recipient_trust: boolean;
  retention_days: number;
  updated_at: string;
};

export type TeamSharingReadiness = {
  ready: boolean;
  status: "Ready" | "Blocked" | "Disabled";
  mode: string;
  approved_devices: number;
  trusted_recipients: number;
  trusted_signers: number;
  allowed_scopes: string[];
  blockers: string[];
  checked_at: string;
};

export type TeamSharingSyncDryRun = {
  ready: boolean;
  status: "Ready" | "Blocked";
  mode: string;
  eligible_devices: number;
  eligible_scopes: string[];
  estimated_records: number;
  estimated_bytes: number;
  blockers: string[];
  generated_at: string;
};

export type TeamSharingManifestInspectionRequest = {
  content: string;
};

export type TeamSharingManifestLedgerExportRequest = {
  filter: "All" | "Verified" | "Warnings" | "Exports" | "FilteredExports";
  query: string;
};

export type TeamSharingManifestLedgerChecksum = {
  filter: string;
  event_count: number;
  event_set_sha256: string;
  search_applied: boolean;
};

export type TeamSharingManifestInspection = {
  valid: boolean;
  status: "Verified" | "Rejected";
  format: string;
  schema_version: number;
  remote_sync_enabled: boolean;
  ready: boolean;
  mode: string;
  estimated_records: number;
  estimated_bytes: number;
  dry_run_sha256: string;
  signature_verified: boolean;
  signer_fingerprint: string | null;
  signature_status: string;
  signer_trusted: boolean;
  trust_status: string;
  blocker_count: number;
  device_count: number;
  failure_reason: string | null;
};

export type TerminalHistoryImportResult = {
  shell: string;
  available: number;
  selected: number;
  imported: number;
  skipped_sensitive: number;
  skipped_irrelevant: number;
};

export type CognitiveOverview = {
  release: string;
  tagline: string;
  readiness_status: string;
  privacy_mode: string;
  memory_score: number;
  module_count: number;
  enterprise_controls: number;
  use_case_count: number;
};

export type CognitiveModule = {
  id: number;
  name: string;
  layer: string;
  status: string;
  capabilities: string[];
  updated_at: string;
};

export type EnterpriseControl = {
  id: number;
  name: string;
  status: string;
  scope: string;
  updated_at: string;
};

export type CognitiveUseCase = {
  id: number;
  audience: string;
  workflow: string;
  status: string;
};

export type CognitiveReleaseResult = {
  modules_verified: number;
  controls_verified: number;
  use_cases_verified: number;
  status: string;
};

export type SearchRequest = {
  query: string;
  content_type: string;
  favorite_only: boolean;
  collection_id: number | null;
  tag: string;
  category: string;
  semantic: boolean;
};
