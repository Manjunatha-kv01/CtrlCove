import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  AgentWorkflowRecord,
  ApiClient,
  AutomationTask,
  ClipboardItem,
  ClipboardStats,
  CognitiveModule,
  CognitiveOverview,
  CognitiveUseCase,
  CognitiveWorkspace,
  DatabaseReliabilityStatus,
  Collection,
  EnterpriseControl,
  IntelligenceReport,
  IntegrationConnector,
  InsightIncident,
  InsightTrailEvent,
  InsightTrailOverview,
  InsightTrailSettings,
  KnowledgeDigest,
  KnowledgeGraph,
  KnowledgeHealth,
  PlatformSummary,
  PluginRecord,
  PrivacyStatus,
  SearchRequest,
  SmartNotification,
  SyncDevice,
  TeamSharingPolicy,
  TeamSharingReadiness,
  AuditLog,
  VaultRetentionSettings,
  WorkspaceSnapshot
} from "../types/cymos";

const emptyGraph: KnowledgeGraph = { nodes: [], edges: [], clusters: [], recommendations: [] };

const defaultStats: ClipboardStats = {
  total_items: 0,
  text_items: 0,
  image_items: 0,
  code_items: 0,
  url_items: 0,
  file_items: 0,
  storage_used: 0,
  favorite_items: 0,
  most_used_application: "Unknown"
};

const defaultHealth: KnowledgeHealth = {
  total_memories: 0,
  connected_entities: 0,
  graph_relationships: 0,
  active_projects: 0,
  ai_activity: 0,
  background_tasks: 0,
  unread_notifications: 0,
  storage_bytes: 0,
  storage_health: "Unknown",
  productivity_score: 0
};

const defaultPlatform: PlatformSummary = {
  sync_mode: "Local-first",
  sync_status: "Local Only",
  device_count: 0,
  integration_count: 0,
  active_plugins: 0,
  api_clients: 0,
  audit_events: 0,
  encryption_status: "Unknown",
  retention_policy: "User controlled",
  performance_score: 0
};

const defaultTeamSharingPolicy: TeamSharingPolicy = {
  enabled: false,
  mode: "LocalOnly",
  allow_workspace_handoffs: true,
  allow_runbook_exports: true,
  allow_imported_references: false,
  require_device_approval: true,
  require_recipient_trust: true,
  retention_days: 30,
  updated_at: ""
};

const defaultTeamSharingReadiness: TeamSharingReadiness = {
  ready: false,
  status: "Disabled",
  mode: "LocalOnly",
  approved_devices: 0,
  trusted_recipients: 0,
  trusted_signers: 0,
  allowed_scopes: [],
  blockers: ["Team sharing policy is disabled."],
  checked_at: ""
};

const defaultCognitiveOverview: CognitiveOverview = {
  release: "v1.0 Personal Cognitive Operating System",
  tagline: "Remember Everything. Understand Everything. Accomplish Anything.",
  readiness_status: "Foundation in progress",
  privacy_mode: "Local-first",
  memory_score: 0,
  module_count: 0,
  enterprise_controls: 0,
  use_case_count: 0
};

const defaultInsightTrailOverview: InsightTrailOverview = {
  event_count: 0,
  active_incident_count: 0,
  screenshot_count: 0,
  error_signal_count: 0,
  capture_state: "Active",
  retention_days: 30
};

const defaultInsightTrailSettings: InsightTrailSettings = {
  enabled: true,
  capture_clipboard: true,
  capture_terminal_history: true,
  capture_copied_images: true,
  create_incidents: true,
  retention_days: 30,
  max_storage_mb: 512,
  excluded_applications: [],
  updated_at: ""
};

const defaultVaultRetentionSettings: VaultRetentionSettings = {
  retention_days: 365,
  max_items: 10_000,
  max_storage_mb: 1_024,
  preserve_favorites: true,
  updated_at: ""
};

const defaultCognitiveWorkspace: CognitiveWorkspace = {
  id: 1,
  name: "Local operations",
  project: "Personal memory",
  status: "Active",
  is_imported: false,
  created_at: "",
  updated_at: "",
  last_event_at: null,
  event_count: 0,
  memory_count: 0,
  error_count: 0,
  sources: [],
  top_topics: [],
  summary: "Personal memory is ready for its first captured work session.",
  next_signal: "Start a session note to give this workspace a durable point of reference."
};

const defaultWorkspaceSnapshot: WorkspaceSnapshot = {
  workspace: defaultCognitiveWorkspace,
  sessions: [],
  active_session: null,
  events: [],
  incidents: [],
  resolutions: [],
  import_provenance: null
};

const defaultDatabaseReliability: DatabaseReliabilityStatus = {
  integrity_status: "Checking",
  foreign_key_issues: 0,
  journal_mode: "WAL",
  database_bytes: 0,
  migration_count: 0,
  backup_count: 0,
  last_backup: null
};

const defaultPrivacyStatus: PrivacyStatus = {
  settings: {
    protection_enabled: true,
    capture_text: true,
    capture_images: true,
    block_sensitive_text: true,
    updated_at: ""
  },
  blocked_capture_count: 0,
  last_blocked_at: null
};

export function useCymosWorkspace(request: SearchRequest) {
  const [items, setItems] = useState<ClipboardItem[]>([]);
  const [stats, setStats] = useState<ClipboardStats>(defaultStats);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [graph, setGraph] = useState<KnowledgeGraph>(emptyGraph);
  const [dailySummary, setDailySummary] = useState<KnowledgeDigest | null>(null);
  const [weeklyReport, setWeeklyReport] = useState<KnowledgeDigest | null>(null);
  const [agentHistory, setAgentHistory] = useState<AgentWorkflowRecord[]>([]);
  const [health, setHealth] = useState<KnowledgeHealth>(defaultHealth);
  const [automationTasks, setAutomationTasks] = useState<AutomationTask[]>([]);
  const [notifications, setNotifications] = useState<SmartNotification[]>([]);
  const [reports, setReports] = useState<IntelligenceReport[]>([]);
  const [platform, setPlatform] = useState<PlatformSummary>(defaultPlatform);
  const [teamSharingPolicy, setTeamSharingPolicy] = useState<TeamSharingPolicy>(defaultTeamSharingPolicy);
  const [teamSharingReadiness, setTeamSharingReadiness] = useState<TeamSharingReadiness>(defaultTeamSharingReadiness);
  const [devices, setDevices] = useState<SyncDevice[]>([]);
  const [connectors, setConnectors] = useState<IntegrationConnector[]>([]);
  const [plugins, setPlugins] = useState<PluginRecord[]>([]);
  const [apiClients, setApiClients] = useState<ApiClient[]>([]);
  const [auditLogs, setAuditLogs] = useState<AuditLog[]>([]);
  const [teamSharingAuditLogs, setTeamSharingAuditLogs] = useState<AuditLog[]>([]);
  const [manifestLedgerAuditLogs, setManifestLedgerAuditLogs] = useState<AuditLog[]>([]);
  const [cognitiveOverview, setCognitiveOverview] = useState<CognitiveOverview>(defaultCognitiveOverview);
  const [cognitiveModules, setCognitiveModules] = useState<CognitiveModule[]>([]);
  const [enterpriseControls, setEnterpriseControls] = useState<EnterpriseControl[]>([]);
  const [cognitiveUseCases, setCognitiveUseCases] = useState<CognitiveUseCase[]>([]);
  const [insightTrailOverview, setInsightTrailOverview] = useState<InsightTrailOverview>(defaultInsightTrailOverview);
  const [insightTrailSettings, setInsightTrailSettings] = useState<InsightTrailSettings>(defaultInsightTrailSettings);
  const [insightTrailEvents, setInsightTrailEvents] = useState<InsightTrailEvent[]>([]);
  const [insightIncidents, setInsightIncidents] = useState<InsightIncident[]>([]);
  const [cognitiveWorkspace, setCognitiveWorkspace] = useState<CognitiveWorkspace>(defaultCognitiveWorkspace);
  const [cognitiveWorkspaces, setCognitiveWorkspaces] = useState<CognitiveWorkspace[]>([defaultCognitiveWorkspace]);
  const [workspaceSnapshot, setWorkspaceSnapshot] = useState<WorkspaceSnapshot>(defaultWorkspaceSnapshot);
  const [databaseReliability, setDatabaseReliability] = useState<DatabaseReliabilityStatus>(defaultDatabaseReliability);
  const [privacyStatus, setPrivacyStatus] = useState<PrivacyStatus>(defaultPrivacyStatus);
  const [vaultRetentionSettings, setVaultRetentionSettings] = useState<VaultRetentionSettings>(defaultVaultRetentionSettings);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const memoryRequestVersion = useRef(0);

  const loadMemory = useCallback(async () => {
    const requestVersion = ++memoryRequestVersion.current;

    // Vite previews run outside the Tauri WebView, where command invocation is unavailable.
    if (!("__TAURI_INTERNALS__" in window)) {
      setLoading(false);
      setError(null);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const nextItems = await invoke<ClipboardItem[]>("search_clipboard_items", { request });
      if (requestVersion === memoryRequestVersion.current) {
        setItems(nextItems);
      }
    } catch (cause) {
      if (requestVersion === memoryRequestVersion.current) {
        setError(cause instanceof Error ? cause.message : String(cause));
      }
    } finally {
      if (requestVersion === memoryRequestVersion.current) {
        setLoading(false);
      }
    }
  }, [request]);

  const loadDashboard = useCallback(async () => {
    if (!("__TAURI_INTERNALS__" in window)) {
      return;
    }

    try {
      const [
        nextStats,
        nextCollections,
        nextGraph,
        nextDaily,
        nextWeekly,
        nextAgentHistory,
        nextHealth,
        nextTasks,
        nextNotifications,
        nextReports,
        nextPlatform,
        nextTeamSharingPolicy,
        nextTeamSharingReadiness,
        nextDevices,
        nextConnectors,
        nextPlugins,
        nextApiClients,
        nextAuditLogs,
        nextTeamSharingAuditLogs,
        nextManifestLedgerAuditLogs,
        nextCognitiveOverview,
        nextCognitiveModules,
        nextEnterpriseControls,
        nextCognitiveUseCases,
        nextInsightTrailOverview,
        nextInsightTrailSettings,
        nextInsightTrailEvents,
        nextInsightIncidents,
        nextCognitiveWorkspace,
        nextCognitiveWorkspaces,
        nextWorkspaceSnapshot,
        nextDatabaseReliability,
        nextPrivacyStatus,
        nextVaultRetentionSettings
      ] = await Promise.all([
        invoke<ClipboardStats>("get_clipboard_stats"),
        invoke<Collection[]>("get_collections"),
        invoke<KnowledgeGraph>("get_knowledge_graph"),
        invoke<KnowledgeDigest>("get_daily_knowledge_summary"),
        invoke<KnowledgeDigest>("get_weekly_learning_report"),
        invoke<AgentWorkflowRecord[]>("get_agent_workflows"),
        invoke<KnowledgeHealth>("get_knowledge_health"),
        invoke<AutomationTask[]>("get_automation_tasks"),
        invoke<SmartNotification[]>("get_smart_notifications"),
        invoke<IntelligenceReport[]>("get_intelligence_reports"),
        invoke<PlatformSummary>("get_platform_summary"),
        invoke<TeamSharingPolicy>("get_team_sharing_policy"),
        invoke<TeamSharingReadiness>("get_team_sharing_readiness"),
        invoke<SyncDevice[]>("get_sync_devices"),
        invoke<IntegrationConnector[]>("get_integration_connectors"),
        invoke<PluginRecord[]>("get_plugin_records"),
        invoke<ApiClient[]>("get_api_clients"),
        invoke<AuditLog[]>("get_audit_logs"),
        invoke<AuditLog[]>("get_team_sharing_audit_logs"),
        invoke<AuditLog[]>("get_team_sharing_manifest_ledger_audit_logs"),
        invoke<CognitiveOverview>("get_cognitive_overview"),
        invoke<CognitiveModule[]>("get_cognitive_modules"),
        invoke<EnterpriseControl[]>("get_enterprise_controls"),
        invoke<CognitiveUseCase[]>("get_cognitive_use_cases"),
        invoke<InsightTrailOverview>("get_insight_trail_overview"),
        invoke<InsightTrailSettings>("get_insight_trail_settings"),
        invoke<InsightTrailEvent[]>("get_insight_trail_events", {
          request: { query: "", event_type: "All", limit: 100 }
        }),
        invoke<InsightIncident[]>("get_insight_incidents"),
        invoke<CognitiveWorkspace>("get_cognitive_workspace"),
        invoke<CognitiveWorkspace[]>("get_cognitive_workspaces"),
        invoke<WorkspaceSnapshot>("get_workspace_snapshot", { workspaceId: null }),
        invoke<DatabaseReliabilityStatus>("get_database_reliability"),
        invoke<PrivacyStatus>("get_privacy_status"),
        invoke<VaultRetentionSettings>("get_vault_retention_settings")
      ]);

      setStats(nextStats);
      setCollections(nextCollections);
      setGraph(nextGraph);
      setDailySummary(nextDaily);
      setWeeklyReport(nextWeekly);
      setAgentHistory(nextAgentHistory);
      setHealth(nextHealth);
      setAutomationTasks(nextTasks);
      setNotifications(nextNotifications);
      setReports(nextReports);
      setPlatform(nextPlatform);
      setTeamSharingPolicy(nextTeamSharingPolicy);
      setTeamSharingReadiness(nextTeamSharingReadiness);
      setDevices(nextDevices);
      setConnectors(nextConnectors);
      setPlugins(nextPlugins);
      setApiClients(nextApiClients);
      setAuditLogs(nextAuditLogs);
      setTeamSharingAuditLogs(nextTeamSharingAuditLogs);
      setManifestLedgerAuditLogs(nextManifestLedgerAuditLogs);
      setCognitiveOverview(nextCognitiveOverview);
      setCognitiveModules(nextCognitiveModules);
      setEnterpriseControls(nextEnterpriseControls);
      setCognitiveUseCases(nextCognitiveUseCases);
      setInsightTrailOverview(nextInsightTrailOverview);
      setInsightTrailSettings(nextInsightTrailSettings);
      setInsightTrailEvents(nextInsightTrailEvents);
      setInsightIncidents(nextInsightIncidents);
      setCognitiveWorkspace(nextCognitiveWorkspace);
      setCognitiveWorkspaces(nextCognitiveWorkspaces);
      setWorkspaceSnapshot(nextWorkspaceSnapshot);
      setDatabaseReliability(nextDatabaseReliability);
      setPrivacyStatus(nextPrivacyStatus);
      setVaultRetentionSettings(nextVaultRetentionSettings);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  const refresh = useCallback(async () => {
    await Promise.all([loadMemory(), loadDashboard()]);
  }, [loadDashboard, loadMemory]);

  const refreshCognitiveWorkspaces = useCallback(async () => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const nextWorkspaces = await invoke<CognitiveWorkspace[]>("get_cognitive_workspaces");
    setCognitiveWorkspaces(nextWorkspaces);
  }, []);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) {
      return;
    }

    void loadMemory();
  }, [loadMemory]);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) {
      return;
    }

    void loadDashboard();
  }, [loadDashboard]);

  useEffect(() => {
    let stopClipboard: (() => void) | undefined;
    let stopAutomation: (() => void) | undefined;
    let stopPrivacy: (() => void) | undefined;

    void listen("clipboard-item-created", () => void refresh()).then((cleanup) => {
      stopClipboard = cleanup;
    });
    void listen("autonomous-cycle-complete", () => void refresh()).then((cleanup) => {
      stopAutomation = cleanup;
    });
    void listen("privacy-capture-blocked", () => void refresh()).then((cleanup) => {
      stopPrivacy = cleanup;
    });

    return () => {
      stopClipboard?.();
      stopAutomation?.();
      stopPrivacy?.();
    };
  }, [refresh]);

  const tags = useMemo(() => Array.from(new Set(items.flatMap((item) => item.tags))).sort(), [items]);
  const categories = useMemo(() => Array.from(new Set(items.map((item) => item.category))).sort(), [items]);

  return {
    items,
    stats,
    collections,
    graph,
    dailySummary,
    weeklyReport,
    agentHistory,
    health,
    automationTasks,
    notifications,
    reports,
    platform,
    teamSharingPolicy,
    teamSharingReadiness,
    devices,
    connectors,
    plugins,
    apiClients,
    auditLogs,
    teamSharingAuditLogs,
    manifestLedgerAuditLogs,
    cognitiveOverview,
    cognitiveModules,
    enterpriseControls,
    cognitiveUseCases,
    insightTrailOverview,
    insightTrailSettings,
    insightTrailEvents,
    insightIncidents,
    cognitiveWorkspace,
    cognitiveWorkspaces,
    workspaceSnapshot,
    databaseReliability,
    privacyStatus,
    vaultRetentionSettings,
    tags,
    categories,
    loading,
    error,
    refresh,
    refreshCognitiveWorkspaces
  };
}
