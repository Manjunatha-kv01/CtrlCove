import { invoke } from "@tauri-apps/api/core";
import { useDeferredValue, useEffect, useMemo, useState } from "react";
import { AppShell, type WorkspaceView } from "../components/AppShell";
import { InsightTrailView } from "../components/InsightTrailView";
import { CognitiveWorkspaceView } from "../components/CognitiveWorkspaceView";
import {
  AgentView,
  AssistantView,
  AutomationView,
  GraphView,
  InsightsView,
  MemoryView,
  OperationsView,
  OverviewView,
  PlatformView
} from "../components/WorkspaceViews";
import { useCymosWorkspace } from "../hooks/useCymosWorkspace";
import type {
  AgentWorkflow,
  AssistantResponse,
  AuditLog,
  AutomationRunResult,
  BrowserBookmarkRequest,
  IdeSnippetRequest,
  IncidentEvidenceLinkRequest,
  ClipboardItem,
  CognitiveReleaseResult,
  DatabaseBackup,
  DatabaseBackupSnapshot,
  DatabaseBackupVerificationRequest,
  DatabaseReliabilityChecksum,
  DatabaseReliabilityReportExport,
  DatabaseReliabilityReportSnapshot,
  DatabaseReliabilityStatus,
  HandoffRecipientTrustRecord,
  HandoffRecipientTrustRequest,
  HandoffSignerTrustRecord,
  HandoffSignerTrustRequest,
  HandoffInspection,
  WorkspaceHandoffReadiness,
  WorkspaceHandoffExportRecord,
  IncidentReopenRequest,
  IncidentResolutionRequest,
  InsightTrailNoteRequest,
  InsightTrailSettings,
  LocalExport,
  ManualRunbookRequest,
  ManualRunbookReviewRequest,
  ManualRunbookRevisionRestoreRequest,
  ManualRunbookUpdateRequest,
  PrivacySettings,
  RunbookEntry,
  RunbookRevision,
  RunbookSearchRequest,
  WorkspaceCreateRequest,
  WorkspaceContextUpdate,
  WorkspaceDocumentImportRequest,
  WorkspaceDocumentImportResult,
  WorkspaceHandoffImportRequest,
  WorkspaceHandoffInspectionRecord,
  WorkspaceHandoffRequest,
  WorkspaceReportRequest,
  WorkspaceSessionStartRequest,
  WorkspaceSnapshot,
  TerminalHistoryImportResult,
  TerminalCommandRequest,
  TeamSharingDeviceRequest,
  TeamSharingDeviceStatusRequest,
  TeamSharingManifestInspection,
  TeamSharingManifestLedgerChecksum,
  TeamSharingManifestInspectionRequest,
  TeamSharingManifestLedgerExportRequest,
  TeamSharingPolicy,
  TeamSharingSyncDryRun,
  UniversalSyncResult,
  VaultRetentionResult,
  VaultRetentionSettings
} from "../types/cymos";

export default function History() {
  const [activeView, setActiveView] = useState<WorkspaceView>("overview");
  const [query, setQuery] = useState("");
  const [typeFilter, setTypeFilter] = useState("All");
  const [tagFilter, setTagFilter] = useState("All");
  const [collectionFilter, setCollectionFilter] = useState("");
  const [categoryFilter, setCategoryFilter] = useState("All");
  const [favoriteOnly, setFavoriteOnly] = useState(false);
  const [semanticSearch, setSemanticSearch] = useState(true);
  const [similarItems, setSimilarItems] = useState<ClipboardItem[]>([]);
  const deferredQuery = useDeferredValue(query.trim());

  const request = useMemo(
    () => ({
      query: deferredQuery,
      content_type: typeFilter,
      favorite_only: favoriteOnly,
      collection_id: collectionFilter ? Number(collectionFilter) : null,
      tag: tagFilter,
      category: activeView === "operations" ? "Operations" : categoryFilter,
      semantic: semanticSearch
    }),
    [activeView, categoryFilter, collectionFilter, deferredQuery, favoriteOnly, semanticSearch, tagFilter, typeFilter]
  );

  const workspace = useCymosWorkspace(request);

  useEffect(() => {
    function focusSearch(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setActiveView("memory");
        window.setTimeout(() => document.getElementById("memory-search")?.focus(), 0);
      }
    }
    window.addEventListener("keydown", focusSearch);
    return () => window.removeEventListener("keydown", focusSearch);
  }, []);

  async function addCollection() {
    const name = window.prompt("Collection name");
    if (!name?.trim()) return;
    await invoke("create_collection", { name: name.trim(), color: "#0369a1" });
    await workspace.refresh();
  }

  async function findSimilar(itemId: number) {
    const items = await invoke<ClipboardItem[]>("get_similar_memories", { itemId });
    setSimilarItems(items);
  }

  async function saveBrowserBookmark(request: BrowserBookmarkRequest) {
    await invoke("capture_browser_bookmark", { request });
    await workspace.refresh();
  }

  async function saveIdeSnippet(request: IdeSnippetRequest) {
    await invoke("capture_ide_snippet", { request });
    await workspace.refresh();
  }

  async function saveTerminalCommand(request: TerminalCommandRequest) {
    await invoke("capture_terminal_command", { request });
    await workspace.refresh();
  }

  async function linkWorkspaceIncidentEvidence(request: IncidentEvidenceLinkRequest) {
    return invoke<WorkspaceSnapshot>("link_workspace_incident_evidence", { request });
  }

  async function rebuildIndex() {
    await invoke("rebuild_semantic_index");
    await workspace.refresh();
  }

  async function rebuildGraph() {
    await invoke("rebuild_knowledge_graph");
    await workspace.refresh();
  }

  async function askAssistant(question: string) {
    return invoke<AssistantResponse>("ask_memory_assistant", { request: { question } });
  }

  async function runAgent(goal: string) {
    const workflow = await invoke<AgentWorkflow>("run_agent_workflow", { request: { goal } });
    await workspace.refresh();
    return workflow;
  }

  async function runAutomation() {
    const result = await invoke<AutomationRunResult>("run_autonomous_cycle");
    await workspace.refresh();
    return result;
  }

  async function runSync() {
    const result = await invoke<UniversalSyncResult>("run_universal_sync_cycle");
    await workspace.refresh();
    return result;
  }

  async function runReleaseCheck() {
    const result = await invoke<CognitiveReleaseResult>("run_cognitive_release_check");
    await workspace.refresh();
    return result;
  }

  async function checkDatabaseReliability() {
    const status = await invoke<DatabaseReliabilityStatus>("get_database_reliability");
    await workspace.refresh();
    return status;
  }

  async function getDatabaseReliabilityChecksum() {
    return invoke<DatabaseReliabilityChecksum>("get_database_reliability_checksum");
  }

  async function createVerifiedBackup() {
    const backup = await invoke<DatabaseBackup>("create_verified_backup");
    await workspace.refresh();
    return backup;
  }

  async function verifyLatestBackup() {
    const backup = await invoke<DatabaseBackup>("verify_latest_backup");
    await workspace.refresh();
    return backup;
  }

  async function getRecentDatabaseBackups() {
    return invoke<DatabaseBackupSnapshot[]>("get_recent_database_backups");
  }

  async function getRecentDatabaseReliabilityReports() {
    return invoke<DatabaseReliabilityReportSnapshot[]>("get_recent_database_reliability_reports");
  }

  async function exportDatabaseReliabilityReport() {
    const report = await invoke<DatabaseReliabilityReportExport>("export_database_reliability_report");
    await workspace.refresh();
    return report;
  }

  async function verifyDatabaseBackupSnapshot(request: DatabaseBackupVerificationRequest) {
    const backup = await invoke<DatabaseBackup>("verify_database_backup_snapshot", { request });
    await workspace.refresh();
    return backup;
  }

  async function savePrivacySettings(settings: PrivacySettings) {
    await invoke("update_privacy_settings", { settings });
    await workspace.refresh();
  }

  async function saveVaultRetentionSettings(settings: VaultRetentionSettings) {
    await invoke("update_vault_retention_settings", { settings });
    await workspace.refresh();
  }

  async function saveTeamSharingPolicy(policy: TeamSharingPolicy) {
    await invoke("update_team_sharing_policy", { policy });
    await workspace.refresh();
  }

  async function registerTeamSharingDevice(request: TeamSharingDeviceRequest) {
    await invoke("register_team_sharing_device", { request });
    await workspace.refresh();
  }

  async function approveTeamSharingDevice(request: TeamSharingDeviceStatusRequest) {
    await invoke("approve_team_sharing_device", { request });
    await workspace.refresh();
  }

  async function revokeTeamSharingDevice(request: TeamSharingDeviceStatusRequest) {
    await invoke("revoke_team_sharing_device", { request });
    await workspace.refresh();
  }

  async function exportTeamSharingReadinessReport() {
    const result = await invoke<LocalExport>("export_team_sharing_readiness_report");
    await workspace.refresh();
    return result;
  }

  async function exportTeamSharingManifestLedger() {
    const result = await invoke<LocalExport>("export_team_sharing_manifest_ledger");
    await workspace.refresh();
    return result;
  }

  async function exportFilteredTeamSharingManifestLedger(request: TeamSharingManifestLedgerExportRequest) {
    const result = await invoke<LocalExport>("export_filtered_team_sharing_manifest_ledger", { request });
    await workspace.refresh();
    return result;
  }

  async function getTeamSharingManifestLedgerChecksum(request: TeamSharingManifestLedgerExportRequest) {
    return invoke<TeamSharingManifestLedgerChecksum>("get_team_sharing_manifest_ledger_checksum", { request });
  }

  async function runTeamSharingSyncDryRun() {
    const result = await invoke<TeamSharingSyncDryRun>("run_team_sharing_sync_dry_run");
    await workspace.refresh();
    return result;
  }

  async function exportTeamSharingSyncDryRunManifest() {
    const result = await invoke<LocalExport>("export_team_sharing_sync_dry_run_manifest");
    await workspace.refresh();
    return result;
  }

  async function inspectTeamSharingSyncDryRunManifest(request: TeamSharingManifestInspectionRequest) {
    const result = await invoke<TeamSharingManifestInspection>("inspect_team_sharing_sync_dry_run_manifest", { request });
    await workspace.refresh();
    return result;
  }

  async function trustCurrentDeviceTeamSharingSigner() {
    const result = await invoke<HandoffSignerTrustRecord>("trust_current_device_team_sharing_signer");
    await workspace.refresh();
    return result;
  }

  async function applyVaultRetention() {
    const result = await invoke<VaultRetentionResult>("apply_vault_retention");
    await workspace.refresh();
    return result;
  }

  async function importTerminalHistory(shell: "Bash" | "Zsh", maxEntries: number) {
    const result = await invoke<TerminalHistoryImportResult>("import_terminal_history", {
      request: { shell, max_entries: maxEntries }
    });
    await workspace.refresh();
    return result;
  }

  async function saveInsightTrailSettings(settings: InsightTrailSettings) {
    await invoke("update_insight_trail_settings", { settings });
    await workspace.refresh();
  }

  async function recordInsightTrailNote(request: InsightTrailNoteRequest) {
    await invoke("record_insight_trail_note", { request });
    await workspace.refresh();
  }

  async function resolveInsightIncident(incidentId: number) {
    await invoke("resolve_insight_incident", { incidentId });
    await workspace.refresh();
  }

  async function applyInsightTrailRetention() {
    const removed = await invoke<number>("apply_insight_trail_retention");
    await workspace.refresh();
    return removed;
  }

  async function loadWorkspaceSnapshot(workspaceId: number) {
    return invoke<WorkspaceSnapshot>("get_workspace_snapshot", { workspaceId });
  }

  async function createWorkspace(request: WorkspaceCreateRequest) {
    return invoke<WorkspaceSnapshot>("create_cognitive_workspace", { request });
  }

  async function saveWorkspaceContext(workspaceId: number, update: WorkspaceContextUpdate) {
    return invoke<WorkspaceSnapshot>("update_cognitive_workspace", { workspaceId, update });
  }

  async function startWorkspaceSession(request: WorkspaceSessionStartRequest) {
    return invoke<WorkspaceSnapshot>("start_workspace_session", { request });
  }

  async function endWorkspaceSession(sessionId: number) {
    return invoke<WorkspaceSnapshot>("end_workspace_session", { sessionId });
  }

  async function archiveWorkspace(workspaceId: number) {
    return invoke<WorkspaceSnapshot>("archive_cognitive_workspace", { workspaceId });
  }

  async function restoreWorkspace(workspaceId: number) {
    return invoke<WorkspaceSnapshot>("restore_cognitive_workspace", { workspaceId });
  }

  async function importWorkspaceDocument(request: WorkspaceDocumentImportRequest) {
    return invoke<WorkspaceDocumentImportResult>("import_workspace_document", { request });
  }

  async function recordWorkspaceIncidentResolution(request: IncidentResolutionRequest) {
    return invoke<WorkspaceSnapshot>("record_workspace_incident_resolution", { request });
  }

  async function reopenWorkspaceIncident(request: IncidentReopenRequest) {
    return invoke<WorkspaceSnapshot>("reopen_workspace_incident", { request });
  }

  async function exportWorkspaceReport(request: WorkspaceReportRequest) {
    return invoke<LocalExport>("export_workspace_report", { request });
  }

  async function exportWorkspaceHandoff(request: WorkspaceHandoffRequest) {
    return invoke<LocalExport>("export_workspace_handoff", { request });
  }

  async function getWorkspaceHandoffReadiness(request: WorkspaceHandoffRequest) {
    return invoke<WorkspaceHandoffReadiness>("get_workspace_handoff_readiness", { request });
  }

  async function getWorkspaceHandoffExports(workspaceId: number) {
    return invoke<WorkspaceHandoffExportRecord[]>("get_workspace_handoff_exports", { workspaceId });
  }

  async function getHandoffRecipientTrustRecords() {
    return invoke<HandoffRecipientTrustRecord[]>("get_handoff_recipient_trust_records");
  }

  async function trustHandoffRecipient(request: HandoffRecipientTrustRequest) {
    return invoke<HandoffRecipientTrustRecord>("trust_handoff_recipient", { request });
  }

  async function revokeHandoffRecipient(recipient: string) {
    return invoke<void>("revoke_handoff_recipient", { request: { recipient } });
  }

  async function getHandoffSignerTrustRecords() {
    return invoke<HandoffSignerTrustRecord[]>("get_handoff_signer_trust_records");
  }

  async function trustHandoffSigner(request: HandoffSignerTrustRequest) {
    return invoke<HandoffSignerTrustRecord>("trust_handoff_signer", { request });
  }

  async function revokeHandoffSigner(signerFingerprint: string) {
    return invoke<void>("revoke_handoff_signer", { request: { signer_fingerprint: signerFingerprint } });
  }

  async function inspectWorkspaceHandoff(content: string) {
    return invoke<HandoffInspection>("inspect_workspace_handoff", { request: { content } });
  }

  async function getWorkspaceHandoffInspections() {
    return invoke<WorkspaceHandoffInspectionRecord[]>("get_workspace_handoff_inspections");
  }

  async function importWorkspaceHandoff(request: WorkspaceHandoffImportRequest) {
    return invoke<WorkspaceSnapshot>("import_workspace_handoff", { request });
  }

  async function searchRunbookEntries(request: RunbookSearchRequest) {
    return invoke<RunbookEntry[]>("search_runbook_entries", { request });
  }

  async function getRunbookAuditLogs() {
    return invoke<AuditLog[]>("get_runbook_audit_logs");
  }

  async function createManualRunbook(request: ManualRunbookRequest) {
    return invoke<RunbookEntry>("create_manual_runbook", { request });
  }

  async function updateManualRunbook(request: ManualRunbookUpdateRequest) {
    return invoke<RunbookEntry>("update_manual_runbook", { request });
  }

  async function deleteManualRunbook(entryId: number) {
    await invoke("delete_manual_runbook", { entryId });
  }

  async function exportRunbookEntry(entryId: number) {
    return invoke<LocalExport>("export_runbook_entry", { entryId });
  }

  async function copyRunbookEntry(entryId: number) {
    await invoke("copy_runbook_entry", { entryId });
  }

  async function getManualRunbookRevisions(entryId: number) {
    return invoke<RunbookRevision[]>("get_manual_runbook_revisions", { entryId });
  }

  async function restoreManualRunbookRevision(request: ManualRunbookRevisionRestoreRequest) {
    return invoke<RunbookEntry>("restore_manual_runbook_revision", { request });
  }

  async function reviewManualRunbook(request: ManualRunbookReviewRequest) {
    return invoke<RunbookEntry>("review_manual_runbook", { request });
  }

  function renderActiveView() {
    switch (activeView) {
      case "memory":
        return (
          <MemoryView
            categories={workspace.categories}
            categoryFilter={categoryFilter}
            collectionFilter={collectionFilter}
            collections={workspace.collections}
            error={workspace.error}
            favoriteOnly={favoriteOnly}
            items={workspace.items}
            loading={workspace.loading}
            onAddCollection={() => void addCollection()}
            onSaveBrowserBookmark={saveBrowserBookmark}
            onSaveIdeSnippet={saveIdeSnippet}
            onChanged={() => void workspace.refresh()}
            onFindSimilar={(itemId) => void findSimilar(itemId)}
            onRebuildGraph={() => void rebuildGraph()}
            onRebuildIndex={() => void rebuildIndex()}
            query={query}
            semanticSearch={semanticSearch}
            setCategoryFilter={setCategoryFilter}
            setCollectionFilter={setCollectionFilter}
            setFavoriteOnly={setFavoriteOnly}
            setQuery={setQuery}
            setSemanticSearch={setSemanticSearch}
            setTagFilter={setTagFilter}
            setTypeFilter={setTypeFilter}
            similarItems={similarItems}
            stats={workspace.stats}
            tagFilter={tagFilter}
            tags={workspace.tags}
            typeFilter={typeFilter}
          />
        );
      case "operations":
        return (
          <OperationsView
            collections={workspace.collections}
            items={workspace.items}
            loading={workspace.loading}
            onChanged={() => void workspace.refresh()}
            onFindSimilar={(itemId) => void findSimilar(itemId)}
            onImportTerminalHistory={importTerminalHistory}
            onSaveTerminalCommand={saveTerminalCommand}
            query={query}
            setQuery={setQuery}
          />
        );
      case "trail":
        return (
          <InsightTrailView
            events={workspace.insightTrailEvents}
            incidents={workspace.insightIncidents}
            onApplyRetention={applyInsightTrailRetention}
            onRecordNote={recordInsightTrailNote}
            onResolveIncident={resolveInsightIncident}
            onSaveSettings={saveInsightTrailSettings}
            overview={workspace.insightTrailOverview}
            settings={workspace.insightTrailSettings}
          />
        );
      case "workspace":
        return (
          <CognitiveWorkspaceView
            initialSnapshot={workspace.workspaceSnapshot}
            onArchiveWorkspace={archiveWorkspace}
            onCreateWorkspace={createWorkspace}
            onCreateRunbook={createManualRunbook}
            onDeleteRunbook={deleteManualRunbook}
            onExportRunbook={exportRunbookEntry}
            onCopyRunbook={copyRunbookEntry}
            onGetRunbookRevisions={getManualRunbookRevisions}
            onRestoreRunbookRevision={restoreManualRunbookRevision}
            onReviewRunbook={reviewManualRunbook}
            onRevokeHandoffRecipient={revokeHandoffRecipient}
            onRevokeHandoffSigner={revokeHandoffSigner}
            onEndSession={endWorkspaceSession}
            onExportHandoff={exportWorkspaceHandoff}
            onExportReport={exportWorkspaceReport}
            onGetHandoffExports={getWorkspaceHandoffExports}
            onGetHandoffReadiness={getWorkspaceHandoffReadiness}
            onGetHandoffRecipientTrustRecords={getHandoffRecipientTrustRecords}
            onGetHandoffSignerTrustRecords={getHandoffSignerTrustRecords}
            onGetHandoffInspections={getWorkspaceHandoffInspections}
            onImportHandoff={importWorkspaceHandoff}
            onInspectHandoff={inspectWorkspaceHandoff}
            onImportDocument={importWorkspaceDocument}
            onLinkIncidentEvidence={linkWorkspaceIncidentEvidence}
            onLoadWorkspace={loadWorkspaceSnapshot}
            onRecordIncidentResolution={recordWorkspaceIncidentResolution}
            onUpdateRunbook={updateManualRunbook}
            onReopenIncident={reopenWorkspaceIncident}
            onSearchRunbooks={searchRunbookEntries}
            onLoadRunbookAudit={getRunbookAuditLogs}
            onSaveContext={saveWorkspaceContext}
            onStartSession={startWorkspaceSession}
            onTrustHandoffRecipient={trustHandoffRecipient}
            onTrustHandoffSigner={trustHandoffSigner}
            onRestoreWorkspace={restoreWorkspace}
            onWorkspacesChanged={workspace.refreshCognitiveWorkspaces}
            workspaces={workspace.cognitiveWorkspaces}
          />
        );
      case "assistant":
        return <AssistantView onAsk={askAssistant} />;
      case "agents":
        return <AgentView history={workspace.agentHistory} onRun={runAgent} />;
      case "automation":
        return <AutomationView health={workspace.health} notifications={workspace.notifications} onRun={runAutomation} reports={workspace.reports} tasks={workspace.automationTasks} />;
      case "graph":
        return <GraphView graph={workspace.graph} modules={workspace.cognitiveModules} onRebuild={() => void rebuildGraph()} />;
      case "platform":
        return (
          <PlatformView
            apiClients={workspace.apiClients}
            auditLogs={workspace.auditLogs}
            databaseReliability={workspace.databaseReliability}
            connectors={workspace.connectors}
            controls={workspace.enterpriseControls}
            devices={workspace.devices}
            onCreateBackup={createVerifiedBackup}
            onVerifyLatestBackup={verifyLatestBackup}
            onGetRecentBackups={getRecentDatabaseBackups}
            onGetRecentReliabilityReports={getRecentDatabaseReliabilityReports}
            onVerifyBackupSnapshot={verifyDatabaseBackupSnapshot}
            onExportDatabaseReliabilityReport={exportDatabaseReliabilityReport}
            onCheckDatabaseReliability={checkDatabaseReliability}
            onGetDatabaseReliabilityChecksum={getDatabaseReliabilityChecksum}
            onSavePrivacySettings={savePrivacySettings}
            onRegisterTeamSharingDevice={registerTeamSharingDevice}
            onApproveTeamSharingDevice={approveTeamSharingDevice}
            onRevokeTeamSharingDevice={revokeTeamSharingDevice}
            onExportTeamSharingReport={exportTeamSharingReadinessReport}
            onExportTeamSharingManifestLedger={exportTeamSharingManifestLedger}
            onExportFilteredTeamSharingManifestLedger={exportFilteredTeamSharingManifestLedger}
            onGetTeamSharingManifestLedgerChecksum={getTeamSharingManifestLedgerChecksum}
            onRunTeamSharingDryRun={runTeamSharingSyncDryRun}
            onExportTeamSharingDryRunManifest={exportTeamSharingSyncDryRunManifest}
            onInspectTeamSharingManifest={inspectTeamSharingSyncDryRunManifest}
            onTrustCurrentDeviceSigner={trustCurrentDeviceTeamSharingSigner}
            onSaveTeamSharingPolicy={saveTeamSharingPolicy}
            onSaveVaultRetentionSettings={saveVaultRetentionSettings}
            onApplyVaultRetention={applyVaultRetention}
            onRunSync={runSync}
            platform={workspace.platform}
            plugins={workspace.plugins}
            privacyStatus={workspace.privacyStatus}
            manifestLedgerAuditLogs={workspace.manifestLedgerAuditLogs}
            teamSharingAuditLogs={workspace.teamSharingAuditLogs}
            teamSharingReadiness={workspace.teamSharingReadiness}
            teamSharingPolicy={workspace.teamSharingPolicy}
            useCases={workspace.cognitiveUseCases}
            vaultRetentionSettings={workspace.vaultRetentionSettings}
          />
        );
      case "insights":
        return <InsightsView dailySummary={workspace.dailySummary} graph={workspace.graph} health={workspace.health} stats={workspace.stats} weeklyReport={workspace.weeklyReport} />;
      default:
        return (
          <OverviewView
            dailySummary={workspace.dailySummary}
            graph={workspace.graph}
            health={workspace.health}
            items={workspace.items}
            onOpenAssistant={() => setActiveView("assistant")}
            onOpenMemory={() => setActiveView("memory")}
            onRunReleaseCheck={runReleaseCheck}
            overview={workspace.cognitiveOverview}
            stats={workspace.stats}
            weeklyReport={workspace.weeklyReport}
          />
        );
    }
  }

  return (
    <AppShell
      activeView={activeView}
      memoryCount={workspace.stats.total_items}
      onRefresh={() => void workspace.refresh()}
      onViewChange={setActiveView}
      refreshing={workspace.loading}
    >
      {renderActiveView()}
    </AppShell>
  );
}
