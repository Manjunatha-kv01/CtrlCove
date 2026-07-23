import { convertFileSrc } from "@tauri-apps/api/core";
import {
  Briefcase,
  BookOpen,
  Archive,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  CircleDot,
  Clock3,
  Copy,
  Download,
  FileCheck2,
  FileJson,
  FileUp,
  FolderPlus,
  Link2,
  Pause,
  Pencil,
  Play,
  Plus,
  RotateCcw,
  Save,
  Search,
  ShieldAlert,
  Sparkles,
  Square,
  Trash2,
  Workflow
} from "lucide-react";
import { type ChangeEvent, type FormEvent, useEffect, useMemo, useState } from "react";
import { Panel, SectionHeading } from "./AppShell";
import type {
  CognitiveWorkspace,
  AuditLog,
  HandoffRecipientTrustRecord,
  HandoffRecipientTrustRequest,
  HandoffSignerTrustRecord,
  HandoffSignerTrustRequest,
  HandoffInspection,
  InsightTrailEvent,
  IncidentEvidenceLinkRequest,
  IncidentReopenRequest,
  IncidentResolutionRequest,
  LocalExport,
  ManualRunbookRequest,
  ManualRunbookReviewRequest,
  ManualRunbookRevisionRestoreRequest,
  ManualRunbookUpdateRequest,
  RunbookEntry,
  RunbookRevision,
  RunbookSearchRequest,
  WorkspaceContextUpdate,
  WorkspaceCreateRequest,
  WorkspaceDocumentImportRequest,
  WorkspaceDocumentImportResult,
  WorkspaceHandoffExportRecord,
  WorkspaceHandoffInspectionRecord,
  WorkspaceHandoffRequest,
  WorkspaceHandoffReadiness,
  WorkspaceSessionStartRequest,
  WorkspaceSnapshot
} from "../types/cymos";

const workspaceEventTypes = ["All", "Clipboard", "Terminal", "Screenshot", "Error", "Note"] as const;

function handoffExpiryLabel(expiresAtUnix: number | null) {
  return expiresAtUnix === null
    ? "No expiry"
    : `Expires ${new Date(expiresAtUnix * 1_000).toLocaleString()}`;
}

function handoffSignatureLabel(signatureVerified: boolean, signerFingerprint: string | null) {
  return signatureVerified && signerFingerprint
    ? `Signed by ${signerFingerprint}`
    : "Unsigned legacy package";
}

function handoffClassificationRank(classification: WorkspaceHandoffRequest["classification"]) {
  return { Internal: 1, Restricted: 2, Confidential: 3 }[classification];
}

export function CognitiveWorkspaceView({
  initialSnapshot,
  workspaces,
  onArchiveWorkspace,
  onLoadWorkspace,
  onCreateWorkspace,
  onCreateRunbook,
  onDeleteRunbook,
  onCopyRunbook,
  onExportRunbook,
  onGetRunbookRevisions,
  onRestoreRunbookRevision,
  onReviewRunbook,
  onSaveContext,
  onStartSession,
  onRestoreWorkspace,
  onEndSession,
  onExportHandoff,
  onExportReport,
  onGetHandoffExports,
  onGetHandoffReadiness,
  onGetHandoffRecipientTrustRecords,
  onGetHandoffSignerTrustRecords,
  onGetHandoffInspections,
  onImportHandoff,
  onInspectHandoff,
  onImportDocument,
  onRecordIncidentResolution,
  onReopenIncident,
  onLinkIncidentEvidence,
  onUpdateRunbook,
  onSearchRunbooks,
  onLoadRunbookAudit,
  onRevokeHandoffRecipient,
  onRevokeHandoffSigner,
  onTrustHandoffRecipient,
  onTrustHandoffSigner,
  onWorkspacesChanged
}: {
  initialSnapshot: WorkspaceSnapshot;
  workspaces: CognitiveWorkspace[];
  onArchiveWorkspace: (workspaceId: number) => Promise<WorkspaceSnapshot>;
  onLoadWorkspace: (workspaceId: number) => Promise<WorkspaceSnapshot>;
  onCreateWorkspace: (request: WorkspaceCreateRequest) => Promise<WorkspaceSnapshot>;
  onCreateRunbook: (request: ManualRunbookRequest) => Promise<RunbookEntry>;
  onDeleteRunbook: (entryId: number) => Promise<void>;
  onCopyRunbook: (entryId: number) => Promise<void>;
  onExportRunbook: (entryId: number) => Promise<LocalExport>;
  onGetRunbookRevisions: (entryId: number) => Promise<RunbookRevision[]>;
  onRestoreRunbookRevision: (request: ManualRunbookRevisionRestoreRequest) => Promise<RunbookEntry>;
  onReviewRunbook: (request: ManualRunbookReviewRequest) => Promise<RunbookEntry>;
  onSaveContext: (workspaceId: number, update: WorkspaceContextUpdate) => Promise<WorkspaceSnapshot>;
  onStartSession: (request: WorkspaceSessionStartRequest) => Promise<WorkspaceSnapshot>;
  onRestoreWorkspace: (workspaceId: number) => Promise<WorkspaceSnapshot>;
  onEndSession: (sessionId: number) => Promise<WorkspaceSnapshot>;
  onExportHandoff: (request: WorkspaceHandoffRequest) => Promise<LocalExport>;
  onExportReport: (request: { workspace_id: number; session_id: number | null }) => Promise<LocalExport>;
  onGetHandoffExports: (workspaceId: number) => Promise<WorkspaceHandoffExportRecord[]>;
  onGetHandoffReadiness: (request: WorkspaceHandoffRequest) => Promise<WorkspaceHandoffReadiness>;
  onGetHandoffRecipientTrustRecords: () => Promise<HandoffRecipientTrustRecord[]>;
  onGetHandoffSignerTrustRecords: () => Promise<HandoffSignerTrustRecord[]>;
  onGetHandoffInspections: () => Promise<WorkspaceHandoffInspectionRecord[]>;
  onImportHandoff: (request: { content: string }) => Promise<WorkspaceSnapshot>;
  onInspectHandoff: (content: string) => Promise<HandoffInspection>;
  onImportDocument: (request: WorkspaceDocumentImportRequest) => Promise<WorkspaceDocumentImportResult>;
  onRecordIncidentResolution: (request: IncidentResolutionRequest) => Promise<WorkspaceSnapshot>;
  onReopenIncident: (request: IncidentReopenRequest) => Promise<WorkspaceSnapshot>;
  onLinkIncidentEvidence: (request: IncidentEvidenceLinkRequest) => Promise<WorkspaceSnapshot>;
  onUpdateRunbook: (request: ManualRunbookUpdateRequest) => Promise<RunbookEntry>;
  onSearchRunbooks: (request: RunbookSearchRequest) => Promise<RunbookEntry[]>;
  onLoadRunbookAudit: () => Promise<AuditLog[]>;
  onRevokeHandoffRecipient: (recipient: string) => Promise<void>;
  onRevokeHandoffSigner: (signerFingerprint: string) => Promise<void>;
  onTrustHandoffRecipient: (request: HandoffRecipientTrustRequest) => Promise<HandoffRecipientTrustRecord>;
  onTrustHandoffSigner: (request: HandoffSignerTrustRequest) => Promise<HandoffSignerTrustRecord>;
  onWorkspacesChanged: () => Promise<void>;
}) {
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot>(initialSnapshot);
  const { active_session: activeSession, events, incidents, sessions, workspace } = snapshot;
  const [draft, setDraft] = useState<WorkspaceContextUpdate>({ name: workspace.name, project: workspace.project });
  const [newWorkspace, setNewWorkspace] = useState<WorkspaceCreateRequest>({ name: "", project: "" });
  const [sessionTitle, setSessionTitle] = useState("");
  const [resolutionDraft, setResolutionDraft] = useState({ incidentId: "", title: "", details: "" });
  const [reopenDraft, setReopenDraft] = useState({ incidentId: "", reason: "" });
  const [evidenceDraft, setEvidenceDraft] = useState({ incidentId: "", eventId: "" });
  const [runbookQuery, setRunbookQuery] = useState("");
  const [runbookReviewStatus, setRunbookReviewStatus] = useState<RunbookSearchRequest["review_status"]>("Needs review");
  const [runbookDraft, setRunbookDraft] = useState({ title: "", details: "", tags: "" });
  const [editingRunbookId, setEditingRunbookId] = useState<number | null>(null);
  const [runbookEntries, setRunbookEntries] = useState<RunbookEntry[]>([]);
  const [runbookAuditLogs, setRunbookAuditLogs] = useState<AuditLog[]>([]);
  const [loadingRunbook, setLoadingRunbook] = useState(false);
  const [savingRunbook, setSavingRunbook] = useState(false);
  const [exportingRunbookId, setExportingRunbookId] = useState<number | null>(null);
  const [copyingRunbookId, setCopyingRunbookId] = useState<number | null>(null);
  const [revisionsRunbookId, setRevisionsRunbookId] = useState<number | null>(null);
  const [runbookRevisions, setRunbookRevisions] = useState<RunbookRevision[]>([]);
  const [loadingRevisions, setLoadingRevisions] = useState(false);
  const [restoringRevisionId, setRestoringRevisionId] = useState<number | null>(null);
  const [reviewingRunbookId, setReviewingRunbookId] = useState<number | null>(null);
  const [showRunbookForm, setShowRunbookForm] = useState(false);
  const [importingDocument, setImportingDocument] = useState(false);
  const [inspectingHandoff, setInspectingHandoff] = useState(false);
  const [importingHandoff, setImportingHandoff] = useState(false);
  const [handoffInspection, setHandoffInspection] = useState<HandoffInspection | null>(null);
  const [handoffReadiness, setHandoffReadiness] = useState<WorkspaceHandoffReadiness | null>(null);
  const [handoffExports, setHandoffExports] = useState<WorkspaceHandoffExportRecord[]>([]);
  const [handoffInspections, setHandoffInspections] = useState<WorkspaceHandoffInspectionRecord[]>([]);
  const [handoffTrustRecords, setHandoffTrustRecords] = useState<HandoffRecipientTrustRecord[]>([]);
  const [handoffSignerTrustRecords, setHandoffSignerTrustRecords] = useState<HandoffSignerTrustRecord[]>([]);
  const [handoffExcludedEventIds, setHandoffExcludedEventIds] = useState<number[]>([]);
  const [handoffDeclaration, setHandoffDeclaration] = useState<Pick<WorkspaceHandoffRequest, "recipient" | "purpose" | "classification" | "expires_in_days">>({ recipient: "", purpose: "", classification: "Internal", expires_in_days: 7 });
  const [verifiedHandoffContent, setVerifiedHandoffContent] = useState<string | null>(null);
  const [showWorkspaceForm, setShowWorkspaceForm] = useState(false);
  const [query, setQuery] = useState("");
  const [eventType, setEventType] = useState<(typeof workspaceEventTypes)[number]>("All");
  const [sessionFilter, setSessionFilter] = useState("All");
  const [replayIndex, setReplayIndex] = useState<number | null>(null);
  const [playing, setPlaying] = useState(false);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (snapshot.workspace.id !== initialSnapshot.workspace.id) return;
    setSnapshot(initialSnapshot);
    setDraft({ name: initialSnapshot.workspace.name, project: initialSnapshot.workspace.project });
  }, [initialSnapshot, snapshot.workspace.id]);

  useEffect(() => {
    let active = true;
    setLoadingRunbook(true);
    void onSearchRunbooks({ query: "", review_status: "Needs review" })
      .then((entries) => {
        if (active) setRunbookEntries(entries);
      })
      .catch((cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (active) setLoadingRunbook(false);
      });
    void onLoadRunbookAudit()
      .then((logs) => {
        if (active) setRunbookAuditLogs(logs);
      })
      .catch((cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    let active = true;
    void onGetHandoffExports(workspace.id)
      .then((exports) => {
        if (active) setHandoffExports(exports);
      })
      .catch((cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      });
    void onGetHandoffRecipientTrustRecords()
      .then((records) => {
        if (active) setHandoffTrustRecords(records);
      })
      .catch((cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      });
    void onGetHandoffInspections()
      .then((records) => {
        if (active) setHandoffInspections(records);
      })
      .catch((cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      });
    void onGetHandoffSignerTrustRecords()
      .then((records) => {
        if (active) setHandoffSignerTrustRecords(records);
      })
      .catch((cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      });
    return () => {
      active = false;
    };
  }, [workspace.id]);

  const filteredEvents = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return events.filter((event) => {
      if (eventType !== "All" && event.event_type !== eventType) return false;
      if (sessionFilter !== "All" && event.session_id !== Number(sessionFilter)) return false;
      if (!normalizedQuery) return true;
      return [event.title, event.details, event.source_application, ...event.tags]
        .join(" ")
        .toLowerCase()
        .includes(normalizedQuery);
    });
  }, [eventType, events, query, sessionFilter]);

  const replayEvents = useMemo(() => [...filteredEvents].reverse(), [filteredEvents]);
  const replayEvent = replayIndex === null ? null : replayEvents[replayIndex] ?? null;
  const linkableEvents = useMemo(
    () => events.filter((event) => event.memory_id !== null && event.incident_id === null),
    [events]
  );
  const handoffScopeEvents = useMemo(
    () => events.filter((event) => sessionFilter === "All" || event.session_id === Number(sessionFilter)),
    [events, sessionFilter]
  );
  const trustedRecipient = useMemo(
    () => handoffTrustRecords.find((record) => record.recipient.toLowerCase() === handoffDeclaration.recipient.trim().toLowerCase()) ?? null,
    [handoffDeclaration.recipient, handoffTrustRecords]
  );
  const recipientTrustAllowsClassification = trustedRecipient?.is_active
    ? handoffClassificationRank(handoffDeclaration.classification) <= handoffClassificationRank(trustedRecipient.max_classification)
    : false;
  const verifiedSignerTrust = useMemo(
    () => handoffInspection?.signer_fingerprint
      ? handoffSignerTrustRecords.find((record) => record.signer_fingerprint === handoffInspection.signer_fingerprint) ?? null
      : null,
    [handoffInspection?.signer_fingerprint, handoffSignerTrustRecords]
  );

  function handoffRequest(): WorkspaceHandoffRequest {
    return {
      workspace_id: workspace.id,
      session_id: sessionFilter === "All" ? null : Number(sessionFilter),
      excluded_event_ids: handoffExcludedEventIds,
      ...handoffDeclaration
    };
  }

  function toggleHandoffEvent(eventId: number) {
    setHandoffExcludedEventIds((current) => current.includes(eventId)
      ? current.filter((id) => id !== eventId)
      : [...current, eventId]);
    setHandoffReadiness(null);
  }

  function useTrustedRecipient(record: HandoffRecipientTrustRecord) {
    setHandoffDeclaration((current) => ({
      ...current,
      recipient: record.recipient,
      classification: record.max_classification
    }));
    setHandoffReadiness(null);
  }

  useEffect(() => {
    if (!playing || replayEvents.length === 0) return;
    const timer = window.setInterval(() => {
      setReplayIndex((current) => {
        const next = (current ?? 0) + 1;
        if (next >= replayEvents.length) {
          setPlaying(false);
          return replayEvents.length - 1;
        }
        return next;
      });
    }, 1500);
    return () => window.clearInterval(timer);
  }, [playing, replayEvents.length]);

  async function saveContext(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setMessage(null);
    setError(null);
    try {
      const nextSnapshot = await onSaveContext(workspace.id, draft);
      setSnapshot(nextSnapshot);
      await onWorkspacesChanged();
      setMessage("Workspace context saved locally.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function selectWorkspace(workspaceId: number) {
    if (workspaceId === workspace.id) return;
    setSaving(true);
    setError(null);
    try {
      setSnapshot(await onLoadWorkspace(workspaceId));
      setReplayIndex(null);
      setPlaying(false);
      setSessionFilter("All");
      setHandoffExcludedEventIds([]);
      setHandoffReadiness(null);
      setHandoffDeclaration({ recipient: "", purpose: "", classification: "Internal", expires_in_days: 7 });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function createWorkspace(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const nextSnapshot = await onCreateWorkspace(newWorkspace);
      setSnapshot(nextSnapshot);
      setSessionFilter("All");
      setHandoffExcludedEventIds([]);
      setHandoffReadiness(null);
      setHandoffDeclaration({ recipient: "", purpose: "", classification: "Internal", expires_in_days: 7 });
      setNewWorkspace({ name: "", project: "" });
      setShowWorkspaceForm(false);
      setMessage("Workspace created. Start a session when you are ready to capture project context.");
      await onWorkspacesChanged();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function startSession(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const nextSnapshot = await onStartSession({ workspace_id: workspace.id, title: sessionTitle });
      setSnapshot(nextSnapshot);
      setSessionFilter(nextSnapshot.active_session ? String(nextSnapshot.active_session.id) : "All");
      setHandoffExcludedEventIds([]);
      setHandoffReadiness(null);
      setHandoffDeclaration({ recipient: "", purpose: "", classification: "Internal", expires_in_days: 7 });
      setSessionTitle("");
      setMessage("Capture is now linked to this workspace session.");
      await onWorkspacesChanged();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function endSession() {
    if (!activeSession) return;
    setSaving(true);
    setError(null);
    try {
      const nextSnapshot = await onEndSession(activeSession.id);
      setSnapshot(nextSnapshot);
      setMessage("Session closed. New captures will remain unassigned until another session starts.");
      await onWorkspacesChanged();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function archiveWorkspace() {
    if (!window.confirm(`Archive ${workspace.name}? Its history, incidents, and runbooks stay local and can be restored later.`)) return;
    setSaving(true);
    setError(null);
    try {
      setSnapshot(await onArchiveWorkspace(workspace.id));
      setMessage("Workspace archived locally. Restore it to begin a new capture session.");
      await onWorkspacesChanged();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function restoreWorkspace() {
    setSaving(true);
    setError(null);
    try {
      setSnapshot(await onRestoreWorkspace(workspace.id));
      setMessage("Workspace restored. Start a capture session when you are ready.");
      await onWorkspacesChanged();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function recordResolution(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const nextSnapshot = await onRecordIncidentResolution({
        workspace_id: workspace.id,
        incident_id: Number(resolutionDraft.incidentId),
        title: resolutionDraft.title,
        details: resolutionDraft.details
      });
      setSnapshot(nextSnapshot);
      setResolutionDraft({ incidentId: "", title: "", details: "" });
      setMessage("Resolution recorded locally and incident marked resolved.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function reopenIncident(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!activeSession) {
      setError("Start a capture session before reopening an incident.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const nextSnapshot = await onReopenIncident({
        workspace_id: workspace.id,
        incident_id: Number(reopenDraft.incidentId),
        reason: reopenDraft.reason
      });
      setSnapshot(nextSnapshot);
      setReopenDraft({ incidentId: "", reason: "" });
      setMessage("Incident reopened and follow-up reason recorded in this workspace timeline.");
      await onWorkspacesChanged();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function linkIncidentEvidence(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const nextSnapshot = await onLinkIncidentEvidence({
        workspace_id: workspace.id,
        incident_id: Number(evidenceDraft.incidentId),
        event_id: Number(evidenceDraft.eventId)
      });
      setSnapshot(nextSnapshot);
      setEvidenceDraft({ incidentId: "", eventId: "" });
      setMessage("Saved memory linked to incident evidence.");
      await onWorkspacesChanged();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function exportReport() {
    setSaving(true);
    setError(null);
    try {
      const exported = await onExportReport({
        workspace_id: workspace.id,
        session_id: sessionFilter === "All" ? null : Number(sessionFilter)
      });
      setMessage(`Local Markdown report created: ${exported.path}`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function exportHandoff() {
    setSaving(true);
    setError(null);
    try {
      const readiness = await onGetHandoffReadiness(handoffRequest());
      setHandoffReadiness(readiness);
      if (!readiness.safe) {
        setMessage("Handoff export remains local and blocked until the safety findings are resolved.");
        return;
      }
      const exported = await onExportHandoff(handoffRequest());
      setHandoffExports(await onGetHandoffExports(workspace.id));
      setMessage(`Local JSON handoff package created: ${exported.path}`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function reviewHandoffReadiness() {
    setSaving(true);
    setError(null);
    try {
      const readiness = await onGetHandoffReadiness(handoffRequest());
      setHandoffReadiness(readiness);
      setMessage(readiness.safe ? "Handoff scope passed the local safety review." : "Handoff export is blocked by the local safety review.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function trustCurrentHandoffRecipient() {
    setSaving(true);
    setError(null);
    try {
      const record = await onTrustHandoffRecipient({
        recipient: handoffDeclaration.recipient,
        max_classification: handoffDeclaration.classification,
        note: `Approved locally for ${workspace.name}`
      });
      setHandoffTrustRecords(await onGetHandoffRecipientTrustRecords());
      setHandoffReadiness(null);
      setMessage(`Trusted handoff recipient saved: ${record.recipient}`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function revokeCurrentHandoffRecipient() {
    if (!trustedRecipient) return;
    setSaving(true);
    setError(null);
    try {
      await onRevokeHandoffRecipient(trustedRecipient.recipient);
      setHandoffTrustRecords(await onGetHandoffRecipientTrustRecords());
      setHandoffReadiness(null);
      setMessage(`Trusted handoff recipient revoked: ${trustedRecipient.recipient}`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function trustVerifiedHandoffSigner() {
    if (!handoffInspection?.signer_fingerprint) return;
    setSaving(true);
    setError(null);
    try {
      const record = await onTrustHandoffSigner({
        signer_fingerprint: handoffInspection.signer_fingerprint,
        label: handoffInspection.workspace_name
      });
      setHandoffSignerTrustRecords(await onGetHandoffSignerTrustRecords());
      setMessage(`Trusted handoff signer saved: ${record.signer_fingerprint}`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function revokeVerifiedHandoffSigner() {
    if (!handoffInspection?.signer_fingerprint) return;
    setSaving(true);
    setError(null);
    try {
      await onRevokeHandoffSigner(handoffInspection.signer_fingerprint);
      setHandoffSignerTrustRecords(await onGetHandoffSignerTrustRecords());
      setMessage(`Trusted handoff signer revoked: ${handoffInspection.signer_fingerprint}`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function inspectHandoff(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    if (file.size > 1_048_576) {
      setError("Handoff verification is limited to 1 MB.");
      return;
    }
    setInspectingHandoff(true);
    setError(null);
    setHandoffInspection(null);
    setVerifiedHandoffContent(null);
    try {
      const content = await file.text();
      setHandoffInspection(await onInspectHandoff(content));
      setVerifiedHandoffContent(content);
      setHandoffInspections(await onGetHandoffInspections());
      setMessage("Handoff package verified locally. Nothing has been imported.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setHandoffInspections(await onGetHandoffInspections());
    } finally {
      setInspectingHandoff(false);
    }
  }

  async function importHandoff() {
    if (!verifiedHandoffContent) return;
    setImportingHandoff(true);
    setError(null);
    try {
      const imported = await onImportHandoff({ content: verifiedHandoffContent });
      setSnapshot(imported);
      setSessionFilter("All");
      setHandoffExcludedEventIds([]);
      setHandoffReadiness(null);
      setHandoffDeclaration({ recipient: "", purpose: "", classification: "Internal", expires_in_days: 7 });
      setReplayIndex(null);
      setPlaying(false);
      setVerifiedHandoffContent(null);
      setMessage("Verified handoff imported as an isolated local workspace. Raw clipboard memories were not merged.");
      await onWorkspacesChanged();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setImportingHandoff(false);
    }
  }

  async function searchRunbooks(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setLoadingRunbook(true);
    setError(null);
    try {
      setRunbookEntries(await onSearchRunbooks({ query: runbookQuery, review_status: runbookReviewStatus }));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoadingRunbook(false);
    }
  }

  async function refreshRunbookAudit() {
    try {
      setRunbookAuditLogs(await onLoadRunbookAudit());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function createRunbook(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSavingRunbook(true);
    setError(null);
    try {
      const request = {
        title: runbookDraft.title,
        details: runbookDraft.details,
        tags: runbookDraft.tags.split(",").map((tag) => tag.trim()).filter(Boolean)
      };
      const entry = editingRunbookId === null
        ? await onCreateRunbook(request)
        : await onUpdateRunbook({ ...request, id: editingRunbookId });
      setRunbookEntries(await onSearchRunbooks({ query: entry.title, review_status: runbookReviewStatus }));
      setRunbookQuery(entry.title);
      setRunbookDraft({ title: "", details: "", tags: "" });
      setEditingRunbookId(null);
      setShowRunbookForm(false);
      setMessage(editingRunbookId === null ? "Local runbook saved and ready to search." : "Local runbook updated.");
      void refreshRunbookAudit();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSavingRunbook(false);
    }
  }

  function beginRunbookEdit(entry: RunbookEntry) {
    setEditingRunbookId(entry.id);
    setRunbookDraft({ title: entry.title, details: entry.details, tags: entry.tags.join(", ") });
    setShowRunbookForm(true);
  }

  async function deleteRunbook(entry: RunbookEntry) {
    if (!window.confirm(`Delete local runbook "${entry.title}"?`)) return;
    setSavingRunbook(true);
    setError(null);
    try {
      await onDeleteRunbook(entry.id);
      setRunbookEntries((current) => current.filter((item) => item.id !== entry.id));
      setMessage("Local runbook deleted.");
      void refreshRunbookAudit();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSavingRunbook(false);
    }
  }

  async function exportRunbook(entry: RunbookEntry) {
    setExportingRunbookId(entry.id);
    setError(null);
    try {
      const exported = await onExportRunbook(entry.id);
      setMessage(`Local runbook export created: ${exported.path}`);
      void refreshRunbookAudit();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setExportingRunbookId(null);
    }
  }

  async function copyRunbook(entry: RunbookEntry) {
    setCopyingRunbookId(entry.id);
    setError(null);
    try {
      await onCopyRunbook(entry.id);
      setMessage("Runbook procedure copied to the system clipboard.");
      void refreshRunbookAudit();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setCopyingRunbookId(null);
    }
  }

  async function toggleRunbookRevisions(entry: RunbookEntry) {
    if (revisionsRunbookId === entry.id) {
      setRevisionsRunbookId(null);
      setRunbookRevisions([]);
      return;
    }
    setLoadingRevisions(true);
    setError(null);
    try {
      setRunbookRevisions(await onGetRunbookRevisions(entry.id));
      setRevisionsRunbookId(entry.id);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoadingRevisions(false);
    }
  }

  async function restoreRunbookRevision(entry: RunbookEntry, revision: RunbookRevision) {
    if (!window.confirm(`Restore version ${revision.revision} of "${entry.title}" as the latest version?`)) return;
    setRestoringRevisionId(revision.id);
    setError(null);
    try {
      const restored = await onRestoreRunbookRevision({ entry_id: entry.id, revision_id: revision.id });
      setRunbookEntries((current) => current.map((item) => item.id === restored.id ? restored : item));
      setRunbookRevisions(await onGetRunbookRevisions(entry.id));
      setMessage(`Version ${revision.revision} restored as the latest local runbook revision.`);
      void refreshRunbookAudit();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setRestoringRevisionId(null);
    }
  }

  async function reviewRunbook(entry: RunbookEntry) {
    const note = window.prompt(`Optional review evidence for "${entry.title}"`);
    if (note === null) return;
    setReviewingRunbookId(entry.id);
    setError(null);
    try {
      const reviewed = await onReviewRunbook({ entry_id: entry.id, note });
      setRunbookEntries((current) => current.map((item) => item.id === reviewed.id ? reviewed : item));
      setMessage(`Runbook version ${reviewed.latest_revision} marked as reviewed.`);
      void refreshRunbookAudit();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setReviewingRunbookId(null);
    }
  }

  async function importDocument(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    if (workspace.is_imported) {
      setError("Imported handoff workspaces are read-only references.");
      return;
    }
    if (workspace.status === "Archived") {
      setError("Restore this workspace before importing a local file.");
      return;
    }
    if (!activeSession) {
      setError("Start a capture session before importing a local file.");
      return;
    }
    if (file.size > 1_048_576) {
      setError("Local file import is limited to 1 MB.");
      return;
    }
    setImportingDocument(true);
    setMessage(null);
    setError(null);
    try {
      const result = await onImportDocument({
        workspace_id: workspace.id,
        file_name: file.name,
        content: await file.text()
      });
      setSnapshot(result.snapshot);
      setMessage(result.stored ? `${file.name} imported into this workspace.` : `${file.name} was not stored because it is already present or blocked by capture privacy.`);
      await onWorkspacesChanged();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setImportingDocument(false);
    }
  }

  function startReplay() {
    if (replayEvents.length === 0) {
      setMessage("Capture an event before starting Memory Replay.");
      return;
    }
    setMessage(null);
    setReplayIndex(0);
    setPlaying(true);
  }

  function setReplayStep(nextIndex: number) {
    setPlaying(false);
    setReplayIndex(Math.max(0, Math.min(nextIndex, replayEvents.length - 1)));
  }

  return (
    <div className="grid gap-5">
      <section className="glass-hero overflow-hidden rounded-lg px-5 py-6 text-white sm:px-7 sm:py-8">
        <div className="flex flex-col gap-7 lg:flex-row lg:items-end lg:justify-between">
          <div className="min-w-0 max-w-3xl">
            <div className="inline-flex items-center gap-2 rounded-md border border-white/15 bg-white/10 px-2.5 py-1 text-xs font-medium text-sky-100">
              <Briefcase size={14} />
              Cognitive workspace
              <span className={`h-1.5 w-1.5 rounded-full ${workspace.status === "Archived" ? "bg-slate-400" : workspace.is_imported ? "bg-amber-400" : "bg-emerald-400"}`} />
              {workspace.is_imported ? "Imported reference" : workspace.status}
            </div>
            <div className="mt-4 flex flex-col gap-2 sm:flex-row sm:items-center">
              <label className="sr-only" htmlFor="workspace-selector">Select workspace</label>
              <select className="h-10 min-w-0 rounded-md border border-white/20 bg-slate-900 px-3 text-sm font-medium text-white outline-none focus:border-sky-300 focus:ring-2 focus:ring-sky-300/30 sm:w-72" disabled={saving} id="workspace-selector" onChange={(event) => void selectWorkspace(Number(event.target.value))} value={workspace.id}>
                {workspaces.map((item) => <option key={item.id} value={item.id}>{item.name} - {item.project}{item.is_imported ? " (Imported)" : item.status === "Archived" ? " (Archived)" : ""}</option>)}
              </select>
              <button className="inline-flex h-10 items-center justify-center gap-2 rounded-md border border-white/20 bg-white/10 px-3 text-sm font-medium text-white transition-colors hover:bg-white/20" onClick={() => setShowWorkspaceForm((current) => !current)} type="button"><FolderPlus size={16} />New workspace</button>
            </div>
            <h2 className="mt-5 break-words text-2xl font-semibold tracking-tight sm:text-3xl">{workspace.name}</h2>
            <p className="mt-2 text-sm font-medium text-sky-100">{workspace.project}</p>
            <p className="mt-3 max-w-2xl text-sm leading-6 text-slate-300">{workspace.summary}</p>
          </div>
          <div className="grid w-full gap-2 sm:flex sm:w-fit">
            {workspace.status === "Archived" ? <button className="inline-flex h-10 items-center justify-center gap-2 rounded-md bg-emerald-500 px-4 text-sm font-medium text-slate-950 transition-colors hover:bg-emerald-400 disabled:opacity-60" disabled={saving} onClick={() => void restoreWorkspace()} type="button"><RotateCcw size={16} />Restore workspace</button> : <button className="inline-flex h-10 items-center justify-center gap-2 rounded-md border border-white/20 bg-white/10 px-4 text-sm font-medium text-white transition-colors hover:bg-white/20 disabled:opacity-60" disabled={saving} onClick={() => void archiveWorkspace()} type="button"><Archive size={16} />Archive workspace</button>}
            <div className="flex gap-2">
              <button className="inline-flex h-10 items-center justify-center gap-2 rounded-md border border-white/20 bg-white/10 px-4 text-sm font-medium text-white transition-colors hover:bg-white/20 disabled:opacity-60" disabled={saving} onClick={() => void exportReport()} type="button"><Download size={16} />Export report</button>
              <button aria-label="Review handoff safety" className="inline-flex h-10 w-10 items-center justify-center rounded-md border border-white/20 bg-white/10 text-white transition-colors hover:bg-white/20 disabled:opacity-60" disabled={saving} onClick={() => void reviewHandoffReadiness()} title="Review handoff safety" type="button"><FileCheck2 size={17} /></button>
            </div>
            <button className="inline-flex h-10 items-center justify-center gap-2 rounded-md bg-white px-4 text-sm font-medium text-slate-950 transition-colors hover:bg-sky-50" onClick={startReplay} type="button">
              <Play size={16} />
              Memory Replay
            </button>
          </div>
        </div>
      </section>

      {message ? <div className="rounded-lg border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900">{message}</div> : null}
      {error ? <div className="rounded-lg border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">{error}</div> : null}

      {showWorkspaceForm ? <Panel>
        <SectionHeading title="Create workspace" description="A workspace keeps one project timeline separate from the rest of the vault." />
        <form className="grid gap-3 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]" onSubmit={(event) => void createWorkspace(event)}>
          <label className="grid gap-1.5 text-sm font-medium text-slate-700">Workspace name<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={120} onChange={(event) => setNewWorkspace((current) => ({ ...current, name: event.target.value }))} required value={newWorkspace.name} /></label>
          <label className="grid gap-1.5 text-sm font-medium text-slate-700">Project<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={120} onChange={(event) => setNewWorkspace((current) => ({ ...current, project: event.target.value }))} required value={newWorkspace.project} /></label>
          <button className="mt-auto inline-flex h-10 items-center justify-center gap-2 rounded-md bg-slate-950 px-4 text-sm font-medium text-white hover:bg-slate-800 disabled:cursor-wait disabled:opacity-60" disabled={saving} type="submit"><FolderPlus size={16} />Create</button>
        </form>
      </Panel> : null}

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <WorkspaceMetric label="Workspace events" value={workspace.event_count} supporting={activeSession ? activeSession.title : "No active capture session"} tone="sky" />
        <WorkspaceMetric label="Memories linked" value={workspace.memory_count} supporting="Clipboard and terminal" tone="violet" />
        <WorkspaceMetric label="Error signals" value={workspace.error_count} supporting="Reviewable incidents" tone="rose" />
        <WorkspaceMetric label="Sources" value={workspace.sources.length} supporting={workspace.last_event_at ? `Last event ${workspace.last_event_at}` : "Waiting for first event"} tone="amber" />
      </div>

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_340px]">
        <Panel className="min-w-0">
          <SectionHeading title="Workspace timeline" description="The working sequence behind this project." />
          <div className="grid gap-2 lg:grid-cols-[minmax(0,1fr)_160px_190px]">
            <label className="flex h-10 items-center gap-2 rounded-md border border-slate-200 bg-slate-50 px-3 focus-within:border-sky-400 focus-within:bg-white focus-within:ring-2 focus-within:ring-sky-100">
              <Search className="shrink-0 text-slate-400" size={17} />
              <span className="sr-only">Search workspace events</span>
              <input className="min-w-0 flex-1 bg-transparent text-sm text-slate-900 outline-none placeholder:text-slate-400" onChange={(event) => setQuery(event.target.value)} placeholder="Search workspace events" value={query} />
            </label>
            <select aria-label="Filter workspace event type" className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-700 outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => setEventType(event.target.value as (typeof workspaceEventTypes)[number])} value={eventType}>
              {workspaceEventTypes.map((type) => <option key={type}>{type}</option>)}
            </select>
            <select aria-label="Filter workspace session" className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-700 outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => { setSessionFilter(event.target.value); setReplayIndex(null); setPlaying(false); setHandoffExcludedEventIds([]); setHandoffReadiness(null); }} value={sessionFilter}>
              <option value="All">All sessions</option>
              {sessions.map((session) => <option key={session.id} value={session.id}>{session.status === "Active" ? "Active: " : "Session: "}{session.title}</option>)}
            </select>
          </div>
          <div className="mt-5 grid gap-2">
            {filteredEvents.map((event) => (
              <button className="flex w-full items-start gap-3 rounded-md border border-slate-200 bg-white p-3 text-left transition-colors hover:border-sky-200 hover:bg-sky-50/40" key={event.id} onClick={() => setReplayStep(replayEvents.findIndex((item) => item.id === event.id))} type="button">
                <EventMarker eventType={event.event_type} />
                <span className="min-w-0 flex-1">
                  <span className="flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between sm:gap-3">
                    <span className="truncate text-sm font-semibold text-slate-800">{event.title}</span>
                    <time className="shrink-0 text-xs tabular-nums text-slate-400">{event.created_at}</time>
                  </span>
                  <span className="mt-1 block line-clamp-2 text-sm leading-6 text-slate-600">{event.details}</span>
                  <span className="mt-2 flex flex-wrap items-center gap-2 text-xs text-slate-400">
                    <span className="rounded-md bg-slate-100 px-2 py-1 font-medium text-slate-600">{event.event_type}</span>
                    <span>{event.source_application}</span>
                    {event.incident_id ? <span className="rounded-md bg-rose-50 px-2 py-1 font-medium text-rose-700">Incident</span> : null}
                  </span>
                </span>
              </button>
            ))}
            {filteredEvents.length === 0 ? <div className="grid min-h-48 place-items-center rounded-md border border-dashed border-slate-200 bg-slate-50 p-6 text-center"><div><Workflow className="mx-auto text-slate-300" size={28} /><p className="mt-3 text-sm font-medium text-slate-600">No workspace events match this view.</p></div></div> : null}
          </div>
        </Panel>

        <div className="grid h-fit gap-5">
          <Panel>
            <SectionHeading title="Handoff readiness" description="Review the current workspace scope locally before creating a JSON handoff package." />
            <div className="grid gap-3 border-b border-slate-100 pb-3">
              <label className="grid gap-1.5 text-sm font-medium text-slate-700">Recipient<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" disabled={saving} maxLength={120} onChange={(event) => { setHandoffDeclaration((current) => ({ ...current, recipient: event.target.value })); setHandoffReadiness(null); }} placeholder="e.g. Platform operations" required value={handoffDeclaration.recipient} /></label>
              <label className="grid gap-1.5 text-sm font-medium text-slate-700">Purpose<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" disabled={saving} maxLength={240} onChange={(event) => { setHandoffDeclaration((current) => ({ ...current, purpose: event.target.value })); setHandoffReadiness(null); }} placeholder="e.g. Incident escalation" required value={handoffDeclaration.purpose} /></label>
              <label className="grid gap-1.5 text-sm font-medium text-slate-700">Handling<select className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" disabled={saving} onChange={(event) => { const classification = event.target.value as WorkspaceHandoffRequest["classification"]; setHandoffDeclaration((current) => ({ ...current, classification, expires_in_days: classification === "Confidential" && current.expires_in_days === null ? 7 : current.expires_in_days })); setHandoffReadiness(null); }} value={handoffDeclaration.classification}><option value="Internal">Internal</option><option value="Restricted">Restricted</option><option value="Confidential">Confidential</option></select></label>
              <label className="grid gap-1.5 text-sm font-medium text-slate-700">Expiry<select className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" disabled={saving} onChange={(event) => { setHandoffDeclaration((current) => ({ ...current, expires_in_days: event.target.value === "Never" ? null : Number(event.target.value) })); setHandoffReadiness(null); }} value={handoffDeclaration.expires_in_days ?? "Never"}><option value="1">1 day</option><option value="7">7 days</option><option value="30">30 days</option><option disabled={handoffDeclaration.classification === "Confidential"} value="Never">No expiry</option></select></label>
              <div className={recipientTrustAllowsClassification ? "rounded-md border border-emerald-200 bg-emerald-50 p-3 text-xs leading-5 text-emerald-950" : "rounded-md border border-amber-200 bg-amber-50 p-3 text-xs leading-5 text-amber-950"}>
                <p className="font-semibold">{recipientTrustAllowsClassification ? `Trusted recipient: ${trustedRecipient?.recipient}` : "Recipient approval required"}</p>
                <p className="mt-1">{trustedRecipient?.is_active ? `Approved up to ${trustedRecipient.max_classification}; used ${trustedRecipient.export_count} time(s).` : trustedRecipient ? `Revoked ${trustedRecipient.revoked_at ?? "locally"}; approve again before export.` : "Approve this recipient locally before exporting a signed handoff package."}</p>
                {!recipientTrustAllowsClassification ? <button className="mt-2 inline-flex h-8 w-full items-center justify-center gap-2 rounded-md border border-amber-300 bg-white px-3 text-xs font-semibold text-amber-900 hover:bg-amber-100 disabled:opacity-60" disabled={saving || handoffDeclaration.recipient.trim().length === 0} onClick={() => void trustCurrentHandoffRecipient()} type="button"><CheckCircle2 size={14} />Trust for {handoffDeclaration.classification}</button> : null}
                {trustedRecipient?.is_active ? <button className="mt-2 inline-flex h-8 w-full items-center justify-center gap-2 rounded-md border border-slate-200 bg-white px-3 text-xs font-semibold text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={saving} onClick={() => void revokeCurrentHandoffRecipient()} type="button"><ShieldAlert size={14} />Revoke local trust</button> : null}
              </div>
              {handoffTrustRecords.length > 0 ? <details className="rounded-md border border-slate-200 bg-slate-50 p-3">
                <summary className="cursor-pointer text-xs font-semibold uppercase tracking-[0.14em] text-slate-500">Trusted recipient registry ({handoffTrustRecords.filter((record) => record.is_active).length}/{handoffTrustRecords.length})</summary>
                <div className="mt-2 grid gap-2">
                  {handoffTrustRecords.slice(0, 6).map((record) => <button className={record.is_active ? "w-full rounded-md border border-emerald-100 bg-white p-2 text-left text-xs hover:border-emerald-200 hover:bg-emerald-50" : "w-full rounded-md border border-slate-200 bg-white p-2 text-left text-xs opacity-75 hover:bg-slate-50"} key={record.id} onClick={() => useTrustedRecipient(record)} type="button"><span className="flex items-start justify-between gap-2"><span className="min-w-0 truncate font-semibold text-slate-700">{record.recipient}</span><span className={record.is_active ? "shrink-0 text-emerald-700" : "shrink-0 text-rose-600"}>{record.is_active ? "Active" : "Revoked"}</span></span><span className="mt-1 block text-slate-500">Up to {record.max_classification} - {record.export_count} export(s){record.last_used_at ? ` - Last used ${record.last_used_at}` : ""}</span>{record.revoked_at ? <span className="mt-1 block text-rose-600">Revoked {record.revoked_at}</span> : null}</button>)}
                </div>
              </details> : null}
            </div>
            {handoffScopeEvents.length > 0 ? <details className="mb-3 border-b border-slate-100 pb-3">
              <summary className="cursor-pointer text-xs font-semibold uppercase tracking-[0.14em] text-slate-500">Included timeline events ({handoffScopeEvents.length - handoffExcludedEventIds.length}/{handoffScopeEvents.length})</summary>
              <div className="mt-2 max-h-52 overflow-y-auto">
                {handoffScopeEvents.map((event) => <label className="flex cursor-pointer items-start gap-2 border-b border-slate-100 py-2 last:border-0" key={event.id}>
                  <input aria-label={`Include ${event.title} in handoff`} checked={!handoffExcludedEventIds.includes(event.id)} className="mt-0.5 h-4 w-4 shrink-0 rounded border-slate-300 text-sky-700 focus:ring-sky-500" disabled={saving} onChange={() => toggleHandoffEvent(event.id)} type="checkbox" />
                  <span className="min-w-0"><span className="block truncate text-sm font-medium text-slate-700">{event.title}</span><span className="mt-0.5 block truncate text-xs text-slate-400">{event.event_type} - {event.created_at}</span></span>
                </label>)}
              </div>
            </details> : null}
            <button className="inline-flex h-10 w-full items-center justify-center gap-2 rounded-md border border-slate-200 bg-white px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={saving} onClick={() => void reviewHandoffReadiness()} type="button"><FileCheck2 size={16} />Review current scope</button>
            {handoffReadiness ? <div className={handoffReadiness.safe ? "mt-3 rounded-md border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-950" : "mt-3 rounded-md border border-rose-200 bg-rose-50 p-3 text-sm text-rose-900"}>
              <p className="font-semibold">{handoffReadiness.safe ? "Ready for local handoff export" : "Handoff export blocked"}</p>
              <p className="mt-1 text-xs leading-5">{handoffReadiness.scope} - {handoffReadiness.event_count} events, {handoffReadiness.incident_count} incidents, {handoffReadiness.resolution_count} remedies{handoffReadiness.excluded_event_count > 0 ? `, ${handoffReadiness.excluded_event_count} excluded` : ""}</p>
              <p className="mt-1 text-xs leading-5">Estimated package: {Math.max(1, Math.ceil(handoffReadiness.estimated_bytes / 1024))} KB</p>
              {handoffReadiness.safe ? <button className="mt-3 inline-flex h-9 w-full items-center justify-center gap-2 rounded-md border border-emerald-300 bg-white px-3 text-sm font-medium text-emerald-900 hover:bg-emerald-100 disabled:opacity-60" disabled={saving} onClick={() => void exportHandoff()} type="button"><FileJson size={15} />Export reviewed handoff</button> : <div className="mt-3 grid gap-1 text-xs leading-5 text-rose-800">{handoffReadiness.blockers.map((blocker) => <p key={blocker}>{blocker}</p>)}</div>}
            </div> : null}
            <div className="mt-4 border-t border-slate-100 pt-3">
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Local export audit</p>
              <div className="mt-2 grid gap-2">
                {handoffExports.slice(0, 3).map((entry) => <div className="border-b border-slate-100 pb-2 text-xs last:border-0 last:pb-0" key={entry.id}><div className="flex items-start justify-between gap-2"><p className="min-w-0 truncate font-medium text-slate-700">{entry.scope}</p><time className="shrink-0 text-slate-400">{entry.created_at}</time></div><p className="mt-1 truncate text-slate-500">{entry.classification} - {entry.recipient}</p><p className="mt-1 text-slate-500">{handoffExpiryLabel(entry.expires_at_unix)}</p><p className="mt-1 text-slate-500">Signer {entry.signer_fingerprint}</p><p className="mt-1 text-slate-500">{entry.event_count} events, {entry.incident_count} incidents, {entry.resolution_count} remedies{entry.excluded_event_count > 0 ? `, ${entry.excluded_event_count} excluded` : ""} - {Math.max(1, Math.ceil(entry.package_bytes / 1024))} KB</p><p className="mt-1 font-mono text-[11px] text-slate-400">SHA-256 {entry.package_sha256.slice(0, 16)}</p></div>)}
                {handoffExports.length === 0 ? <p className="text-sm text-slate-500">No local handoff exports recorded.</p> : null}
              </div>
            </div>
          </Panel>

          <Panel>
            <SectionHeading title="Capture session" description="Only the active session receives new timeline events." />
            {workspace.status === "Archived" ? <div className="rounded-md border border-slate-200 bg-slate-50 p-3 text-sm leading-6 text-slate-600">This workspace is archived. Its timeline remains available, but new captures stay paused until you restore it.</div> : workspace.is_imported ? <div className="rounded-md border border-amber-200 bg-amber-50 p-3 text-sm leading-6 text-amber-950">This is a verified imported handoff reference. Its timeline and remedies remain locally reviewable, but new capture sessions are disabled to preserve source fidelity.</div> : activeSession ? <div className="rounded-md border border-emerald-200 bg-emerald-50 p-3"><div className="flex items-start justify-between gap-3"><div className="min-w-0"><p className="truncate text-sm font-semibold text-emerald-950">{activeSession.title}</p><p className="mt-1 text-xs text-emerald-800">Started {activeSession.started_at} - {activeSession.event_count} events</p></div><span className="shrink-0 rounded-md bg-emerald-100 px-2 py-1 text-xs font-semibold text-emerald-800">Active</span></div><button className="mt-3 inline-flex h-9 w-full items-center justify-center gap-2 rounded-md border border-emerald-300 bg-white px-3 text-sm font-medium text-emerald-800 hover:bg-emerald-100 disabled:opacity-60" disabled={saving} onClick={() => void endSession()} type="button"><Square size={15} />End session</button></div> : <form className="grid gap-3" onSubmit={(event) => void startSession(event)}><label className="grid gap-1.5 text-sm font-medium text-slate-700">Session title<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={120} onChange={(event) => setSessionTitle(event.target.value)} placeholder="e.g. Nginx incident review" required value={sessionTitle} /></label><button className="inline-flex h-10 w-full items-center justify-center gap-2 rounded-md bg-emerald-700 px-4 text-sm font-medium text-white hover:bg-emerald-800 disabled:cursor-wait disabled:opacity-60" disabled={saving} type="submit"><Play size={16} />Start capture session</button></form>}
            {sessions.length > 0 ? <div className="mt-4 border-t border-slate-100 pt-3"><p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Recent sessions</p><div className="mt-2 grid gap-2">{sessions.slice(0, 3).map((session) => <div className="flex items-center justify-between gap-3 text-xs" key={session.id}><span className="min-w-0 truncate text-slate-600">{session.title}</span><span className="shrink-0 text-slate-400">{session.event_count} events</span></div>)}</div></div> : null}
          </Panel>

          <Panel>
            <SectionHeading title="Verify handoff package" description="Inspect a selected CYMOS JSON package locally before any future import workflow." />
            <label className="flex min-h-20 cursor-pointer flex-col items-center justify-center gap-2 rounded-md border border-dashed border-slate-300 bg-slate-50 px-4 py-3 text-center transition-colors hover:border-sky-400 hover:bg-sky-50">
              <FileCheck2 className="text-sky-700" size={20} />
              <span className="text-sm font-medium text-slate-700">{inspectingHandoff ? "Verifying package" : "Choose CYMOS JSON package"}</span>
              <input accept="application/json,.json" className="sr-only" disabled={inspectingHandoff} onChange={(event) => void inspectHandoff(event)} type="file" />
            </label>
            {handoffInspection ? <div className={handoffInspection.is_expired ? "mt-3 rounded-md border border-rose-200 bg-rose-50 p-3 text-sm text-rose-900" : "mt-3 rounded-md border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-950"}><p className="font-semibold">{handoffInspection.is_expired ? "Verified but expired" : `Verified: ${handoffInspection.workspace_name}`}</p><p className="mt-1 text-xs leading-5">{handoffInspection.project} - {handoffInspection.scope}</p><p className="mt-2 text-xs leading-5">{handoffInspection.classification} - {handoffInspection.recipient}</p><p className="mt-1 text-xs leading-5">Purpose: {handoffInspection.purpose}</p><p className="mt-1 text-xs leading-5">{handoffExpiryLabel(handoffInspection.expires_at_unix)}</p><p className="mt-1 text-xs leading-5">{handoffInspection.signature_status} - {handoffSignatureLabel(handoffInspection.signature_verified, handoffInspection.signer_fingerprint)}</p>{handoffInspection.signer_fingerprint ? <div className="mt-2 rounded-md border border-white/60 bg-white/70 p-2 text-xs leading-5"><p className="font-semibold text-slate-800">{verifiedSignerTrust?.is_active ? "Signer trusted locally" : verifiedSignerTrust ? "Signer trust revoked" : "Signer not trusted locally"}</p><p className="text-slate-600">{verifiedSignerTrust?.is_active ? `${verifiedSignerTrust.label || verifiedSignerTrust.signer_fingerprint} - ${verifiedSignerTrust.import_count} import(s)` : handoffInspection.classification === "Confidential" ? "Trust this signer before importing confidential packages." : "Trust is optional unless importing confidential packages."}</p>{verifiedSignerTrust?.is_active ? <button className="mt-2 inline-flex h-8 w-full items-center justify-center gap-2 rounded-md border border-slate-200 bg-white px-3 text-xs font-semibold text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={saving} onClick={() => void revokeVerifiedHandoffSigner()} type="button"><ShieldAlert size={14} />Revoke signer trust</button> : <button className="mt-2 inline-flex h-8 w-full items-center justify-center gap-2 rounded-md border border-emerald-300 bg-white px-3 text-xs font-semibold text-emerald-900 hover:bg-emerald-100 disabled:opacity-60" disabled={saving} onClick={() => void trustVerifiedHandoffSigner()} type="button"><CheckCircle2 size={14} />Trust signer</button>}</div> : null}<p className="mt-2 text-xs leading-5">{handoffInspection.event_count} events, {handoffInspection.incident_count} incidents, {handoffInspection.resolution_count} remedies</p><p className="mt-2 break-all text-xs text-slate-600">SHA-256: {handoffInspection.checksum}</p>{handoffInspection.is_expired ? <p className="mt-3 text-xs leading-5">This package cannot be imported.</p> : <button className="mt-3 inline-flex h-9 w-full items-center justify-center gap-2 rounded-md border border-emerald-300 bg-white px-3 text-sm font-medium text-emerald-900 hover:bg-emerald-100 disabled:opacity-60" disabled={importingHandoff || !verifiedHandoffContent} onClick={() => void importHandoff()} type="button"><FileCheck2 size={15} />{importingHandoff ? "Importing verified handoff" : "Import as isolated workspace"}</button>}</div> : null}
            {handoffSignerTrustRecords.length > 0 ? <details className="mt-4 rounded-md border border-slate-200 bg-slate-50 p-3">
              <summary className="cursor-pointer text-xs font-semibold uppercase tracking-[0.14em] text-slate-500">Trusted signer registry ({handoffSignerTrustRecords.filter((record) => record.is_active).length}/{handoffSignerTrustRecords.length})</summary>
              <div className="mt-2 grid gap-2">
                {handoffSignerTrustRecords.slice(0, 6).map((record) => <div className={record.is_active ? "rounded-md border border-emerald-100 bg-white p-2 text-xs" : "rounded-md border border-slate-200 bg-white p-2 text-xs opacity-75"} key={record.id}>
                  <div className="flex items-start justify-between gap-2">
                    <p className="min-w-0 truncate font-semibold text-slate-700">{record.label || "Unlabeled signer"}</p>
                    <span className={record.is_active ? "shrink-0 text-emerald-700" : "shrink-0 text-rose-600"}>{record.is_active ? "Active" : "Revoked"}</span>
                  </div>
                  <p className="mt-1 break-all font-mono text-[11px] text-slate-500">{record.signer_fingerprint}</p>
                  <p className="mt-1 text-slate-500">{record.import_count} import(s){record.last_used_at ? ` - Last used ${record.last_used_at}` : ""}</p>
                  {record.revoked_at ? <p className="mt-1 text-rose-600">Revoked {record.revoked_at}</p> : null}
                </div>)}
              </div>
            </details> : null}
            <div className="mt-4 border-t border-slate-100 pt-3">
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Verification audit</p>
              <div className="mt-2 grid gap-2">
                {handoffInspections.slice(0, 4).map((entry) => <div className="border-b border-slate-100 pb-2 text-xs last:border-0 last:pb-0" key={entry.id}><div className="flex items-start justify-between gap-2"><p className={entry.status === "Rejected" ? "font-semibold text-rose-700" : entry.status === "Expired" ? "font-semibold text-amber-700" : "font-semibold text-emerald-700"}>{entry.status}</p><time className="shrink-0 text-slate-400">{entry.inspected_at}</time></div><p className="mt-1 truncate text-slate-500">{entry.workspace_name ?? "Unreadable package"}{entry.classification ? ` - ${entry.classification}` : ""}</p><p className="mt-1 text-slate-500">{entry.signer_fingerprint ? `Signer ${entry.signer_fingerprint}` : "No signer recorded"} - {Math.max(1, Math.ceil(entry.package_bytes / 1024))} KB</p>{entry.failure_reason ? <p className="mt-1 line-clamp-2 text-rose-600">{entry.failure_reason}</p> : null}<p className="mt-1 font-mono text-[11px] text-slate-400">Package {entry.package_sha256.slice(0, 16)}</p></div>)}
                {handoffInspections.length === 0 ? <p className="text-sm text-slate-500">No handoff packages inspected yet.</p> : null}
              </div>
            </div>
          </Panel>

          <Panel>
            <SectionHeading title="Local file import" description="Explicitly add one text-based config, log, script, or document to this active session." />
            <label className="flex min-h-24 cursor-pointer flex-col items-center justify-center gap-2 rounded-md border border-dashed border-slate-300 bg-slate-50 px-4 py-4 text-center transition-colors hover:border-sky-400 hover:bg-sky-50 disabled:cursor-not-allowed">
              <FileUp className="text-sky-700" size={20} />
              <span className="text-sm font-medium text-slate-700">{importingDocument ? "Importing local file" : "Choose local file"}</span>
              <span className="text-xs leading-5 text-slate-500">Text, logs, configs, scripts, JSON, YAML, or Markdown. Maximum 1 MB.</span>
              <input accept=".txt,.md,.markdown,.log,.conf,.cfg,.ini,.yaml,.yml,.json,.toml,.xml,.csv,.sh,.bash,.zsh,.py,.rs,.ts,.tsx,.js,.jsx,.sql" className="sr-only" disabled={importingDocument || saving || workspace.status === "Archived" || workspace.is_imported || !activeSession} onChange={(event) => void importDocument(event)} type="file" />
            </label>
            {workspace.is_imported ? <p className="mt-3 text-xs leading-5 text-slate-500">Imported references do not accept new files.</p> : workspace.status !== "Archived" && !activeSession ? <p className="mt-3 text-xs leading-5 text-slate-500">Start a capture session to import a file into this workspace timeline.</p> : null}
          </Panel>

          <Panel>
            <SectionHeading title="Project memory" description="The durable context for this workspace." />
            {workspace.is_imported ? <div className="rounded-md border border-amber-200 bg-amber-50 p-3 text-sm leading-6 text-amber-950">The imported name and project are preserved as received. Create a new workspace to add your own project context.</div> : <form className="grid gap-3" onSubmit={(event) => void saveContext(event)}>
              <label className="grid gap-1.5 text-sm font-medium text-slate-700">
                Workspace name
                <input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={120} onChange={(event) => setDraft((current) => ({ ...current, name: event.target.value }))} required value={draft.name} />
              </label>
              <label className="grid gap-1.5 text-sm font-medium text-slate-700">
                Project
                <input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={120} onChange={(event) => setDraft((current) => ({ ...current, project: event.target.value }))} required value={draft.project} />
              </label>
              <button className="inline-flex h-10 w-full items-center justify-center gap-2 rounded-md bg-slate-950 px-4 text-sm font-medium text-white hover:bg-slate-800 disabled:cursor-wait disabled:opacity-60" disabled={saving} type="submit"><Save size={16} />{saving ? "Saving context" : "Save context"}</button>
            </form>}
            <div className="mt-5 border-t border-slate-100 pt-4">
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Local pattern signal</p>
              <p className="mt-2 text-sm leading-6 text-slate-600">{workspace.next_signal}</p>
            </div>
          </Panel>

          <Panel>
            <SectionHeading title="Connected context" description="Topics and sources currently present." />
            <div className="flex flex-wrap gap-2">
              {workspace.top_topics.map((topic) => <span className="rounded-md bg-sky-50 px-2.5 py-1.5 text-xs font-medium text-sky-700" key={topic}>{topic}</span>)}
              {workspace.top_topics.length === 0 ? <span className="text-sm text-slate-500">No topics extracted yet.</span> : null}
            </div>
            <div className="mt-4 flex flex-wrap gap-2 border-t border-slate-100 pt-4">
              {workspace.sources.map((source) => <span className="rounded-md border border-slate-200 px-2.5 py-1.5 text-xs font-medium text-slate-600" key={source}>{source}</span>)}
              {workspace.sources.length === 0 ? <span className="text-sm text-slate-500">No sources recorded yet.</span> : null}
            </div>
          </Panel>

          {snapshot.import_provenance ? <Panel>
            <SectionHeading title="Handoff provenance" description="Source evidence retained for this read-only imported reference." />
            <div className="grid gap-2 text-sm text-slate-700">
              <p><span className="font-semibold text-slate-900">Source:</span> {snapshot.import_provenance.source_workspace}</p>
              <p><span className="font-semibold text-slate-900">Project:</span> {snapshot.import_provenance.source_project}</p>
              <p><span className="font-semibold text-slate-900">Scope:</span> {snapshot.import_provenance.source_scope}</p>
              <p><span className="font-semibold text-slate-900">Recipient:</span> {snapshot.import_provenance.source_recipient}</p>
              <p><span className="font-semibold text-slate-900">Purpose:</span> {snapshot.import_provenance.source_purpose}</p>
              <p><span className="font-semibold text-slate-900">Handling:</span> {snapshot.import_provenance.source_classification}</p>
              <p><span className="font-semibold text-slate-900">Expiry:</span> {handoffExpiryLabel(snapshot.import_provenance.source_expires_at_unix)}</p>
              <p><span className="font-semibold text-slate-900">Signer:</span> {snapshot.import_provenance.source_signer_fingerprint ?? "Unsigned legacy package"}</p>
              <p><span className="font-semibold text-slate-900">Generated:</span> {snapshot.import_provenance.source_generated_at}</p>
              <p><span className="font-semibold text-slate-900">Imported:</span> {snapshot.import_provenance.imported_at}</p>
              <p className="break-all text-xs leading-5 text-slate-500">SHA-256: {snapshot.import_provenance.checksum}</p>
            </div>
          </Panel> : null}

          <Panel>
            <SectionHeading title="Local runbook" description="Search fixes recorded anywhere in this local CYMOS vault." />
            <form className="flex gap-2" onSubmit={(event) => void searchRunbooks(event)}>
              <label className="flex h-10 min-w-0 flex-1 items-center gap-2 rounded-md border border-slate-200 bg-slate-50 px-3 focus-within:border-sky-400 focus-within:bg-white focus-within:ring-2 focus-within:ring-sky-100"><Search className="shrink-0 text-slate-400" size={16} /><span className="sr-only">Search local runbook</span><input className="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-slate-400" maxLength={512} onChange={(event) => setRunbookQuery(event.target.value)} placeholder="Search fixes" value={runbookQuery} /></label>
              <button aria-label="Search local runbook" className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-slate-950 text-white hover:bg-slate-800 disabled:opacity-60" disabled={loadingRunbook} title="Search local runbook" type="submit"><BookOpen size={16} /></button>
              <button aria-label="Create local runbook" className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-slate-200 bg-white text-slate-700 hover:bg-slate-50" onClick={() => { setEditingRunbookId(null); setRunbookDraft({ title: "", details: "", tags: "" }); setShowRunbookForm((current) => !current); }} title="Create local runbook" type="button"><Plus size={17} /></button>
            </form>
            <select aria-label="Filter runbooks by review status" className="mt-2 h-9 w-full rounded-md border border-slate-200 bg-white px-2 text-sm text-slate-700 outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => setRunbookReviewStatus(event.target.value as RunbookSearchRequest["review_status"])} value={runbookReviewStatus}>
              <option value="All">All runbooks</option>
              <option value="Needs review">Needs review</option>
              <option value="Review due">Review due</option>
              <option value="Reviewed">Reviewed</option>
            </select>
            {showRunbookForm ? <form className="mt-3 grid gap-3 border-t border-slate-100 pt-3" onSubmit={(event) => void createRunbook(event)}><label className="grid gap-1.5 text-sm font-medium text-slate-700">Runbook title<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={120} onChange={(event) => setRunbookDraft((current) => ({ ...current, title: event.target.value }))} placeholder="e.g. Restart PostgreSQL safely" required value={runbookDraft.title} /></label><label className="grid gap-1.5 text-sm font-medium text-slate-700">Steps or notes<textarea className="min-h-20 rounded-md border border-slate-200 px-3 py-2 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={1500} onChange={(event) => setRunbookDraft((current) => ({ ...current, details: event.target.value }))} placeholder="Document the commands, checks, and expected outcome" required value={runbookDraft.details} /></label><label className="grid gap-1.5 text-sm font-medium text-slate-700">Tags<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={240} onChange={(event) => setRunbookDraft((current) => ({ ...current, tags: event.target.value }))} placeholder="postgresql, maintenance" value={runbookDraft.tags} /></label><button className="inline-flex h-10 items-center justify-center gap-2 rounded-md bg-slate-950 px-4 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60" disabled={savingRunbook} type="submit"><Save size={16} />{savingRunbook ? "Saving runbook" : editingRunbookId === null ? "Save runbook" : "Update runbook"}</button></form> : null}
            <div className="mt-4 grid gap-2">
              {runbookEntries.slice(0, 3).map((entry) => <div className="rounded-md border border-slate-200 bg-slate-50 p-3" key={entry.id}>
                <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                  <p className="min-w-0 text-sm font-semibold text-slate-800">{entry.title}</p>
                  <div className="flex flex-wrap gap-1 sm:shrink-0 sm:justify-end">
                    <button aria-label={`Copy ${entry.title}`} className="inline-flex h-7 w-7 items-center justify-center rounded-md text-slate-500 hover:bg-white hover:text-slate-900" disabled={copyingRunbookId === entry.id} onClick={() => void copyRunbook(entry)} title="Copy procedure to clipboard" type="button"><Copy size={14} /></button>
                    <button aria-label={`Export ${entry.title}`} className="inline-flex h-7 w-7 items-center justify-center rounded-md text-slate-500 hover:bg-white hover:text-slate-900" disabled={exportingRunbookId === entry.id} onClick={() => void exportRunbook(entry)} title="Export local Markdown runbook" type="button"><Download size={14} /></button>
                    {entry.incident_id === null ? <>
                      <button aria-label={`Mark ${entry.title} reviewed`} className="inline-flex h-7 w-7 items-center justify-center rounded-md text-slate-500 hover:bg-emerald-50 hover:text-emerald-700 disabled:opacity-60" disabled={reviewingRunbookId === entry.id || entry.review_status === "Reviewed"} onClick={() => void reviewRunbook(entry)} title="Mark latest revision reviewed" type="button"><CheckCircle2 size={14} /></button>
                      <button aria-label={`View revision history for ${entry.title}`} className="inline-flex h-7 w-7 items-center justify-center rounded-md text-slate-500 hover:bg-white hover:text-slate-900" disabled={loadingRevisions} onClick={() => void toggleRunbookRevisions(entry)} title="View revision history" type="button"><Clock3 size={14} /></button>
                      <button aria-label={`Edit ${entry.title}`} className="inline-flex h-7 w-7 items-center justify-center rounded-md text-slate-500 hover:bg-white hover:text-slate-900" onClick={() => beginRunbookEdit(entry)} title="Edit local runbook" type="button"><Pencil size={14} /></button>
                      <button aria-label={`Delete ${entry.title}`} className="inline-flex h-7 w-7 items-center justify-center rounded-md text-slate-500 hover:bg-rose-50 hover:text-rose-700" disabled={savingRunbook} onClick={() => void deleteRunbook(entry)} title="Delete local runbook" type="button"><Trash2 size={14} /></button>
                    </> : null}
                  </div>
                </div>
                <p className="mt-1 line-clamp-2 text-xs leading-5 text-slate-600">{entry.details}</p>
                <p className="mt-2 text-xs text-slate-400">{entry.incident_title} - {entry.workspace_name}{entry.tags.length > 0 ? ` - ${entry.tags.join(", ")}` : ""}</p>
                {entry.incident_id === null ? <>
                  <p className={entry.review_status === "Reviewed" ? "mt-2 text-xs text-emerald-700" : entry.review_status === "Review due" ? "mt-2 text-xs text-rose-700" : "mt-2 text-xs text-amber-700"}>{entry.review_status === "Reviewed" ? `Reviewed version ${entry.latest_revision}${entry.last_reviewed_at ? ` - ${entry.last_reviewed_at}` : ""}` : entry.review_status === "Review due" ? `Review due for version ${entry.latest_revision}` : `Review required for version ${entry.latest_revision}`}</p>
                  {entry.last_review_note ? <p className="mt-1 text-xs leading-5 text-slate-500">Review evidence: {entry.last_review_note}</p> : null}
                </> : null}
                {revisionsRunbookId === entry.id ? <div className="mt-3 grid gap-2 border-t border-slate-200 pt-3">{runbookRevisions.map((revision) => <div className="rounded-md bg-white p-2 text-xs text-slate-600" key={revision.id}><div className="flex items-start justify-between gap-2"><p className="font-semibold text-slate-800">Version {revision.revision} - {revision.created_at}</p><button aria-label={`Restore version ${revision.revision}`} className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-slate-500 hover:bg-slate-100 hover:text-slate-900 disabled:opacity-60" disabled={restoringRevisionId === revision.id} onClick={() => void restoreRunbookRevision(entry, revision)} title="Restore as latest revision" type="button"><RotateCcw size={14} /></button></div><p className="mt-1 line-clamp-2 leading-5">{revision.details}</p></div>)}</div> : null}
              </div>)}
              {runbookEntries.length === 0 ? <p className="text-sm text-slate-500">{runbookReviewStatus === "Needs review" ? "No runbooks currently need review." : "Search recorded remedies by incident, command, outcome, or workspace."}</p> : null}
            </div>
            <div className="mt-5 border-t border-slate-100 pt-4">
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Runbook audit trail</p>
              <div className="mt-2 grid gap-2">
                {runbookAuditLogs.slice(0, 4).map((log) => <div className="flex items-start justify-between gap-3 text-xs" key={log.id}><p className="min-w-0 truncate text-slate-600"><span className="font-medium text-slate-800">{log.action.replace("runbook.", "")}</span> - {log.resource}</p><span className="shrink-0 text-slate-400">{log.created_at}</span></div>)}
                {runbookAuditLogs.length === 0 ? <p className="text-sm text-slate-500">No runbook audit activity yet.</p> : null}
              </div>
            </div>
          </Panel>
        </div>
      </div>

      {replayEvent ? <ReplayPanel event={replayEvent} index={replayIndex ?? 0} total={replayEvents.length} playing={playing} onNext={() => setReplayStep((replayIndex ?? 0) + 1)} onPrevious={() => setReplayStep((replayIndex ?? 0) - 1)} onToggle={() => setPlaying((current) => !current)} /> : null}

      <Panel>
        <SectionHeading title="Incident memory" description="Known failure signals and reusable local remedies." />
        <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {incidents.slice(0, 3).map((incident) => {
            const resolutionHistory = snapshot.resolutions.filter((resolution) => resolution.incident_id === incident.id);
            const latestResolution = resolutionHistory[0];
            const evidenceCount = events.filter((event) => event.incident_id === incident.id && event.memory_id !== null).length;
            return <div className="rounded-md border border-slate-200 bg-slate-50 p-3" key={incident.id}><div className="flex items-start gap-2"><ShieldAlert className={incident.status === "Open" ? "mt-0.5 shrink-0 text-rose-600" : "mt-0.5 shrink-0 text-emerald-600"} size={17} /><div className="min-w-0"><p className="text-sm font-semibold capitalize text-slate-800">{incident.title}</p><p className="mt-1 text-xs leading-5 text-slate-500">{incident.event_count} signals - {evidenceCount} linked evidence - {incident.status}</p>{latestResolution ? <div className="mt-3 border-t border-slate-200 pt-3"><p className="flex items-center gap-1.5 text-xs font-semibold text-emerald-800"><CheckCircle2 size={14} />{latestResolution.title}</p><p className="mt-1 line-clamp-2 text-xs leading-5 text-slate-600">{latestResolution.details}</p><p className="mt-1 text-xs text-slate-400">Known remedy from {latestResolution.workspace_name} - {latestResolution.created_at}</p>{resolutionHistory.length > 1 ? <p className="mt-2 text-xs font-medium text-slate-500">{resolutionHistory.length} recorded remedies in local history</p> : null}</div> : null}</div></div></div>;
          })}
          {incidents.length === 0 ? <p className="text-sm text-slate-500">No incident signals in this workspace.</p> : null}
        </div>
        {!workspace.is_imported && workspace.status !== "Archived" && incidents.length > 0 && linkableEvents.length > 0 ? <form className="mt-5 grid gap-3 border-t border-slate-100 pt-5 lg:grid-cols-[minmax(0,220px)_minmax(0,1fr)_auto]" onSubmit={(event) => void linkIncidentEvidence(event)}>
          <label className="grid gap-1.5 text-sm font-medium text-slate-700">Incident<select className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => setEvidenceDraft((current) => ({ ...current, incidentId: event.target.value }))} required value={evidenceDraft.incidentId}><option value="">Select incident</option>{incidents.map((incident) => <option key={incident.id} value={incident.id}>{incident.title}</option>)}</select></label>
          <label className="grid gap-1.5 text-sm font-medium text-slate-700">Saved evidence<select className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => setEvidenceDraft((current) => ({ ...current, eventId: event.target.value }))} required value={evidenceDraft.eventId}><option value="">Select captured memory</option>{linkableEvents.map((event) => <option key={event.id} value={event.id}>{event.event_type} - {event.details} ({event.created_at})</option>)}</select></label>
          <button className="mt-auto inline-flex h-10 items-center justify-center gap-2 rounded-md border border-sky-200 bg-sky-50 px-4 text-sm font-medium text-sky-900 hover:bg-sky-100 disabled:cursor-wait disabled:opacity-60" disabled={saving} type="submit"><Link2 size={16} />Link evidence</button>
        </form> : null}
        {!workspace.is_imported && incidents.some((incident) => incident.status === "Open") ? <form className="mt-5 grid gap-3 border-t border-slate-100 pt-5 lg:grid-cols-[minmax(0,180px)_minmax(0,1fr)_auto]" onSubmit={(event) => void recordResolution(event)}>
          <label className="grid gap-1.5 text-sm font-medium text-slate-700">Incident<select className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => setResolutionDraft((current) => ({ ...current, incidentId: event.target.value }))} required value={resolutionDraft.incidentId}><option value="">Select incident</option>{incidents.filter((incident) => incident.status === "Open").map((incident) => <option key={incident.id} value={incident.id}>{incident.title}</option>)}</select></label>
          <div className="grid gap-3 sm:grid-cols-2"><label className="grid gap-1.5 text-sm font-medium text-slate-700">Fix or command<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={120} onChange={(event) => setResolutionDraft((current) => ({ ...current, title: event.target.value }))} placeholder="e.g. restorecon -Rv /var/www" required value={resolutionDraft.title} /></label><label className="grid gap-1.5 text-sm font-medium text-slate-700">Outcome<textarea className="min-h-10 rounded-md border border-slate-200 px-3 py-2 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={1500} onChange={(event) => setResolutionDraft((current) => ({ ...current, details: event.target.value }))} placeholder="What changed and why it resolved the issue" required value={resolutionDraft.details} /></label></div>
          <button className="mt-auto inline-flex h-10 items-center justify-center gap-2 rounded-md bg-emerald-700 px-4 text-sm font-medium text-white hover:bg-emerald-800 disabled:cursor-wait disabled:opacity-60" disabled={saving} type="submit"><CheckCircle2 size={16} />Record fix</button>
        </form> : null}
        {!workspace.is_imported && incidents.some((incident) => incident.status === "Resolved") ? <form className="mt-5 grid gap-3 border-t border-slate-100 pt-5 lg:grid-cols-[minmax(0,220px)_minmax(0,1fr)_auto]" onSubmit={(event) => void reopenIncident(event)}>
          <label className="grid gap-1.5 text-sm font-medium text-slate-700">Resolved incident<select className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => setReopenDraft((current) => ({ ...current, incidentId: event.target.value }))} required value={reopenDraft.incidentId}><option value="">Select incident</option>{incidents.filter((incident) => incident.status === "Resolved").map((incident) => <option key={incident.id} value={incident.id}>{incident.title}</option>)}</select></label>
          <label className="grid gap-1.5 text-sm font-medium text-slate-700">Follow-up reason<textarea className="min-h-10 rounded-md border border-slate-200 px-3 py-2 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={1500} onChange={(event) => setReopenDraft((current) => ({ ...current, reason: event.target.value }))} placeholder="What returned, changed, or still needs investigation?" required value={reopenDraft.reason} /></label>
          <button className="mt-auto inline-flex h-10 items-center justify-center gap-2 rounded-md border border-amber-300 bg-amber-50 px-4 text-sm font-medium text-amber-900 hover:bg-amber-100 disabled:cursor-wait disabled:opacity-60" disabled={saving || !activeSession || workspace.status === "Archived"} type="submit"><RotateCcw size={16} />Reopen incident</button>
        </form> : null}
      </Panel>

      <p className="flex items-start gap-2 text-xs leading-5 text-slate-500"><Sparkles className="mt-0.5 shrink-0 text-sky-600" size={14} /> Workspace Replay uses the local event journal. It does not record the desktop or infer activity from apps that have not been explicitly connected.</p>
    </div>
  );
}

function ReplayPanel({ event, index, total, playing, onNext, onPrevious, onToggle }: { event: InsightTrailEvent; index: number; total: number; playing: boolean; onNext: () => void; onPrevious: () => void; onToggle: () => void }) {
  const screenshotUrl = event.screenshot_path ? convertFileSrc(event.screenshot_path) : null;
  return (
    <Panel className="border-slate-300 bg-slate-50">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.14em] text-sky-700"><CircleDot size={13} /> Memory Replay</div>
          <h2 className="mt-2 text-base font-semibold text-slate-950">Event {index + 1} of {total}</h2>
        </div>
        <div className="flex items-center gap-2">
          <button aria-label="Previous replay event" className="inline-flex h-9 w-9 items-center justify-center rounded-md border border-slate-200 bg-white text-slate-600 hover:bg-slate-50 disabled:opacity-40" disabled={index === 0} onClick={onPrevious} title="Previous replay event" type="button"><ChevronLeft size={16} /></button>
          <button aria-label={playing ? "Pause memory replay" : "Play memory replay"} className="inline-flex h-9 items-center gap-2 rounded-md bg-slate-950 px-3 text-sm font-medium text-white hover:bg-slate-800" onClick={onToggle} type="button">{playing ? <Pause size={15} /> : <Play size={15} />}{playing ? "Pause" : "Play"}</button>
          <button aria-label="Next replay event" className="inline-flex h-9 w-9 items-center justify-center rounded-md border border-slate-200 bg-white text-slate-600 hover:bg-slate-50 disabled:opacity-40" disabled={index >= total - 1} onClick={onNext} title="Next replay event" type="button"><ChevronRight size={16} /></button>
        </div>
      </div>
      <div className="mt-4 rounded-md border border-slate-200 bg-white p-4">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between"><div className="min-w-0"><p className="text-sm font-semibold text-slate-800">{event.title}</p><p className="mt-1 text-sm leading-6 text-slate-600">{event.details}</p></div><time className="shrink-0 text-xs tabular-nums text-slate-400">{event.created_at}</time></div>
        <div className="mt-3 flex flex-wrap items-center gap-2 text-xs"><span className="rounded-md bg-slate-100 px-2 py-1 font-medium text-slate-600">{event.event_type}</span><span className="inline-flex items-center gap-1 text-slate-500"><Clock3 size={13} />{event.source_application}</span></div>
        {screenshotUrl ? <img alt="Replay event capture" className="mt-4 max-h-72 w-full rounded-md border border-slate-100 bg-slate-50 object-contain" src={screenshotUrl} /> : null}
      </div>
    </Panel>
  );
}

function EventMarker({ eventType }: { eventType: InsightTrailEvent["event_type"] }) {
  const tone = eventType === "Error" ? "bg-rose-50 text-rose-700" : eventType === "Terminal" ? "bg-violet-50 text-violet-700" : eventType === "Screenshot" ? "bg-sky-50 text-sky-700" : "bg-slate-100 text-slate-700";
  return <span className={`grid h-8 w-8 shrink-0 place-items-center rounded-md ${tone}`}><CircleDot size={15} /></span>;
}

function WorkspaceMetric({ label, value, supporting, tone }: { label: string; value: number; supporting: string; tone: "sky" | "violet" | "rose" | "amber" }) {
  const tones = { sky: "border-sky-100 bg-sky-50", violet: "border-violet-100 bg-violet-50", rose: "border-rose-100 bg-rose-50", amber: "border-amber-100 bg-amber-50" };
  return <div className={`rounded-lg border p-4 ${tones[tone]}`}><p className="text-xs font-medium text-slate-500">{label}</p><p className="mt-2 text-xl font-semibold tabular-nums text-slate-950">{value}</p><p className="mt-1 truncate text-xs text-slate-500">{supporting}</p></div>;
}
