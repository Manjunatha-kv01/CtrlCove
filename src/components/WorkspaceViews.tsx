import {
  Activity,
  ArrowUpRight,
  Bot,
  BrainCircuit,
  Braces,
  CheckCircle2,
  CircleDot,
  Clock3,
  CloudCog,
  Copy,
  Database,
  Download,
  FileText,
  FolderPlus,
  Gauge,
  Globe2,
  Grid2X2,
  Link2,
  Network,
  RefreshCw,
  Search,
  ShieldCheck,
  ShieldAlert,
  Sparkles,
  Terminal,
  Workflow,
  Zap
} from "lucide-react";
import { FormEvent, useEffect, useState } from "react";
import type { ReactNode } from "react";
import ClipboardList from "./ClipboardList";
import { Panel, SectionHeading } from "./AppShell";
import type {
  AgentWorkflow,
  AgentWorkflowRecord,
  ApiClient,
  AuditLog,
  AssistantResponse,
  AutomationRunResult,
  AutomationTask,
  BrowserBookmarkRequest,
  ClipboardItem,
  ClipboardStats,
  CognitiveModule,
  CognitiveOverview,
  CognitiveReleaseResult,
  CognitiveUseCase,
  Collection,
  DatabaseBackup,
  DatabaseBackupSnapshot,
  DatabaseBackupVerificationRequest,
  DatabaseReliabilityChecksum,
  DatabaseReliabilityReportExport,
  DatabaseReliabilityReportSnapshot,
  DatabaseReliabilityStatus,
  EnterpriseControl,
  IntelligenceReport,
  IdeSnippetRequest,
  IntegrationConnector,
  KnowledgeDigest,
  KnowledgeGraph,
  KnowledgeHealth,
  LocalExport,
  PlatformSummary,
  PluginRecord,
  PrivacySettings,
  PrivacyStatus,
  HandoffSignerTrustRecord,
  SmartNotification,
  SyncDevice,
  TeamSharingDeviceRequest,
  TeamSharingDeviceStatusRequest,
  TeamSharingManifestInspection,
  TeamSharingManifestLedgerChecksum,
  TeamSharingManifestLedgerExportRequest,
  TeamSharingManifestInspectionRequest,
  TeamSharingPolicy,
  TeamSharingReadiness,
  TeamSharingSyncDryRun,
  TerminalCommandRequest,
  TerminalHistoryImportResult,
  UniversalSyncResult,
  VaultRetentionResult,
  VaultRetentionSettings
} from "../types/cymos";

const IDE_SNIPPET_LANGUAGES = [
  "Auto", "Bash", "C/C++", "CSS", "HTML", "Java", "JavaScript", "JSON", "Python", "Rust", "SQL", "TypeScript", "YAML"
];
type ManifestLedgerFilter = "All" | "Verified" | "Warnings" | "Exports" | "FilteredExports";
const MANIFEST_LEDGER_FILTER_OPTIONS: Array<{ label: string; value: ManifestLedgerFilter }> = [
  { label: "All", value: "All" },
  { label: "Verified", value: "Verified" },
  { label: "Warnings", value: "Warnings" },
  { label: "Exports", value: "Exports" },
  { label: "Filtered exports", value: "FilteredExports" }
];

export function OverviewView({
  overview,
  health,
  stats,
  dailySummary,
  weeklyReport,
  graph,
  items,
  onOpenMemory,
  onOpenAssistant,
  onRunReleaseCheck
}: {
  overview: CognitiveOverview;
  health: KnowledgeHealth;
  stats: ClipboardStats;
  dailySummary: KnowledgeDigest | null;
  weeklyReport: KnowledgeDigest | null;
  graph: KnowledgeGraph;
  items: ClipboardItem[];
  onOpenMemory: () => void;
  onOpenAssistant: () => void;
  onRunReleaseCheck: () => Promise<CognitiveReleaseResult>;
}) {
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState<CognitiveReleaseResult | null>(null);

  async function verifyRelease() {
    setChecking(true);
    try {
      setResult(await onRunReleaseCheck());
    } finally {
      setChecking(false);
    }
  }

  return (
    <div className="grid gap-5">
      <section className="glass-hero overflow-hidden rounded-lg px-5 py-6 text-white sm:px-7 sm:py-8">
        <div className="flex flex-col gap-8 lg:flex-row lg:items-end lg:justify-between">
          <div className="max-w-2xl">
            <div className="inline-flex items-center gap-2 rounded-md border border-white/15 bg-white/10 px-2.5 py-1 text-xs font-medium text-sky-100">
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
              Capture service is active
            </div>
            <h2 className="mt-5 text-2xl font-semibold tracking-tight sm:text-3xl">A clearer view of what you know.</h2>
            <p className="mt-3 text-sm leading-6 text-slate-300">{overview.tagline}</p>
          </div>
          <div className="flex flex-wrap gap-2">
            <button className="inline-flex h-10 items-center gap-2 rounded-md bg-white px-4 text-sm font-medium text-slate-950 transition-colors hover:bg-sky-50" onClick={onOpenMemory} type="button">
              <Search size={16} />
              Browse memory
            </button>
            <button className="inline-flex h-10 items-center gap-2 rounded-md border border-white/20 px-4 text-sm font-medium text-white transition-colors hover:bg-white/10" onClick={onOpenAssistant} type="button">
              <BrainCircuit size={16} />
              Ask CYMOS
            </button>
          </div>
        </div>

        <div className="mt-8 grid gap-4 border-t border-white/10 pt-5 sm:grid-cols-3">
          <HeroMetric icon={<Database size={17} />} label="Saved memories" value={stats.total_items} />
          <HeroMetric icon={<Network size={17} />} label="Connected entities" value={health.connected_entities} />
          <HeroMetric icon={<Gauge size={17} />} label="Memory health" value={`${health.productivity_score}/100`} />
        </div>
      </section>

      {result ? (
        <div className="flex items-start gap-3 rounded-lg border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-950">
          <CheckCircle2 className="mt-0.5 shrink-0 text-emerald-600" size={17} />
          <span>{result.status}. {result.modules_verified} modules, {result.controls_verified} controls, and {result.use_cases_verified} use cases verified.</span>
        </div>
      ) : null}

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <Metric label="Memory score" value={`${overview.memory_score}/100`} supporting="System readiness" tone="sky" />
        <Metric label="Active capture" value={stats.total_items} supporting={`${stats.code_items} code, ${stats.url_items} links`} tone="violet" />
        <Metric label="Knowledge graph" value={graph.nodes.length} supporting={`${graph.edges.length} relationships`} tone="emerald" />
        <Metric label="Privacy mode" value={overview.privacy_mode} supporting="Your data stays yours" tone="amber" />
      </div>

      <div className="grid gap-5 xl:grid-cols-[1.25fr_0.75fr]">
        <Panel>
          <SectionHeading
            title="Recent capture"
            description="The latest items in your local vault."
            action={<button className="inline-flex items-center gap-1 text-sm font-medium text-sky-700 hover:text-sky-800" onClick={onOpenMemory} type="button">View all <ArrowUpRight size={15} /></button>}
          />
          <div className="divide-y divide-slate-100">
            {items.slice(0, 5).map((item) => (
              <div key={item.id} className="flex items-center gap-3 py-3 first:pt-0 last:pb-0">
                <MemoryTypeIcon type={item.content_type} />
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium text-slate-800">{previewTitle(item)}</p>
                  <p className="mt-0.5 truncate text-xs text-slate-500">{item.category || item.content_type} - {item.created_at}</p>
                </div>
                <span className="rounded-md bg-slate-100 px-2 py-1 text-xs font-medium text-slate-600">{item.content_type}</span>
              </div>
            ))}
            {items.length === 0 ? <EmptyRow label="Copy something to start building your private memory." /> : null}
          </div>
        </Panel>

        <Panel>
          <SectionHeading title="Knowledge pulse" description="Signals from your memory today." />
          <div className="grid gap-3">
            <Pulse label="Background tasks" value={health.background_tasks} icon={<Activity size={16} />} />
            <Pulse label="Active projects" value={health.active_projects} icon={<Workflow size={16} />} />
            <Pulse label="Unread signals" value={health.unread_notifications} icon={<Sparkles size={16} />} />
          </div>
          <button className="mt-5 inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50" disabled={checking} onClick={() => void verifyRelease()} type="button">
            <ShieldCheck size={16} />
            {checking ? "Checking system" : "Run system check"}
          </button>
        </Panel>
      </div>

      <div className="grid gap-5 xl:grid-cols-2">
        <DigestCard digest={dailySummary} icon={<Sparkles size={17} />} />
        <DigestCard digest={weeklyReport} icon={<ClockIcon />} />
      </div>
    </div>
  );
}

export function MemoryView({
  items,
  collections,
  stats,
  loading,
  error,
  query,
  setQuery,
  typeFilter,
  setTypeFilter,
  tagFilter,
  setTagFilter,
  categoryFilter,
  setCategoryFilter,
  collectionFilter,
  setCollectionFilter,
  favoriteOnly,
  setFavoriteOnly,
  semanticSearch,
  setSemanticSearch,
  tags,
  categories,
  similarItems,
  onChanged,
  onFindSimilar,
  onAddCollection,
  onSaveBrowserBookmark,
  onSaveIdeSnippet,
  onRebuildIndex,
  onRebuildGraph
}: {
  items: ClipboardItem[];
  collections: Collection[];
  stats: ClipboardStats;
  loading: boolean;
  error: string | null;
  query: string;
  setQuery: (value: string) => void;
  typeFilter: string;
  setTypeFilter: (value: string) => void;
  tagFilter: string;
  setTagFilter: (value: string) => void;
  categoryFilter: string;
  setCategoryFilter: (value: string) => void;
  collectionFilter: string;
  setCollectionFilter: (value: string) => void;
  favoriteOnly: boolean;
  setFavoriteOnly: (value: boolean) => void;
  semanticSearch: boolean;
  setSemanticSearch: (value: boolean) => void;
  tags: string[];
  categories: string[];
  similarItems: ClipboardItem[];
  onChanged: () => void;
  onFindSimilar: (itemId: number) => void;
  onAddCollection: () => void;
  onSaveBrowserBookmark: (request: BrowserBookmarkRequest) => Promise<void>;
  onSaveIdeSnippet: (request: IdeSnippetRequest) => Promise<void>;
  onRebuildIndex: () => void;
  onRebuildGraph: () => void;
}) {
  const [showBookmarkForm, setShowBookmarkForm] = useState(false);
  const [bookmarkUrl, setBookmarkUrl] = useState("");
  const [bookmarkTitle, setBookmarkTitle] = useState("");
  const [bookmarkTags, setBookmarkTags] = useState("");
  const [savingBookmark, setSavingBookmark] = useState(false);
  const [bookmarkError, setBookmarkError] = useState<string | null>(null);
  const [showSnippetForm, setShowSnippetForm] = useState(false);
  const [snippetContent, setSnippetContent] = useState("");
  const [snippetTitle, setSnippetTitle] = useState("");
  const [snippetLanguage, setSnippetLanguage] = useState("Auto");
  const [snippetProject, setSnippetProject] = useState("");
  const [snippetFilePath, setSnippetFilePath] = useState("");
  const [snippetTags, setSnippetTags] = useState("");
  const [savingSnippet, setSavingSnippet] = useState(false);
  const [snippetError, setSnippetError] = useState<string | null>(null);

  async function saveBookmark(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSavingBookmark(true);
    setBookmarkError(null);
    try {
      await onSaveBrowserBookmark({
        url: bookmarkUrl,
        title: bookmarkTitle,
        tags: bookmarkTags.split(",").map((tag) => tag.trim()).filter(Boolean)
      });
      setBookmarkUrl("");
      setBookmarkTitle("");
      setBookmarkTags("");
      setShowBookmarkForm(false);
    } catch (cause) {
      setBookmarkError(messageFor(cause));
    } finally {
      setSavingBookmark(false);
    }
  }

  async function saveIdeSnippet(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSavingSnippet(true);
    setSnippetError(null);
    try {
      await onSaveIdeSnippet({
        content: snippetContent,
        title: snippetTitle,
        language: snippetLanguage,
        project: snippetProject,
        file_path: snippetFilePath,
        tags: snippetTags.split(",").map((tag) => tag.trim()).filter(Boolean)
      });
      setSnippetContent("");
      setSnippetTitle("");
      setSnippetLanguage("Auto");
      setSnippetProject("");
      setSnippetFilePath("");
      setSnippetTags("");
      setShowSnippetForm(false);
    } catch (cause) {
      setSnippetError(messageFor(cause));
    } finally {
      setSavingSnippet(false);
    }
  }

  return (
    <div className="grid gap-5">
      <Panel>
        <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
          <div className="min-w-0 flex-1">
            <label className="sr-only" htmlFor="memory-search">Search saved memory</label>
            <div className="flex h-11 items-center gap-2 rounded-md border border-slate-200 bg-slate-50 px-3 transition-shadow focus-within:border-sky-400 focus-within:bg-white focus-within:ring-2 focus-within:ring-sky-100">
              <Search size={18} className="shrink-0 text-slate-400" />
              <input id="memory-search" className="min-w-0 flex-1 bg-transparent text-sm text-slate-900 outline-none placeholder:text-slate-400" placeholder="Search your saved memory" value={query} onChange={(event) => setQuery(event.target.value)} />
              <span className="hidden rounded border border-slate-200 bg-white px-1.5 py-0.5 text-[11px] font-medium text-slate-400 sm:inline">Cmd K</span>
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            <button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50" onClick={onAddCollection} type="button"><FolderPlus size={16} /> Collection</button>
            <button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50" onClick={() => { setShowBookmarkForm((visible) => !visible); setShowSnippetForm(false); }} type="button"><Link2 size={16} /> Browser bookmark</button>
            <button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50" onClick={() => { setShowSnippetForm((visible) => !visible); setShowBookmarkForm(false); }} type="button"><Braces size={16} /> IDE snippet</button>
            <button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50" onClick={onRebuildIndex} type="button"><RefreshCw size={16} /> Reindex</button>
            <button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50" onClick={onRebuildGraph} type="button"><Network size={16} /> Rebuild graph</button>
          </div>
        </div>

        {showBookmarkForm ? (
          <form className="mt-4 grid gap-3 border-t border-slate-100 pt-4" onSubmit={saveBookmark}>
            <div className="grid gap-3 lg:grid-cols-[minmax(0,1.15fr)_minmax(0,0.85fr)_auto] lg:items-end">
              <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="bookmark-url">
                URL
                <input id="bookmark-url" type="url" required value={bookmarkUrl} onChange={(event) => setBookmarkUrl(event.target.value)} placeholder="https://example.com/reference" className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
              </label>
              <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="bookmark-title">
                Title <span className="font-normal text-slate-400">optional</span>
                <input id="bookmark-title" value={bookmarkTitle} onChange={(event) => setBookmarkTitle(event.target.value)} placeholder="What this is for" className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
              </label>
              <div className="flex gap-2 lg:pb-0">
                <button className="inline-flex h-9 items-center gap-2 rounded-md bg-slate-900 px-3 text-sm font-medium text-white hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-60" disabled={savingBookmark} type="submit"><Link2 size={15} /> {savingBookmark ? "Saving" : "Save"}</button>
                <button className="h-9 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-600 hover:bg-slate-50" onClick={() => setShowBookmarkForm(false)} type="button">Cancel</button>
              </div>
            </div>
            <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="bookmark-tags">
              Tags <span className="font-normal text-slate-400">comma separated</span>
              <input id="bookmark-tags" value={bookmarkTags} onChange={(event) => setBookmarkTags(event.target.value)} placeholder="nginx, rhel, operations" className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
            </label>
            {bookmarkError ? <ErrorMessage message={bookmarkError} /> : null}
          </form>
        ) : null}

        {showSnippetForm ? (
          <form className="mt-4 grid gap-3 border-t border-slate-100 pt-4" onSubmit={saveIdeSnippet}>
            <div className="grid gap-3 lg:grid-cols-3">
              <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="snippet-title">
                Title <span className="font-normal text-slate-400">optional</span>
                <input id="snippet-title" value={snippetTitle} onChange={(event) => setSnippetTitle(event.target.value)} placeholder="What this code does" className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
              </label>
              <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="snippet-project">
                Project <span className="font-normal text-slate-400">optional</span>
                <input id="snippet-project" value={snippetProject} onChange={(event) => setSnippetProject(event.target.value)} placeholder="CYMOS" className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
              </label>
              <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="snippet-language">
                Language
                <select id="snippet-language" value={snippetLanguage} onChange={(event) => setSnippetLanguage(event.target.value)} className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-100">
                  {IDE_SNIPPET_LANGUAGES.map((language) => <option key={language}>{language}</option>)}
                </select>
              </label>
            </div>
            <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
              <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="snippet-file-path">
                File path <span className="font-normal text-slate-400">optional</span>
                <input id="snippet-file-path" value={snippetFilePath} onChange={(event) => setSnippetFilePath(event.target.value)} placeholder="src/components/MemoryView.tsx" className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
              </label>
              <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="snippet-tags">
                Tags <span className="font-normal text-slate-400">comma separated</span>
                <input id="snippet-tags" value={snippetTags} onChange={(event) => setSnippetTags(event.target.value)} placeholder="authentication, api, review" className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
              </label>
            </div>
            <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="snippet-content">
              Code
              <textarea id="snippet-content" required value={snippetContent} onChange={(event) => setSnippetContent(event.target.value)} placeholder="Paste the code snippet" className="min-h-36 resize-y rounded-md border border-slate-200 bg-slate-950 p-3 font-mono text-sm leading-6 text-slate-100 outline-none transition placeholder:text-slate-500 focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
            </label>
            <div className="flex flex-wrap gap-2">
              <button className="inline-flex h-9 items-center gap-2 rounded-md bg-slate-900 px-3 text-sm font-medium text-white hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-60" disabled={savingSnippet} type="submit"><Braces size={15} /> {savingSnippet ? "Saving" : "Save snippet"}</button>
              <button className="h-9 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-600 hover:bg-slate-50" onClick={() => setShowSnippetForm(false)} type="button">Cancel</button>
            </div>
            {snippetError ? <ErrorMessage message={snippetError} /> : null}
          </form>
        ) : null}

        <div className="mt-4 grid gap-2 sm:grid-cols-2 xl:grid-cols-5">
          <Select value={typeFilter} onChange={setTypeFilter} label="Content type">
            {(["All", "Text", "Image", "Code", "URL", "File", "Folder", "Color", "Table", "HTML"] as const).map((filter) => <option key={filter}>{filter}</option>)}
          </Select>
          <Select value={collectionFilter} onChange={setCollectionFilter} label="Collection">
            <option value="">All collections</option>
            {collections.map((collection) => <option key={collection.id} value={collection.id}>{collection.name}</option>)}
          </Select>
          <Select value={tagFilter} onChange={setTagFilter} label="Tag">
            <option>All</option>
            {tags.map((tag) => <option key={tag}>{tag}</option>)}
          </Select>
          <Select value={categoryFilter} onChange={setCategoryFilter} label="Category">
            <option>All</option>
            {categories.map((category) => <option key={category}>{category}</option>)}
          </Select>
          <div className="flex h-9 items-center gap-4 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-700">
            <Toggle label="Favorites" value={favoriteOnly} onChange={setFavoriteOnly} />
            <Toggle label="Semantic" value={semanticSearch} onChange={setSemanticSearch} />
          </div>
        </div>
      </Panel>

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_260px]">
        <div className="min-w-0">
          <SectionHeading title={`${items.length} memories`} description={query ? `Results for "${query}"` : "Newest first, saved locally."} />
          {error ? <ErrorMessage message={error} /> : <ClipboardList items={items} collections={collections} loading={loading} query={query.trim()} onChanged={onChanged} onFindSimilar={onFindSimilar} />}
        </div>
        <div className="grid h-fit gap-4">
          <Panel>
            <p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Vault overview</p>
            <div className="mt-4 grid gap-3">
              <SmallStat label="Text" value={stats.text_items} />
              <SmallStat label="Code" value={stats.code_items} />
              <SmallStat label="URLs" value={stats.url_items} />
              <SmallStat label="Favorites" value={stats.favorite_items} />
            </div>
          </Panel>
          <Panel>
            <p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Similar memories</p>
            <div className="mt-3 grid gap-2">
              {similarItems.length > 0 ? similarItems.slice(0, 5).map((item) => (
                <div className="rounded-md bg-slate-50 px-3 py-2" key={item.id}>
                  <p className="truncate text-sm font-medium text-slate-700">{previewTitle(item)}</p>
                  <p className="mt-1 text-xs text-sky-700">{Math.round(item.semantic_score * 100)}% similar</p>
                </div>
              )) : <p className="text-sm leading-6 text-slate-500">Use the sparkle action on a memory to find related items.</p>}
            </div>
          </Panel>
        </div>
      </div>
    </div>
  );
}

export function OperationsView({
  items,
  collections,
  loading,
  query,
  setQuery,
  onChanged,
  onFindSimilar,
  onImportTerminalHistory,
  onSaveTerminalCommand
}: {
  items: ClipboardItem[];
  collections: Collection[];
  loading: boolean;
  query: string;
  setQuery: (value: string) => void;
  onChanged: () => void;
  onFindSimilar: (itemId: number) => void;
  onImportTerminalHistory: (shell: "Bash" | "Zsh", maxEntries: number) => Promise<TerminalHistoryImportResult>;
  onSaveTerminalCommand: (request: TerminalCommandRequest) => Promise<void>;
}) {
  const [shell, setShell] = useState<"Bash" | "Zsh">("Bash");
  const [maxEntries, setMaxEntries] = useState(250);
  const [importing, setImporting] = useState(false);
  const [result, setResult] = useState<TerminalHistoryImportResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showCommandForm, setShowCommandForm] = useState(false);
  const [command, setCommand] = useState("");
  const [host, setHost] = useState("");
  const [project, setProject] = useState("");
  const [commandTags, setCommandTags] = useState("");
  const [savingCommand, setSavingCommand] = useState(false);
  const [commandError, setCommandError] = useState<string | null>(null);
  const hosts = new Set(items.flatMap((item) => item.operational_context.hostnames));
  const addresses = new Set(items.flatMap((item) => item.operational_context.ip_addresses));
  const incidents = items.filter((item) => item.operational_context.kind === "Incident").length;

  async function importHistory() {
    setImporting(true);
    setError(null);
    try {
      setResult(await onImportTerminalHistory(shell, maxEntries));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setImporting(false);
    }
  }

  async function saveTerminalCommand(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSavingCommand(true);
    setCommandError(null);
    try {
      await onSaveTerminalCommand({
        command,
        shell,
        host,
        project,
        tags: commandTags.split(",").map((tag) => tag.trim()).filter(Boolean)
      });
      setCommand("");
      setHost("");
      setProject("");
      setCommandTags("");
      setShowCommandForm(false);
    } catch (cause) {
      setCommandError(messageFor(cause));
    } finally {
      setSavingCommand(false);
    }
  }

  return (
    <div className="grid gap-5">
      <Panel>
        <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
          <div>
            <div className="flex items-center gap-2 text-sm font-medium text-sky-700"><Terminal size={16} /> Local terminal history</div>
            <h2 className="mt-2 text-xl font-semibold text-slate-950">Operational knowledge vault</h2>
          </div>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
            <div className="inline-flex h-10 rounded-md border border-slate-200 bg-slate-50 p-1" aria-label="Terminal shell">
              {(["Bash", "Zsh"] as const).map((option) => (
                <button
                  aria-pressed={shell === option}
                  className={`rounded px-3 text-sm font-medium ${shell === option ? "bg-white text-slate-950 shadow-sm" : "text-slate-500 hover:text-slate-800"}`}
                  key={option}
                  onClick={() => setShell(option)}
                  type="button"
                >
                  {option}
                </button>
              ))}
            </div>
            <label className="sr-only" htmlFor="terminal-import-limit">Terminal history import size</label>
            <select className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm font-medium text-slate-700 outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" id="terminal-import-limit" onChange={(event) => setMaxEntries(Number(event.target.value))} value={maxEntries}>
              {[100, 250, 500, 1000].map((limit) => <option key={limit} value={limit}>Newest {limit}</option>)}
            </select>
            <button className="inline-flex h-10 items-center justify-center gap-2 rounded-md bg-slate-950 px-4 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60" disabled={importing} onClick={() => void importHistory()} type="button">
              <Terminal size={16} />{importing ? "Importing" : "Import history"}
            </button>
            <button className="inline-flex h-10 items-center justify-center gap-2 rounded-md border border-slate-200 px-4 text-sm font-medium text-slate-700 hover:bg-slate-50" onClick={() => setShowCommandForm((visible) => !visible)} type="button">
              <Terminal size={16} /> Capture command
            </button>
          </div>
        </div>

        {showCommandForm ? (
          <form className="mt-4 grid gap-3 border-t border-slate-100 pt-4" onSubmit={saveTerminalCommand}>
            <div className="grid gap-3 lg:grid-cols-3">
              <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="terminal-command-host">
                Host <span className="font-normal text-slate-400">optional</span>
                <input id="terminal-command-host" value={host} onChange={(event) => setHost(event.target.value)} placeholder="web-01.internal" className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
              </label>
              <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="terminal-command-project">
                Project <span className="font-normal text-slate-400">optional</span>
                <input id="terminal-command-project" value={project} onChange={(event) => setProject(event.target.value)} placeholder="edge-platform" className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
              </label>
              <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="terminal-command-tags">
                Tags <span className="font-normal text-slate-400">comma separated</span>
                <input id="terminal-command-tags" value={commandTags} onChange={(event) => setCommandTags(event.target.value)} placeholder="nginx, maintenance" className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
              </label>
            </div>
            <label className="grid gap-1.5 text-xs font-medium text-slate-600" htmlFor="terminal-command-content">
              Command
              <textarea id="terminal-command-content" required value={command} onChange={(event) => setCommand(event.target.value)} placeholder="systemctl restart nginx" className="min-h-28 resize-y rounded-md border border-slate-200 bg-slate-950 p-3 font-mono text-sm leading-6 text-slate-100 outline-none transition placeholder:text-slate-500 focus:border-sky-400 focus:ring-2 focus:ring-sky-100" />
            </label>
            <div className="flex flex-wrap gap-2">
              <button className="inline-flex h-9 items-center gap-2 rounded-md bg-slate-950 px-3 text-sm font-medium text-white hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-60" disabled={savingCommand} type="submit"><Terminal size={15} /> {savingCommand ? "Saving" : "Save command"}</button>
              <button className="h-9 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-600 hover:bg-slate-50" onClick={() => setShowCommandForm(false)} type="button">Cancel</button>
            </div>
            {commandError ? <ErrorMessage message={commandError} /> : null}
          </form>
        ) : null}

        {result ? <div className="mt-4 flex items-start gap-3 rounded-md border border-emerald-100 bg-emerald-50 px-3 py-2.5 text-sm text-emerald-950"><ShieldCheck className="mt-0.5 shrink-0 text-emerald-600" size={16} /><span>{result.imported} of {result.selected} newest eligible {result.shell} commands imported from {result.available} available entries. {result.skipped_sensitive} sensitive and {result.skipped_irrelevant} low-signal entries skipped.</span></div> : null}
        {error ? <div className="mt-4"><ErrorMessage message={error} /></div> : null}

        <div className="mt-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <Metric label="Operational memories" value={items.length} supporting="Commands and context" tone="sky" />
          <Metric label="Incidents" value={incidents} supporting="Errors and denials" tone="rose" />
          <Metric label="Hosts" value={hosts.size} supporting="Captured locally" tone="violet" />
          <Metric label="IP addresses" value={addresses.size} supporting="Network context" tone="amber" />
        </div>
      </Panel>

      <Panel>
        <label className="sr-only" htmlFor="operations-search">Search operational memory</label>
        <div className="flex h-11 items-center gap-2 rounded-md border border-slate-200 bg-slate-50 px-3 transition-shadow focus-within:border-sky-400 focus-within:bg-white focus-within:ring-2 focus-within:ring-sky-100">
          <Search size={18} className="shrink-0 text-slate-400" />
          <input id="operations-search" className="min-w-0 flex-1 bg-transparent text-sm text-slate-900 outline-none placeholder:text-slate-400" placeholder="Search Nginx, SELinux, Kubernetes, host, or IP" value={query} onChange={(event) => setQuery(event.target.value)} />
        </div>
      </Panel>

      <div className="flex items-center gap-2 text-sm text-slate-500"><ShieldAlert size={16} className="text-amber-600" /> Only the selected newest window is imported. Sensitive terminal entries are filtered before import.</div>
      <ClipboardList items={items} collections={collections} loading={loading} query={query.trim()} onChanged={onChanged} onFindSimilar={onFindSimilar} />
    </div>
  );
}

export function AssistantView({ onAsk }: { onAsk: (question: string) => Promise<AssistantResponse> }) {
  const [question, setQuestion] = useState("");
  const [response, setResponse] = useState<AssistantResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!question.trim()) return;
    setLoading(true);
    setError(null);
    try {
      setResponse(await onAsk(question.trim()));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_280px]">
      <Panel className="min-h-[500px]">
        <div className="flex items-start gap-3 border-b border-slate-100 pb-5">
          <span className="grid h-10 w-10 place-items-center rounded-lg bg-sky-50 text-sky-700"><BrainCircuit size={20} /></span>
          <div><h2 className="text-base font-semibold text-slate-950">Memory assistant</h2><p className="mt-1 text-sm text-slate-500">Ask one question. CYMOS retrieves local context before answering.</p></div>
        </div>
        <form className="mt-5 flex flex-col gap-2 sm:flex-row" onSubmit={submit}>
          <input className="h-11 min-w-0 flex-1 rounded-md border border-slate-200 bg-slate-50 px-3 text-sm outline-none focus:border-sky-400 focus:bg-white focus:ring-2 focus:ring-sky-100" placeholder="What did I save about Docker?" value={question} onChange={(event) => setQuestion(event.target.value)} />
          <button className="inline-flex h-11 items-center justify-center gap-2 rounded-md bg-slate-950 px-4 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60" disabled={loading} type="submit"><Sparkles size={16} />{loading ? "Thinking" : "Ask"}</button>
        </form>
        {error ? <div className="mt-4"><ErrorMessage message={error} /></div> : null}
        {response ? <AssistantResponseCard response={response} /> : <AssistantEmpty />}
      </Panel>
      <AssistantPrompts onPick={setQuestion} />
    </div>
  );
}

export function AgentView({ history, onRun }: { history: AgentWorkflowRecord[]; onRun: (goal: string) => Promise<AgentWorkflow> }) {
  const [goal, setGoal] = useState("");
  const [workflow, setWorkflow] = useState<AgentWorkflow | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!goal.trim()) return;
    setLoading(true);
    setError(null);
    try {
      setWorkflow(await onRun(goal.trim()));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_320px]">
      <Panel>
        <SectionHeading title="Agent workspace" description="Give CYMOS a goal and it will organize your local context into a practical plan." />
        <form className="flex flex-col gap-3 sm:flex-row" onSubmit={submit}>
          <input className="h-11 min-w-0 flex-1 rounded-md border border-slate-200 bg-slate-50 px-3 text-sm outline-none focus:border-violet-400 focus:bg-white focus:ring-2 focus:ring-violet-100" placeholder="Prepare a Docker learning roadmap from my saved notes" value={goal} onChange={(event) => setGoal(event.target.value)} />
          <button className="inline-flex h-11 items-center justify-center gap-2 rounded-md bg-violet-700 px-4 text-sm font-medium text-white hover:bg-violet-800 disabled:opacity-60" disabled={loading} type="submit"><Bot size={16} />{loading ? "Planning" : "Run agents"}</button>
        </form>
        {error ? <div className="mt-4"><ErrorMessage message={error} /></div> : null}
        {workflow ? <WorkflowResult workflow={workflow} /> : <AgentEmpty />}
      </Panel>
      <Panel>
        <p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Recent workflows</p>
        <div className="mt-3 grid gap-3">
          {history.length > 0 ? history.slice(0, 8).map((entry) => <HistoryEntry entry={entry} key={entry.id} />) : <p className="text-sm text-slate-500">Your completed agent workflows will appear here.</p>}
        </div>
      </Panel>
    </div>
  );
}

export function AutomationView({
  health,
  tasks,
  notifications,
  reports,
  onRun
}: {
  health: KnowledgeHealth;
  tasks: AutomationTask[];
  notifications: SmartNotification[];
  reports: IntelligenceReport[];
  onRun: () => Promise<AutomationRunResult>;
}) {
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<AutomationRunResult | null>(null);

  async function run() {
    setRunning(true);
    try {
      setResult(await onRun());
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="grid gap-5">
      <Panel>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
          <div><h2 className="text-base font-semibold text-slate-950">Background care</h2><p className="mt-1 text-sm text-slate-500">CYMOS indexes, connects, and safeguards your memory in the background.</p></div>
          <button className="inline-flex h-10 items-center justify-center gap-2 rounded-md bg-sky-700 px-4 text-sm font-medium text-white hover:bg-sky-800 disabled:opacity-60" disabled={running} onClick={() => void run()} type="button"><Zap size={16} />{running ? "Running cycle" : "Run maintenance"}</button>
        </div>
        {result ? <div className="mt-4 rounded-md border border-sky-100 bg-sky-50 px-3 py-2 text-sm text-sky-950">Completed {result.tasks_run} tasks, created {result.reports_created} reports and {result.notifications_created} new signals.</div> : null}
        <div className="mt-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <Metric label="Productivity score" value={`${health.productivity_score}/100`} supporting="Knowledge health" tone="sky" />
          <Metric label="Active projects" value={health.active_projects} supporting="Detected from memory" tone="violet" />
          <Metric label="Background tasks" value={health.background_tasks} supporting="Running in local mode" tone="emerald" />
          <Metric label="Storage health" value={health.storage_health} supporting="SQLite vault" tone="amber" />
        </div>
      </Panel>
      <div className="grid gap-5 xl:grid-cols-3">
        <CompactList title="Scheduled tasks" items={tasks.map((task) => ({ title: task.service, detail: task.details, badge: task.status }))} empty="No automation tasks yet." />
        <CompactList title="Smart signals" items={notifications.map((note) => ({ title: note.message, detail: note.created_at, badge: note.severity }))} empty="No new signals." />
        <CompactList title="Latest reports" items={reports.map((report) => ({ title: report.title, detail: report.summary, badge: report.report_type }))} empty="Reports will appear after a maintenance cycle." />
      </div>
    </div>
  );
}

export function GraphView({ graph, modules, onRebuild }: { graph: KnowledgeGraph; modules: CognitiveModule[]; onRebuild: () => void }) {
  return (
    <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_300px]">
      <Panel>
        <SectionHeading title="Connected topics" description="Relationships inferred from the memories you saved." action={<button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50" onClick={onRebuild} type="button"><RefreshCw size={15} /> Rebuild</button>} />
        <div className="grid min-h-[320px] place-items-center rounded-lg border border-dashed border-sky-200 bg-sky-50/40 p-6">
          {graph.nodes.length > 0 ? <GraphMap graph={graph} /> : <div className="text-center"><Network className="mx-auto text-sky-600" size={30} /><p className="mt-3 text-sm font-medium text-slate-700">Your graph will take shape as you capture more memory.</p></div>}
        </div>
        {graph.recommendations.length > 0 ? <div className="mt-4 rounded-md bg-slate-50 p-3"><p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Suggested next connections</p><ul className="mt-2 grid gap-1.5 text-sm text-slate-600">{graph.recommendations.map((item) => <li className="flex gap-2" key={item}><span className="text-sky-600">-</span>{item}</li>)}</ul></div> : null}
      </Panel>
      <div className="grid h-fit gap-4">
        <CompactList title="Topic clusters" items={graph.clusters.map((cluster) => ({ title: cluster.name, detail: cluster.entities.join(", "), badge: String(cluster.count) }))} empty="No topic clusters yet." />
        <CompactList title="Core modules" items={modules.map((module) => ({ title: module.name, detail: module.layer, badge: module.status }))} empty="No modules registered." />
      </div>
    </div>
  );
}

export function PlatformView({
  platform,
  devices,
  connectors,
  plugins,
  apiClients,
  auditLogs,
  teamSharingAuditLogs,
  manifestLedgerAuditLogs,
  controls,
  useCases,
  onRunSync,
  databaseReliability,
  onCheckDatabaseReliability,
  onGetDatabaseReliabilityChecksum,
  onCreateBackup,
  onVerifyLatestBackup,
  onGetRecentBackups,
  onGetRecentReliabilityReports,
  onVerifyBackupSnapshot,
  onExportDatabaseReliabilityReport,
  privacyStatus,
  onSavePrivacySettings,
  onRegisterTeamSharingDevice,
  onApproveTeamSharingDevice,
  onRevokeTeamSharingDevice,
  onExportTeamSharingReport,
  onExportTeamSharingManifestLedger,
  onExportFilteredTeamSharingManifestLedger,
  onGetTeamSharingManifestLedgerChecksum,
  onRunTeamSharingDryRun,
  onExportTeamSharingDryRunManifest,
  onInspectTeamSharingManifest,
  onTrustCurrentDeviceSigner,
  teamSharingPolicy,
  teamSharingReadiness,
  onSaveTeamSharingPolicy,
  vaultRetentionSettings,
  onSaveVaultRetentionSettings,
  onApplyVaultRetention
}: {
  platform: PlatformSummary;
  devices: SyncDevice[];
  connectors: IntegrationConnector[];
  plugins: PluginRecord[];
  apiClients: ApiClient[];
  auditLogs: AuditLog[];
  teamSharingAuditLogs: AuditLog[];
  manifestLedgerAuditLogs: AuditLog[];
  controls: EnterpriseControl[];
  useCases: CognitiveUseCase[];
  onRunSync: () => Promise<UniversalSyncResult>;
  databaseReliability: DatabaseReliabilityStatus;
  onCheckDatabaseReliability: () => Promise<DatabaseReliabilityStatus>;
  onGetDatabaseReliabilityChecksum: () => Promise<DatabaseReliabilityChecksum>;
  onCreateBackup: () => Promise<DatabaseBackup>;
  onVerifyLatestBackup: () => Promise<DatabaseBackup>;
  onGetRecentBackups: () => Promise<DatabaseBackupSnapshot[]>;
  onGetRecentReliabilityReports: () => Promise<DatabaseReliabilityReportSnapshot[]>;
  onVerifyBackupSnapshot: (request: DatabaseBackupVerificationRequest) => Promise<DatabaseBackup>;
  onExportDatabaseReliabilityReport: () => Promise<DatabaseReliabilityReportExport>;
  privacyStatus: PrivacyStatus;
  onSavePrivacySettings: (settings: PrivacySettings) => Promise<void>;
  onRegisterTeamSharingDevice: (request: TeamSharingDeviceRequest) => Promise<void>;
  onApproveTeamSharingDevice: (request: TeamSharingDeviceStatusRequest) => Promise<void>;
  onRevokeTeamSharingDevice: (request: TeamSharingDeviceStatusRequest) => Promise<void>;
  onExportTeamSharingReport: () => Promise<LocalExport>;
  onExportTeamSharingManifestLedger: () => Promise<LocalExport>;
  onExportFilteredTeamSharingManifestLedger: (request: TeamSharingManifestLedgerExportRequest) => Promise<LocalExport>;
  onGetTeamSharingManifestLedgerChecksum: (request: TeamSharingManifestLedgerExportRequest) => Promise<TeamSharingManifestLedgerChecksum>;
  onRunTeamSharingDryRun: () => Promise<TeamSharingSyncDryRun>;
  onExportTeamSharingDryRunManifest: () => Promise<LocalExport>;
  onInspectTeamSharingManifest: (request: TeamSharingManifestInspectionRequest) => Promise<TeamSharingManifestInspection>;
  onTrustCurrentDeviceSigner: () => Promise<HandoffSignerTrustRecord>;
  teamSharingPolicy: TeamSharingPolicy;
  teamSharingReadiness: TeamSharingReadiness;
  onSaveTeamSharingPolicy: (policy: TeamSharingPolicy) => Promise<void>;
  vaultRetentionSettings: VaultRetentionSettings;
  onSaveVaultRetentionSettings: (settings: VaultRetentionSettings) => Promise<void>;
  onApplyVaultRetention: () => Promise<VaultRetentionResult>;
}) {
  const [syncing, setSyncing] = useState(false);
  const [result, setResult] = useState<UniversalSyncResult | null>(null);
  const [checkingVault, setCheckingVault] = useState(false);
  const [backingUp, setBackingUp] = useState(false);
  const [verifyingBackup, setVerifyingBackup] = useState(false);
  const [verifyingBackupSnapshot, setVerifyingBackupSnapshot] = useState<string | null>(null);
  const [loadingRecentBackups, setLoadingRecentBackups] = useState(false);
  const [loadingReliabilityReports, setLoadingReliabilityReports] = useState(false);
  const [exportingReliabilityReport, setExportingReliabilityReport] = useState(false);
  const [calculatingReliabilityChecksum, setCalculatingReliabilityChecksum] = useState(false);
  const [vaultStatus, setVaultStatus] = useState<DatabaseReliabilityStatus | null>(null);
  const [backup, setBackup] = useState<DatabaseBackup | null>(null);
  const [backupMessage, setBackupMessage] = useState<string | null>(null);
  const [copiedBackupPath, setCopiedBackupPath] = useState(false);
  const [recentBackups, setRecentBackups] = useState<DatabaseBackupSnapshot[] | null>(null);
  const [recentReliabilityReports, setRecentReliabilityReports] = useState<DatabaseReliabilityReportSnapshot[] | null>(null);
  const [sessionVerifiedBackupPaths, setSessionVerifiedBackupPaths] = useState<Set<string>>(() => new Set());
  const [reliabilityChecksum, setReliabilityChecksum] = useState<DatabaseReliabilityChecksum | null>(null);
  const [copiedReliabilityChecksum, setCopiedReliabilityChecksum] = useState(false);
  const [copiedReliabilityReportPath, setCopiedReliabilityReportPath] = useState<string | null>(null);
  const [reliabilityMessage, setReliabilityMessage] = useState<string | null>(null);
  const [vaultError, setVaultError] = useState<string | null>(null);
  const [privacyDraft, setPrivacyDraft] = useState<PrivacySettings>(privacyStatus.settings);
  const [savingPrivacy, setSavingPrivacy] = useState(false);
  const [privacyMessage, setPrivacyMessage] = useState<string | null>(null);
  const [privacyError, setPrivacyError] = useState<string | null>(null);
  const [sharingDraft, setSharingDraft] = useState<TeamSharingPolicy>(teamSharingPolicy);
  const [deviceDraft, setDeviceDraft] = useState<TeamSharingDeviceRequest>({ device_name: "", platform: "macOS", sync_mode: "Local-only" });
  const [savingSharing, setSavingSharing] = useState(false);
  const [savingDevice, setSavingDevice] = useState(false);
  const [exportingSharingReport, setExportingSharingReport] = useState(false);
  const [exportingManifestLedger, setExportingManifestLedger] = useState(false);
  const [exportingFilteredManifestLedger, setExportingFilteredManifestLedger] = useState(false);
  const [calculatingManifestLedgerChecksum, setCalculatingManifestLedgerChecksum] = useState(false);
  const [exportingSharingManifest, setExportingSharingManifest] = useState(false);
  const [runningSharingDryRun, setRunningSharingDryRun] = useState(false);
  const [sharingDryRun, setSharingDryRun] = useState<TeamSharingSyncDryRun | null>(null);
  const [manifestContent, setManifestContent] = useState("");
  const [inspectingManifest, setInspectingManifest] = useState(false);
  const [manifestInspection, setManifestInspection] = useState<TeamSharingManifestInspection | null>(null);
  const [manifestLedgerFilter, setManifestLedgerFilter] = useState<ManifestLedgerFilter>("All");
  const [manifestLedgerQuery, setManifestLedgerQuery] = useState("");
  const [manifestLedgerLimit, setManifestLedgerLimit] = useState(4);
  const [expandedManifestLogId, setExpandedManifestLogId] = useState<number | null>(null);
  const [copiedManifestLogId, setCopiedManifestLogId] = useState<number | null>(null);
  const [copiedVisibleManifestLogs, setCopiedVisibleManifestLogs] = useState(false);
  const [manifestLedgerChecksum, setManifestLedgerChecksum] = useState<TeamSharingManifestLedgerChecksum | null>(null);
  const [copiedManifestLedgerChecksum, setCopiedManifestLedgerChecksum] = useState(false);
  const [trustingCurrentSigner, setTrustingCurrentSigner] = useState(false);
  const [sharingMessage, setSharingMessage] = useState<string | null>(null);
  const [sharingError, setSharingError] = useState<string | null>(null);
  const [retentionDraft, setRetentionDraft] = useState<VaultRetentionSettings>(vaultRetentionSettings);
  const [savingRetention, setSavingRetention] = useState(false);
  const [applyingRetention, setApplyingRetention] = useState(false);
  const [retentionResult, setRetentionResult] = useState<VaultRetentionResult | null>(null);
  const [retentionError, setRetentionError] = useState<string | null>(null);
  const manifestAuditLogs = manifestLedgerAuditLogs;
  const manifestWarningCount = manifestAuditLogs.filter((log) => log.severity === "Warning").length;
  const manifestVerifiedLogs = manifestAuditLogs.filter((log) => log.action === "team_sharing.manifest.inspected" && log.severity === "Info");
  const manifestExportLogs = manifestAuditLogs.filter((log) => log.action.includes(".exported"));
  const manifestFilteredExportLogs = manifestAuditLogs.filter((log) => log.action === "team_sharing.manifest_ledger.exported_filtered");
  const manifestVerifiedCount = manifestVerifiedLogs.length;
  const manifestExportCount = manifestExportLogs.length;
  const normalizedManifestLedgerQuery = manifestLedgerQuery.trim().toLowerCase();
  const filteredManifestAuditLogs = manifestAuditLogs.filter((log) => {
    if (manifestLedgerFilter === "Verified" && !manifestVerifiedLogs.includes(log)) return false;
    if (manifestLedgerFilter === "Warnings" && log.severity !== "Warning") return false;
    if (manifestLedgerFilter === "Exports" && !manifestExportLogs.includes(log)) return false;
    if (manifestLedgerFilter === "FilteredExports" && !manifestFilteredExportLogs.includes(log)) return false;
    if (!normalizedManifestLedgerQuery) return true;
    return `${log.id} ${log.action} ${log.actor} ${log.severity} ${log.created_at} ${log.resource}`.toLowerCase().includes(normalizedManifestLedgerQuery);
  });
  const visibleManifestAuditLogs = filteredManifestAuditLogs.slice(0, manifestLedgerLimit);
  useEffect(() => setPrivacyDraft(privacyStatus.settings), [privacyStatus.settings]);
  useEffect(() => setSharingDraft(teamSharingPolicy), [teamSharingPolicy]);
  useEffect(() => setRetentionDraft(vaultRetentionSettings), [vaultRetentionSettings]);
  useEffect(() => setManifestLedgerLimit(4), [manifestLedgerFilter, normalizedManifestLedgerQuery]);
  useEffect(() => setExpandedManifestLogId(null), [manifestLedgerFilter, normalizedManifestLedgerQuery]);
  useEffect(() => setManifestLedgerChecksum(null), [manifestLedgerFilter, normalizedManifestLedgerQuery]);
  async function sync() {
    setSyncing(true);
    try { setResult(await onRunSync()); } finally { setSyncing(false); }
  }
  async function checkVault() {
    setCheckingVault(true);
    setVaultError(null);
    try {
      setVaultStatus(await onCheckDatabaseReliability());
      setReliabilityChecksum(null);
      setCopiedReliabilityChecksum(false);
    } catch (cause) { setVaultError(messageFor(cause)); } finally { setCheckingVault(false); }
  }
  async function createBackup() {
    setBackingUp(true);
    setVaultError(null);
    try {
      const nextBackup = await onCreateBackup();
      setBackup(nextBackup);
      setBackupMessage("Verified local backup created.");
      setSessionVerifiedBackupPaths((current) => new Set(current).add(nextBackup.path));
      setCopiedBackupPath(false);
      setReliabilityChecksum(null);
      setCopiedReliabilityChecksum(false);
    } catch (cause) { setVaultError(messageFor(cause)); } finally { setBackingUp(false); }
  }
  async function verifyLatestBackup() {
    setVerifyingBackup(true);
    setVaultError(null);
    try {
      const nextBackup = await onVerifyLatestBackup();
      setBackup(nextBackup);
      setBackupMessage("Latest local backup re-verified.");
      setSessionVerifiedBackupPaths((current) => new Set(current).add(nextBackup.path));
      setCopiedBackupPath(false);
      setReliabilityChecksum(null);
      setCopiedReliabilityChecksum(false);
    } catch (cause) { setVaultError(messageFor(cause)); } finally { setVerifyingBackup(false); }
  }
  async function verifyBackupSnapshot(fileName: string) {
    setVerifyingBackupSnapshot(fileName);
    setVaultError(null);
    try {
      const nextBackup = await onVerifyBackupSnapshot({ file_name: fileName });
      setBackup(nextBackup);
      setBackupMessage(`Local backup ${fileName} re-verified.`);
      setSessionVerifiedBackupPaths((current) => new Set(current).add(nextBackup.path));
      setCopiedBackupPath(false);
      setReliabilityChecksum(null);
      setCopiedReliabilityChecksum(false);
    } catch (cause) {
      setVaultError(messageFor(cause));
    } finally {
      setVerifyingBackupSnapshot(null);
    }
  }
  async function copyBackupPath() {
    if (!backup) return;
    try {
      await navigator.clipboard.writeText(backup.path);
      setCopiedBackupPath(true);
      window.setTimeout(() => setCopiedBackupPath(false), 1800);
    } catch (cause) {
      setVaultError(messageFor(cause));
    }
  }
  async function loadRecentBackups() {
    setLoadingRecentBackups(true);
    setVaultError(null);
    try {
      setRecentBackups(await onGetRecentBackups());
    } catch (cause) {
      setVaultError(messageFor(cause));
    } finally {
      setLoadingRecentBackups(false);
    }
  }
  async function loadRecentReliabilityReports() {
    setLoadingReliabilityReports(true);
    setVaultError(null);
    try {
      setRecentReliabilityReports(await onGetRecentReliabilityReports());
    } catch (cause) {
      setVaultError(messageFor(cause));
    } finally {
      setLoadingReliabilityReports(false);
    }
  }
  async function exportReliabilityReport() {
    setExportingReliabilityReport(true);
    setVaultError(null);
    setReliabilityMessage(null);
    try {
      const report = await onExportDatabaseReliabilityReport();
      setReliabilityChecksum({
        integrity_status: report.integrity_status,
        snapshot_count: report.snapshot_count,
        report_data_sha256: report.report_data_sha256
      });
      setCopiedReliabilityChecksum(false);
      setRecentReliabilityReports(null);
      setCopiedReliabilityReportPath(null);
      setReliabilityMessage(`Vault reliability report exported to ${report.path}.`);
    } catch (cause) {
      setVaultError(messageFor(cause));
    } finally {
      setExportingReliabilityReport(false);
    }
  }
  async function calculateReliabilityChecksum() {
    setCalculatingReliabilityChecksum(true);
    setVaultError(null);
    try {
      setReliabilityChecksum(await onGetDatabaseReliabilityChecksum());
      setCopiedReliabilityChecksum(false);
    } catch (cause) {
      setVaultError(messageFor(cause));
    } finally {
      setCalculatingReliabilityChecksum(false);
    }
  }
  async function copyReliabilityChecksum() {
    if (!reliabilityChecksum) return;
    const checksumMetadata = [
      "CYMOS Vault Reliability Report Data SHA-256",
      `Integrity: ${reliabilityChecksum.integrity_status}`,
      `Snapshot count: ${reliabilityChecksum.snapshot_count}`,
      `SHA-256: ${reliabilityChecksum.report_data_sha256}`
    ].join("\n");
    try {
      await navigator.clipboard.writeText(checksumMetadata);
      setCopiedReliabilityChecksum(true);
      window.setTimeout(() => setCopiedReliabilityChecksum(false), 1800);
    } catch (cause) {
      setVaultError(messageFor(cause));
    }
  }
  async function copyReliabilityReportPath(path: string) {
    try {
      await navigator.clipboard.writeText(path);
      setCopiedReliabilityReportPath(path);
      window.setTimeout(() => setCopiedReliabilityReportPath(null), 1800);
    } catch (cause) {
      setVaultError(messageFor(cause));
    }
  }
  async function savePrivacy(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSavingPrivacy(true);
    setPrivacyMessage(null);
    setPrivacyError(null);
    try { await onSavePrivacySettings(privacyDraft); setPrivacyMessage("Capture privacy controls saved locally."); } catch (cause) { setPrivacyError(messageFor(cause)); } finally { setSavingPrivacy(false); }
  }
  async function saveSharing(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSavingSharing(true);
    setSharingMessage(null);
    setSharingError(null);
    try { await onSaveTeamSharingPolicy(sharingDraft); setSharingMessage("Team sharing policy saved locally."); } catch (cause) { setSharingError(messageFor(cause)); } finally { setSavingSharing(false); }
  }
  async function registerDevice(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSavingDevice(true);
    setSharingMessage(null);
    setSharingError(null);
    try { await onRegisterTeamSharingDevice(deviceDraft); setDeviceDraft({ device_name: "", platform: "macOS", sync_mode: "Local-only" }); setSharingMessage("Sharing device registered locally."); } catch (cause) { setSharingError(messageFor(cause)); } finally { setSavingDevice(false); }
  }
  async function approveDevice(deviceId: number) {
    setSharingError(null);
    try { await onApproveTeamSharingDevice({ device_id: deviceId }); setSharingMessage("Sharing device approved locally."); } catch (cause) { setSharingError(messageFor(cause)); }
  }
  async function revokeDevice(deviceId: number) {
    setSharingError(null);
    try { await onRevokeTeamSharingDevice({ device_id: deviceId }); setSharingMessage("Sharing device revoked locally."); } catch (cause) { setSharingError(messageFor(cause)); }
  }
  async function exportSharingReport() {
    setExportingSharingReport(true);
    setSharingMessage(null);
    setSharingError(null);
    try { const result = await onExportTeamSharingReport(); setSharingMessage(`Team sharing readiness report exported to ${result.path}.`); } catch (cause) { setSharingError(messageFor(cause)); } finally { setExportingSharingReport(false); }
  }
  async function exportManifestLedger() {
    setExportingManifestLedger(true);
    setSharingMessage(null);
    setSharingError(null);
    try {
      const result = await onExportTeamSharingManifestLedger();
      setSharingMessage(`Team sharing manifest ledger exported to ${result.path}.`);
    } catch (cause) {
      setSharingError(messageFor(cause));
    } finally {
      setExportingManifestLedger(false);
    }
  }
  async function exportFilteredManifestLedger() {
    setExportingFilteredManifestLedger(true);
    setSharingMessage(null);
    setSharingError(null);
    try {
      const result = await onExportFilteredTeamSharingManifestLedger({
        filter: manifestLedgerFilter,
        query: manifestLedgerQuery,
      });
      setSharingMessage(`Matching manifest ledger events exported to ${result.path}.`);
    } catch (cause) {
      setSharingError(messageFor(cause));
    } finally {
      setExportingFilteredManifestLedger(false);
    }
  }
  async function calculateManifestLedgerChecksum() {
    setCalculatingManifestLedgerChecksum(true);
    setSharingError(null);
    try {
      setManifestLedgerChecksum(await onGetTeamSharingManifestLedgerChecksum({
        filter: manifestLedgerFilter,
        query: manifestLedgerQuery,
      }));
    } catch (cause) {
      setSharingError(messageFor(cause));
    } finally {
      setCalculatingManifestLedgerChecksum(false);
    }
  }
  async function copyManifestLedgerChecksum() {
    if (!manifestLedgerChecksum) return;
    const metadata = [
      "CYMOS Manifest Ledger Event-Set SHA-256",
      `Filter: ${manifestLedgerChecksum.filter}`,
      `Search applied: ${manifestLedgerChecksum.search_applied ? "Yes" : "No"}`,
      `Event count: ${manifestLedgerChecksum.event_count}`,
      `SHA-256: ${manifestLedgerChecksum.event_set_sha256}`
    ].join("\n");
    try {
      await navigator.clipboard.writeText(metadata);
      setCopiedManifestLedgerChecksum(true);
      window.setTimeout(() => setCopiedManifestLedgerChecksum(false), 1800);
    } catch (cause) {
      setSharingError(messageFor(cause));
    }
  }
  async function runSharingDryRun() {
    setRunningSharingDryRun(true);
    setSharingMessage(null);
    setSharingError(null);
    try { setSharingDryRun(await onRunTeamSharingDryRun()); setSharingMessage("Local sync dry-run manifest generated."); } catch (cause) { setSharingError(messageFor(cause)); } finally { setRunningSharingDryRun(false); }
  }
  async function exportSharingManifest() {
    setExportingSharingManifest(true);
    setSharingMessage(null);
    setSharingError(null);
    try { const result = await onExportTeamSharingDryRunManifest(); setSharingMessage(`Team sharing dry-run manifest exported to ${result.path}.`); } catch (cause) { setSharingError(messageFor(cause)); } finally { setExportingSharingManifest(false); }
  }
  async function inspectSharingManifest(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setInspectingManifest(true);
    setSharingMessage(null);
    setSharingError(null);
    try { setManifestInspection(await onInspectTeamSharingManifest({ content: manifestContent })); setSharingMessage("Team sharing manifest inspected locally."); } catch (cause) { setSharingError(messageFor(cause)); } finally { setInspectingManifest(false); }
  }
  async function trustCurrentSigner() {
    setTrustingCurrentSigner(true);
    setSharingMessage(null);
    setSharingError(null);
    try {
      const record = await onTrustCurrentDeviceSigner();
      setSharingMessage(`Current device signer trusted locally: ${record.signer_fingerprint}.`);
    } catch (cause) {
      setSharingError(messageFor(cause));
    } finally {
      setTrustingCurrentSigner(false);
    }
  }
  async function copyManifestLog(log: AuditLog) {
    const metadata = [
      "CYMOS Manifest Ledger Event",
      `Event ID: ${log.id}`,
      `Actor: ${log.actor}`,
      `Action: ${log.action}`,
      `Severity: ${log.severity}`,
      `Time: ${log.created_at}`,
      `Resource: ${log.resource}`
    ].join("\n");
    try {
      await navigator.clipboard.writeText(metadata);
      setCopiedManifestLogId(log.id);
      window.setTimeout(() => setCopiedManifestLogId((current) => current === log.id ? null : current), 1800);
    } catch (cause) {
      setSharingError(messageFor(cause));
    }
  }
  async function copyVisibleManifestLogs() {
    if (visibleManifestAuditLogs.length === 0) return;
    const metadata = [
      "CYMOS Visible Manifest Ledger Events",
      `Filter: ${manifestLedgerFilter}`,
      `Search: ${manifestLedgerQuery.trim() || "None"}`,
      `Visible events: ${visibleManifestAuditLogs.length} of ${filteredManifestAuditLogs.length}`,
      "",
      ...visibleManifestAuditLogs.map((log, index) => [
        `Event ${index + 1} (ID: ${log.id})`,
        `Actor: ${log.actor}`,
        `Action: ${log.action}`,
        `Severity: ${log.severity}`,
        `Time: ${log.created_at}`,
        `Resource: ${log.resource}`
      ].join("\n"))
    ].join("\n\n");
    try {
      await navigator.clipboard.writeText(metadata);
      setCopiedVisibleManifestLogs(true);
      window.setTimeout(() => setCopiedVisibleManifestLogs(false), 1800);
    } catch (cause) {
      setSharingError(messageFor(cause));
    }
  }
  function resetManifestLedgerControls() {
    setManifestLedgerFilter("All");
    setManifestLedgerQuery("");
    setManifestLedgerLimit(4);
    setExpandedManifestLogId(null);
    setCopiedManifestLogId(null);
    setCopiedVisibleManifestLogs(false);
  }
  async function saveRetention(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSavingRetention(true);
    setRetentionError(null);
    try { await onSaveVaultRetentionSettings(retentionDraft); } catch (cause) { setRetentionError(messageFor(cause)); } finally { setSavingRetention(false); }
  }
  async function applyRetention() {
    setApplyingRetention(true);
    setRetentionError(null);
    try { setRetentionResult(await onApplyVaultRetention()); } catch (cause) { setRetentionError(messageFor(cause)); } finally { setApplyingRetention(false); }
  }
  const reliability = vaultStatus ?? databaseReliability;
  return (
    <div className="grid gap-5">
      <Panel>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
          <div><div className="flex items-center gap-2 text-sm font-medium text-emerald-700"><ShieldCheck size={16} /> {platform.encryption_status}</div><h2 className="mt-2 text-xl font-semibold text-slate-950">Universal memory platform</h2><p className="mt-1 text-sm text-slate-500">A local-first foundation for devices, integrations, and enterprise controls.</p></div>
          <button className="inline-flex h-10 items-center justify-center gap-2 rounded-md bg-slate-950 px-4 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60" disabled={syncing} onClick={() => void sync()} type="button"><CloudCog size={16} />{syncing ? "Checking" : "Run sync check"}</button>
        </div>
        {result ? <div className="mt-4 rounded-md border border-emerald-100 bg-emerald-50 px-3 py-2 text-sm text-emerald-950">{result.status}: {result.devices_checked} devices and {result.integrations_checked} integrations checked.</div> : null}
        <div className="mt-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <Metric label="Sync status" value={platform.sync_status} supporting={platform.sync_mode} tone="sky" />
          <Metric label="Connected devices" value={platform.device_count} supporting="Cross-platform ready" tone="violet" />
          <Metric label="Integrations" value={platform.integration_count} supporting="Workspace connectors" tone="emerald" />
          <Metric label="Performance" value={`${platform.performance_score}/100`} supporting={platform.retention_policy} tone="amber" />
        </div>
      </Panel>
      <Panel>
        <SectionHeading title="Team sharing policy" description="Local guardrails for future team handoff and synchronization workflows." />
        {sharingError ? <ErrorMessage message={sharingError} /> : null}
        {sharingMessage ? <div className="mb-4 rounded-md border border-emerald-100 bg-emerald-50 px-3 py-2 text-sm text-emerald-950">{sharingMessage}</div> : null}
        <div className={teamSharingReadiness.ready ? "mb-4 rounded-md border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-950" : teamSharingReadiness.status === "Blocked" ? "mb-4 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-950" : "mb-4 rounded-md border border-slate-200 bg-slate-50 p-3 text-sm text-slate-700"}>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <p className="font-semibold">Readiness: {teamSharingReadiness.status}</p>
              <p className="mt-1 text-xs leading-5">{teamSharingReadiness.mode} - {teamSharingReadiness.approved_devices} approved device(s), {teamSharingReadiness.trusted_recipients} trusted recipient(s), {teamSharingReadiness.trusted_signers} trusted signer(s)</p>
            </div>
            {teamSharingReadiness.checked_at ? <time className="shrink-0 text-xs opacity-70">{teamSharingReadiness.checked_at}</time> : null}
          </div>
          {teamSharingReadiness.allowed_scopes.length > 0 ? <div className="mt-2 flex flex-wrap gap-2">{teamSharingReadiness.allowed_scopes.map((scope) => <span className="rounded-md bg-white/70 px-2 py-1 text-xs font-medium" key={scope}>{scope}</span>)}</div> : null}
          {teamSharingReadiness.blockers.length > 0 ? <div className="mt-2 grid gap-1 text-xs leading-5">{teamSharingReadiness.blockers.map((blocker) => <p key={blocker}>{blocker}</p>)}</div> : null}
          <button className="mt-3 inline-flex h-9 w-full items-center justify-center gap-2 rounded-md border border-slate-200 bg-white px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={runningSharingDryRun} onClick={() => void runSharingDryRun()} type="button"><CloudCog size={15} />{runningSharingDryRun ? "Generating dry run" : "Generate dry-run manifest"}</button>
        </div>
        {sharingDryRun ? <div className={sharingDryRun.ready ? "mb-4 rounded-md border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-950" : "mb-4 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-950"}>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <p className="font-semibold">Dry run: {sharingDryRun.status}</p>
              <p className="mt-1 text-xs leading-5">{sharingDryRun.mode} - {sharingDryRun.eligible_devices} eligible device(s), {sharingDryRun.estimated_records} record(s), {formatBytes(sharingDryRun.estimated_bytes)}</p>
            </div>
            <time className="shrink-0 text-xs opacity-70">{sharingDryRun.generated_at}</time>
          </div>
          {sharingDryRun.eligible_scopes.length > 0 ? <div className="mt-2 flex flex-wrap gap-2">{sharingDryRun.eligible_scopes.map((scope) => <span className="rounded-md bg-white/70 px-2 py-1 text-xs font-medium" key={scope}>{scope}</span>)}</div> : null}
          {sharingDryRun.blockers.length > 0 ? <div className="mt-2 grid gap-1 text-xs leading-5">{sharingDryRun.blockers.map((blocker) => <p key={blocker}>{blocker}</p>)}</div> : null}
          <button className="mt-3 inline-flex h-9 w-full items-center justify-center gap-2 rounded-md border border-slate-200 bg-white px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={exportingSharingManifest} onClick={() => void exportSharingManifest()} type="button"><Download size={15} />{exportingSharingManifest ? "Exporting manifest" : "Export JSON manifest"}</button>
        </div> : null}
        <button className="mb-4 inline-flex h-9 w-full items-center justify-center gap-2 rounded-md border border-slate-200 bg-white px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={trustingCurrentSigner} onClick={() => void trustCurrentSigner()} type="button"><ShieldCheck size={15} />{trustingCurrentSigner ? "Trusting signer" : "Trust this device signer"}</button>
        <form className="mb-4 grid gap-3 rounded-md border border-slate-200 bg-slate-50 p-3" onSubmit={(event) => void inspectSharingManifest(event)}>
          <label className="grid gap-1.5 text-sm font-medium text-slate-700">Inspect dry-run manifest<textarea className="min-h-24 rounded-md border border-slate-200 bg-white px-3 py-2 font-mono text-xs leading-5 text-slate-700 outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => setManifestContent(event.target.value)} placeholder='{"format":"cymos.team_sharing_sync_dry_run", ...}' value={manifestContent} /></label>
          <button className="inline-flex h-9 items-center justify-center gap-2 rounded-md border border-slate-200 bg-white px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={inspectingManifest || manifestContent.trim().length === 0} type="submit"><ShieldCheck size={15} />{inspectingManifest ? "Inspecting" : "Inspect manifest"}</button>
        </form>
        {manifestInspection ? <div className={manifestInspection.valid ? "mb-4 rounded-md border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-950" : "mb-4 rounded-md border border-rose-200 bg-rose-50 p-3 text-sm text-rose-900"}>
          <p className="font-semibold">Manifest: {manifestInspection.status}</p>
          {manifestInspection.valid ? <p className="mt-1 text-xs leading-5">{manifestInspection.format} v{manifestInspection.schema_version} - {manifestInspection.mode} - {manifestInspection.estimated_records} record(s), {formatBytes(manifestInspection.estimated_bytes)} - {manifestInspection.device_count} device(s), {manifestInspection.blocker_count} blocker(s) - SHA-256 {manifestInspection.dry_run_sha256.slice(0, 16)} - {manifestInspection.signature_status}{manifestInspection.signer_fingerprint ? ` (${manifestInspection.signer_fingerprint})` : ""} - {manifestInspection.trust_status}</p> : <p className="mt-1 text-xs leading-5">{manifestInspection.failure_reason}</p>}
        </div> : null}
        <div className="mb-4 rounded-md border border-slate-200 bg-white p-3">
          <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
            <p className="text-sm font-semibold text-slate-800">Manifest inspection ledger</p>
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-xs font-medium text-slate-500">{manifestAuditLogs.length} local event(s)</span>
              <button className="inline-flex h-8 items-center justify-center gap-1.5 rounded-md border border-slate-200 px-2 text-xs font-semibold text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={visibleManifestAuditLogs.length === 0} onClick={() => void copyVisibleManifestLogs()} type="button"><Copy size={13} />{copiedVisibleManifestLogs ? "Copied visible" : "Copy visible"}</button>
              <button className="inline-flex h-8 items-center justify-center gap-1.5 rounded-md border border-slate-200 px-2 text-xs font-semibold text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={calculatingManifestLedgerChecksum} onClick={() => void calculateManifestLedgerChecksum()} type="button"><ShieldCheck size={13} />{calculatingManifestLedgerChecksum ? "Calculating" : "Show checksum"}</button>
              <button className="inline-flex h-8 items-center justify-center gap-1.5 rounded-md border border-slate-200 px-2 text-xs font-semibold text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={exportingFilteredManifestLedger || filteredManifestAuditLogs.length === 0} onClick={() => void exportFilteredManifestLedger()} type="button"><Download size={13} />{exportingFilteredManifestLedger ? "Exporting" : "Export matching"}</button>
              <button className="inline-flex h-8 items-center justify-center gap-1.5 rounded-md border border-slate-200 px-2 text-xs font-semibold text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={exportingManifestLedger} onClick={() => void exportManifestLedger()} type="button"><Download size={13} />{exportingManifestLedger ? "Exporting" : "Export all"}</button>
            </div>
          </div>
          <div className="mt-3 grid gap-2 sm:grid-cols-4">
            <SmallStat label="Events" value={manifestAuditLogs.length} />
            <SmallStat label="Verified" value={manifestVerifiedCount} />
            <SmallStat label="Warnings" value={manifestWarningCount} />
            <SmallStat label="Exports" value={manifestExportCount} />
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <div className="flex flex-wrap rounded-md border border-slate-200 bg-slate-50 p-1">
              {MANIFEST_LEDGER_FILTER_OPTIONS.map(({ label, value }) => <button className={manifestLedgerFilter === value ? "h-7 rounded bg-white px-2.5 text-xs font-semibold text-slate-900 shadow-sm" : "h-7 rounded px-2.5 text-xs font-semibold text-slate-500 hover:text-slate-800"} key={value} onClick={() => setManifestLedgerFilter(value)} type="button">{label}</button>)}
            </div>
            <button className="inline-flex h-8 items-center justify-center rounded-md border border-slate-200 px-2.5 text-xs font-semibold text-slate-700 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50" disabled={manifestLedgerFilter === "All" && manifestLedgerQuery.trim().length === 0 && manifestLedgerLimit === 4 && expandedManifestLogId === null} onClick={resetManifestLedgerControls} type="button">Reset view</button>
          </div>
          <label className="mt-3 grid gap-1.5 text-xs font-semibold text-slate-600">Search manifest ledger<input className="h-9 rounded-md border border-slate-200 bg-white px-3 text-sm font-normal text-slate-700 outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => setManifestLedgerQuery(event.target.value)} placeholder="Signer, checksum, status, path..." value={manifestLedgerQuery} /></label>
          {manifestLedgerChecksum ? <div className="mt-3 flex flex-wrap items-center justify-between gap-2 rounded-md border border-slate-200 bg-slate-50 p-2 text-xs text-slate-600"><p className="min-w-0 break-all">Event-set SHA-256: <span className="font-mono text-slate-800">{manifestLedgerChecksum.event_set_sha256}</span></p><button aria-label="Copy manifest ledger checksum" className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-slate-200 bg-white text-slate-600 hover:bg-slate-100" onClick={() => void copyManifestLedgerChecksum()} title="Copy checksum" type="button"><Copy size={13} /></button>{copiedManifestLedgerChecksum ? <span className="text-emerald-700">Copied</span> : null}</div> : null}
          <div className="mt-3 grid gap-2">
            {visibleManifestAuditLogs.map((log) => <div className="rounded-md bg-slate-50 p-2.5 text-xs" key={log.id}>
              <div className="flex items-start justify-between gap-3">
                <p className="min-w-0 truncate font-semibold text-slate-700">{log.action}</p>
                <div className="flex shrink-0 items-center gap-2 text-slate-400"><span>#{log.id}</span><time>{log.created_at}</time></div>
              </div>
              <p className="mt-1 line-clamp-2 leading-5 text-slate-500">{log.resource}</p>
              <div className="mt-1 flex flex-wrap items-center justify-between gap-2">
                <p className={log.severity === "Warning" ? "font-medium text-amber-700" : "font-medium text-emerald-700"}>{log.severity}</p>
                <div className="flex items-center gap-1.5">
                  <button className="inline-flex items-center gap-1 rounded-md border border-slate-200 bg-white px-2 py-1 text-[11px] font-semibold text-slate-600 hover:bg-slate-50" onClick={() => void copyManifestLog(log)} type="button"><Copy size={11} />{copiedManifestLogId === log.id ? "Copied" : "Copy"}</button>
                  <button className="rounded-md border border-slate-200 bg-white px-2 py-1 text-[11px] font-semibold text-slate-600 hover:bg-slate-50" onClick={() => setExpandedManifestLogId((current) => current === log.id ? null : log.id)} type="button">{expandedManifestLogId === log.id ? "Hide details" : "Details"}</button>
                </div>
              </div>
              {expandedManifestLogId === log.id ? <dl className="mt-2 grid gap-1 rounded-md border border-slate-200 bg-white p-2 text-[11px] leading-5 text-slate-600">
                <div className="flex gap-2"><dt className="w-16 shrink-0 font-semibold text-slate-500">Event ID</dt><dd>#{log.id}</dd></div>
                <div className="flex gap-2"><dt className="w-16 shrink-0 font-semibold text-slate-500">Actor</dt><dd>{log.actor}</dd></div>
                <div className="flex gap-2"><dt className="w-16 shrink-0 font-semibold text-slate-500">Action</dt><dd className="min-w-0 break-all">{log.action}</dd></div>
                <div className="flex gap-2"><dt className="w-16 shrink-0 font-semibold text-slate-500">Time</dt><dd>{log.created_at}</dd></div>
                <div className="flex gap-2"><dt className="w-16 shrink-0 font-semibold text-slate-500">Resource</dt><dd className="min-w-0 break-words">{log.resource}</dd></div>
              </dl> : null}
            </div>)}
            {filteredManifestAuditLogs.length === 0 ? <p className="text-xs leading-5 text-slate-500">{normalizedManifestLedgerQuery ? "No manifest ledger events match the current search." : manifestLedgerFilter === "Warnings" ? "No warning manifest events in the local ledger." : manifestLedgerFilter === "Verified" ? "No verified manifest inspections in the local ledger." : manifestLedgerFilter === "Exports" ? "No manifest exports in the local ledger." : manifestLedgerFilter === "FilteredExports" ? "No filtered manifest ledger exports in the local ledger." : "Manifest exports and inspections will appear here after local review."}</p> : null}
            {filteredManifestAuditLogs.length > 4 ? <div className="flex flex-wrap items-center gap-2 pt-1">
              {manifestLedgerLimit < filteredManifestAuditLogs.length ? <button className="inline-flex h-8 items-center justify-center rounded-md border border-slate-200 px-2.5 text-xs font-semibold text-slate-700 hover:bg-slate-50" onClick={() => setManifestLedgerLimit((current) => Math.min(current + 4, filteredManifestAuditLogs.length))} type="button">Show more</button> : null}
              {manifestLedgerLimit > 4 ? <button className="inline-flex h-8 items-center justify-center rounded-md border border-slate-200 px-2.5 text-xs font-semibold text-slate-700 hover:bg-slate-50" onClick={() => setManifestLedgerLimit(4)} type="button">Show fewer</button> : null}
              <span className="text-xs text-slate-500">Showing {Math.min(manifestLedgerLimit, filteredManifestAuditLogs.length)} of {filteredManifestAuditLogs.length}</span>
            </div> : null}
          </div>
        </div>
        <form className="grid gap-4" onSubmit={(event) => void saveSharing(event)}>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <Toggle label="Policy enabled" onChange={(enabled) => setSharingDraft((current) => ({ ...current, enabled }))} value={sharingDraft.enabled} />
            <label className="grid gap-1.5 text-sm font-medium text-slate-700">Mode<select className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => setSharingDraft((current) => ({ ...current, mode: event.target.value as TeamSharingPolicy["mode"] }))} value={sharingDraft.mode}><option value="LocalOnly">Local only</option><option value="SelfHosted">Self-hosted</option><option value="EncryptedCloud">Encrypted cloud</option></select></label>
            <label className="grid gap-1.5 text-sm font-medium text-slate-700">Shared data retention<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" max={365} min={1} onChange={(event) => setSharingDraft((current) => ({ ...current, retention_days: Number(event.target.value) || 1 }))} type="number" value={sharingDraft.retention_days} /></label>
            <Toggle label="Approve devices" onChange={(require_device_approval) => setSharingDraft((current) => ({ ...current, require_device_approval }))} value={sharingDraft.require_device_approval} />
          </div>
          <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
            <Toggle label="Workspace handoffs" onChange={(allow_workspace_handoffs) => setSharingDraft((current) => ({ ...current, allow_workspace_handoffs }))} value={sharingDraft.allow_workspace_handoffs} />
            <Toggle label="Runbook exports" onChange={(allow_runbook_exports) => setSharingDraft((current) => ({ ...current, allow_runbook_exports }))} value={sharingDraft.allow_runbook_exports} />
            <Toggle label="Imported references" onChange={(allow_imported_references) => setSharingDraft((current) => ({ ...current, allow_imported_references }))} value={sharingDraft.allow_imported_references} />
            <Toggle label="Trusted recipients" onChange={(require_recipient_trust) => setSharingDraft((current) => ({ ...current, require_recipient_trust }))} value={sharingDraft.require_recipient_trust} />
          </div>
          <div className="flex flex-col gap-3 border-t border-slate-100 pt-4 sm:flex-row sm:items-center sm:justify-between">
            <div className="text-sm text-slate-600">{sharingDraft.enabled ? `Policy ready in ${sharingDraft.mode} mode` : "Team sharing remains disabled"}{teamSharingPolicy.updated_at ? ` - updated ${teamSharingPolicy.updated_at}` : ""}</div>
            <button className="inline-flex h-9 items-center justify-center gap-2 rounded-md bg-slate-950 px-3 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60" disabled={savingSharing} type="submit"><ShieldCheck size={15} />{savingSharing ? "Saving" : "Save sharing policy"}</button>
          </div>
        </form>
      </Panel>
      <Panel>
        <SectionHeading title="Approved devices" description="Local device approvals used by the team sharing readiness check." />
        <form className="grid gap-3 border-b border-slate-100 pb-4 lg:grid-cols-[minmax(0,1fr)_150px_170px_auto]" onSubmit={(event) => void registerDevice(event)}>
          <label className="grid gap-1.5 text-sm font-medium text-slate-700">Device name<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={120} onChange={(event) => setDeviceDraft((current) => ({ ...current, device_name: event.target.value }))} placeholder="e.g. RHEL admin laptop" required value={deviceDraft.device_name} /></label>
          <label className="grid gap-1.5 text-sm font-medium text-slate-700">Platform<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={80} onChange={(event) => setDeviceDraft((current) => ({ ...current, platform: event.target.value }))} required value={deviceDraft.platform} /></label>
          <label className="grid gap-1.5 text-sm font-medium text-slate-700">Mode<select className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => setDeviceDraft((current) => ({ ...current, sync_mode: event.target.value as TeamSharingDeviceRequest["sync_mode"] }))} value={deviceDraft.sync_mode}><option value="Local-only">Local-only</option><option value="Self-hosted">Self-hosted</option><option value="Encrypted cloud">Encrypted cloud</option></select></label>
          <button className="mt-auto inline-flex h-10 items-center justify-center gap-2 rounded-md bg-slate-950 px-3 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60" disabled={savingDevice} type="submit"><ShieldCheck size={15} />{savingDevice ? "Saving" : "Register"}</button>
        </form>
        <div className="mt-4 grid gap-2">
          {devices.map((device) => <div className={device.status === "Approved" ? "rounded-md border border-emerald-100 bg-emerald-50 p-3" : device.status === "Revoked" ? "rounded-md border border-rose-100 bg-rose-50 p-3" : "rounded-md border border-slate-200 bg-slate-50 p-3"} key={device.id}>
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
              <div className="min-w-0">
                <p className="truncate text-sm font-semibold text-slate-800">{device.device_name}</p>
                <p className="mt-1 text-xs leading-5 text-slate-500">{device.platform} - {device.sync_mode} - last seen {device.last_seen_at}</p>
              </div>
              <div className="flex shrink-0 flex-wrap items-center gap-2">
                <span className={device.status === "Approved" ? "rounded-md bg-white px-2 py-1 text-xs font-semibold text-emerald-700" : device.status === "Revoked" ? "rounded-md bg-white px-2 py-1 text-xs font-semibold text-rose-700" : "rounded-md bg-white px-2 py-1 text-xs font-semibold text-slate-600"}>{device.status}</span>
                {device.status !== "Approved" ? <button className="inline-flex h-8 items-center justify-center rounded-md border border-emerald-200 bg-white px-2 text-xs font-semibold text-emerald-800 hover:bg-emerald-50" onClick={() => void approveDevice(device.id)} type="button">Approve</button> : <button className="inline-flex h-8 items-center justify-center rounded-md border border-slate-200 bg-white px-2 text-xs font-semibold text-slate-700 hover:bg-slate-50" onClick={() => void revokeDevice(device.id)} type="button">Revoke</button>}
              </div>
            </div>
          </div>)}
          {devices.length === 0 ? <p className="text-sm text-slate-500">No local sharing devices registered.</p> : null}
        </div>
      </Panel>
      <Panel>
        <SectionHeading
          title="Sharing audit"
          description="Local record of team sharing policy and device decisions."
          action={<button className="inline-flex h-9 items-center justify-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={exportingSharingReport} onClick={() => void exportSharingReport()} type="button"><Download size={15} />{exportingSharingReport ? "Exporting" : "Export report"}</button>}
        />
        <div className="grid gap-2">
          {teamSharingAuditLogs.slice(0, 8).map((log) => <div className="rounded-md border border-slate-100 bg-slate-50 p-3 text-xs" key={log.id}>
            <div className="flex items-start justify-between gap-3">
              <p className="min-w-0 truncate font-semibold text-slate-700">{log.action}</p>
              <time className="shrink-0 text-slate-400">{log.created_at}</time>
            </div>
            <p className="mt-1 text-slate-500">{log.resource}</p>
            <p className={log.severity === "Warning" ? "mt-1 font-medium text-amber-700" : "mt-1 font-medium text-slate-500"}>{log.actor} - {log.severity}</p>
          </div>)}
          {teamSharingAuditLogs.length === 0 ? <p className="text-sm text-slate-500">No team sharing audit events yet.</p> : null}
        </div>
      </Panel>
      <Panel>
        <SectionHeading
          title="Vault reliability"
          description="Local integrity checks and consistent SQLite snapshots."
          action={<div className="flex flex-wrap gap-2"><button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={checkingVault} onClick={() => void checkVault()} type="button"><RefreshCw className={checkingVault ? "animate-spin" : undefined} size={15} />{checkingVault ? "Checking" : "Run check"}</button><button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={verifyingBackup || verifyingBackupSnapshot !== null || reliability.backup_count === 0} onClick={() => void verifyLatestBackup()} type="button"><ShieldCheck size={15} />{verifyingBackup ? "Verifying" : "Verify latest"}</button><button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={loadingRecentBackups || reliability.backup_count === 0} onClick={() => void loadRecentBackups()} type="button"><Clock3 size={15} />{loadingRecentBackups ? "Loading" : "Show snapshots"}</button><button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={loadingReliabilityReports} onClick={() => void loadRecentReliabilityReports()} type="button"><FileText size={15} />{loadingReliabilityReports ? "Loading" : "Show reports"}</button><button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={exportingReliabilityReport} onClick={() => void exportReliabilityReport()} type="button"><Download size={15} />{exportingReliabilityReport ? "Exporting" : "Export status"}</button><button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={calculatingReliabilityChecksum} onClick={() => void calculateReliabilityChecksum()} type="button"><ShieldCheck size={15} />{calculatingReliabilityChecksum ? "Calculating" : "Show checksum"}</button><button className="inline-flex h-9 items-center gap-2 rounded-md bg-slate-950 px-3 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60" disabled={backingUp || verifyingBackup || verifyingBackupSnapshot !== null} onClick={() => void createBackup()} type="button"><Database size={15} />{backingUp ? "Creating backup" : "Create backup"}</button></div>}
        />
        {vaultError ? <ErrorMessage message={vaultError} /> : null}
        {reliabilityMessage ? <div className="mb-4 rounded-md border border-sky-100 bg-sky-50 px-3 py-2 text-sm text-sky-950">{reliabilityMessage}</div> : null}
        {reliabilityChecksum ? <div className="mb-4 flex flex-wrap items-center justify-between gap-2 rounded-md border border-slate-200 bg-slate-50 px-3 py-2 text-xs text-slate-700"><p className="min-w-0 break-all">Report data SHA-256: <span className="font-mono text-slate-900">{reliabilityChecksum.report_data_sha256}</span></p><div className="flex items-center gap-2"><span className="text-slate-500">{reliabilityChecksum.snapshot_count} snapshot(s)</span><button aria-label="Copy vault reliability checksum" className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-slate-200 bg-white text-slate-600 hover:bg-slate-100" onClick={() => void copyReliabilityChecksum()} title="Copy checksum" type="button"><Copy size={13} /></button>{copiedReliabilityChecksum ? <span className="text-emerald-700">Copied</span> : null}</div></div> : null}
        {recentReliabilityReports ? <div className="mb-4 border-t border-slate-100 pt-4"><div className="flex items-center justify-between gap-3"><p className="text-sm font-semibold text-slate-800">Recent reliability reports</p><span className="text-xs text-slate-500">Showing {recentReliabilityReports.length} newest</span></div><div className="mt-2 grid gap-2">{recentReliabilityReports.map((report) => <div className="rounded-md bg-slate-50 p-2.5 text-xs" key={report.path}><div className="flex flex-wrap items-center justify-between gap-2"><div className="min-w-0"><p className="break-all font-medium text-slate-700">{report.file_name}</p><p className="mt-1 break-all text-slate-500">{formatBytes(report.bytes)} - {report.modified_at_unix > 0 ? new Date(report.modified_at_unix * 1000).toLocaleString() : "Time unavailable"}</p></div><button className="inline-flex h-7 shrink-0 items-center gap-1 rounded-md border border-slate-200 bg-white px-2 text-xs font-semibold text-slate-700 hover:bg-slate-100" onClick={() => void copyReliabilityReportPath(report.path)} type="button"><Copy size={12} />{copiedReliabilityReportPath === report.path ? "Copied" : "Copy path"}</button></div></div>)}{recentReliabilityReports.length === 0 ? <p className="text-sm text-slate-500">No local vault reliability reports are available.</p> : null}</div></div> : null}
        {backup ? <div className="mb-4 rounded-md border border-emerald-100 bg-emerald-50 px-3 py-2 text-sm text-emerald-950"><p>{backupMessage ?? "Verified local backup ready."} {backup.backup_count} snapshots are retained.</p><div className="mt-2 flex flex-wrap items-center gap-2 text-xs"><span className="font-semibold">Path</span><code className="min-w-0 flex-1 break-all rounded bg-white/70 px-2 py-1 text-slate-700">{backup.path}</code><button className="inline-flex h-7 items-center gap-1 rounded-md border border-emerald-200 bg-white px-2 text-xs font-semibold text-emerald-800 hover:bg-emerald-100" onClick={() => void copyBackupPath()} type="button"><Copy size={12} />{copiedBackupPath ? "Copied" : "Copy path"}</button></div></div> : null}
        {recentBackups ? <div className="mb-4 border-t border-slate-100 pt-4"><div className="flex items-center justify-between gap-3"><p className="text-sm font-semibold text-slate-800">Recent local snapshots</p><span className="text-xs text-slate-500">Showing {recentBackups.length} newest</span></div><div className="mt-2 grid gap-2">{recentBackups.map((snapshot) => <div className="rounded-md bg-slate-50 p-2.5 text-xs" key={snapshot.path}><div className="flex flex-wrap items-center justify-between gap-2"><p className="min-w-0 break-all font-medium text-slate-700">{snapshot.file_name}</p><div className="flex flex-wrap items-center gap-2"><span className="shrink-0 text-slate-500">{formatBytes(snapshot.bytes)} - {snapshot.modified_at_unix > 0 ? new Date(snapshot.modified_at_unix * 1000).toLocaleString() : "Time unavailable"}</span>{sessionVerifiedBackupPaths.has(snapshot.path) ? <span className="font-medium text-emerald-700">Verified this session</span> : null}<button className="inline-flex h-7 items-center gap-1 rounded-md border border-slate-200 bg-white px-2 text-xs font-semibold text-slate-700 hover:bg-slate-100 disabled:opacity-60" disabled={verifyingBackup || verifyingBackupSnapshot !== null} onClick={() => void verifyBackupSnapshot(snapshot.file_name)} type="button"><ShieldCheck size={12} />{verifyingBackupSnapshot === snapshot.file_name ? "Verifying" : "Verify"}</button></div></div></div>)}{recentBackups.length === 0 ? <p className="text-sm text-slate-500">No local snapshots are available.</p> : null}</div></div> : null}
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <Metric label="Integrity" value={reliability.integrity_status} supporting={reliability.foreign_key_issues === 0 ? "No foreign-key issues" : `${reliability.foreign_key_issues} foreign-key issues`} tone={reliability.integrity_status === "Healthy" ? "emerald" : "rose"} />
          <Metric label="Recovery mode" value={reliability.journal_mode} supporting="Crash-safe journal" tone="sky" />
          <Metric label="Backups" value={reliability.backup_count} supporting={reliability.last_backup ?? "No snapshot yet"} tone="violet" />
          <Metric label="Schema" value={`${reliability.migration_count} migrations`} supporting={formatBytes(reliability.database_bytes)} tone="amber" />
        </div>
      </Panel>
      <Panel>
        <SectionHeading title="Capture privacy" description="Sensitive values are blocked before local analysis and storage." />
        {privacyError ? <ErrorMessage message={privacyError} /> : null}
        {privacyMessage ? <div className="mb-4 rounded-md border border-emerald-100 bg-emerald-50 px-3 py-2 text-sm text-emerald-950">{privacyMessage}</div> : null}
        <form className="grid gap-4" onSubmit={(event) => void savePrivacy(event)}>
          <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
            <Toggle label="Protection enabled" onChange={(protection_enabled) => setPrivacyDraft((current) => ({ ...current, protection_enabled }))} value={privacyDraft.protection_enabled} />
            <Toggle label="Capture text" onChange={(capture_text) => setPrivacyDraft((current) => ({ ...current, capture_text }))} value={privacyDraft.capture_text} />
            <Toggle label="Capture copied images" onChange={(capture_images) => setPrivacyDraft((current) => ({ ...current, capture_images }))} value={privacyDraft.capture_images} />
            <Toggle label="Block likely secrets" onChange={(block_sensitive_text) => setPrivacyDraft((current) => ({ ...current, block_sensitive_text }))} value={privacyDraft.block_sensitive_text} />
          </div>
          <div className="flex flex-col gap-3 border-t border-slate-100 pt-4 sm:flex-row sm:items-center sm:justify-between">
            <div className="text-sm text-slate-600"><span className="font-semibold text-slate-800">{privacyStatus.blocked_capture_count}</span> sensitive captures blocked locally{privacyStatus.last_blocked_at ? ` - last ${privacyStatus.last_blocked_at}` : ""}</div>
            <button className="inline-flex h-9 items-center justify-center gap-2 rounded-md bg-slate-950 px-3 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60" disabled={savingPrivacy} type="submit"><ShieldCheck size={15} />{savingPrivacy ? "Saving" : "Save privacy controls"}</button>
          </div>
        </form>
      </Panel>
      <Panel>
        <SectionHeading title="Vault lifecycle" description="Repeated copies refresh one memory. Retention removes only unprotected memories." />
        {retentionError ? <ErrorMessage message={retentionError} /> : null}
        {retentionResult ? <div className={`mb-4 rounded-md border px-3 py-2 text-sm ${retentionResult.limits_met ? "border-emerald-100 bg-emerald-50 text-emerald-950" : "border-amber-200 bg-amber-50 text-amber-950"}`}>{retentionResult.removed_items} memories removed, {retentionResult.remaining_items} retained, {retentionResult.protected_favorites} favorites protected. {retentionResult.limits_met ? "Policy limits met." : "Protected favorites exceed a configured limit."}</div> : null}
        <form className="grid gap-4" onSubmit={(event) => void saveRetention(event)}>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <label className="grid gap-1.5 text-sm font-medium text-slate-700">Retention days<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" max={3650} min={1} onChange={(event) => setRetentionDraft((current) => ({ ...current, retention_days: Number(event.target.value) || 1 }))} type="number" value={retentionDraft.retention_days} /></label>
            <label className="grid gap-1.5 text-sm font-medium text-slate-700">Memory limit<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" max={1000000} min={100} onChange={(event) => setRetentionDraft((current) => ({ ...current, max_items: Number(event.target.value) || 100 }))} type="number" value={retentionDraft.max_items} /></label>
            <label className="grid gap-1.5 text-sm font-medium text-slate-700">Storage limit (MB)<input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" max={102400} min={64} onChange={(event) => setRetentionDraft((current) => ({ ...current, max_storage_mb: Number(event.target.value) || 64 }))} type="number" value={retentionDraft.max_storage_mb} /></label>
            <Toggle label="Protect favorites" onChange={(preserve_favorites) => setRetentionDraft((current) => ({ ...current, preserve_favorites }))} value={retentionDraft.preserve_favorites} />
          </div>
          <div className="flex flex-wrap gap-2 border-t border-slate-100 pt-4">
            <button className="inline-flex h-9 items-center justify-center gap-2 rounded-md bg-slate-950 px-3 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60" disabled={savingRetention} type="submit">{savingRetention ? "Saving" : "Save lifecycle"}</button>
            <button className="inline-flex h-9 items-center justify-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60" disabled={applyingRetention} onClick={() => void applyRetention()} type="button"><RefreshCw className={applyingRetention ? "animate-spin" : undefined} size={15} />{applyingRetention ? "Applying" : "Apply policy"}</button>
          </div>
        </form>
      </Panel>
      <div className="grid gap-5 xl:grid-cols-3">
        <CompactList title="Integrations" items={connectors.map((connector) => ({ title: connector.name, detail: connector.capabilities.join(", "), badge: connector.category }))} empty="No integrations configured." />
        <CompactList title="Enterprise controls" items={controls.map((control) => ({ title: control.name, detail: control.scope, badge: control.status }))} empty="No controls registered." />
        <CompactList title="Use cases" items={useCases.map((useCase) => ({ title: useCase.audience, detail: useCase.workflow, badge: useCase.status }))} empty="No use cases registered." />
      </div>
      <div className="grid gap-5 xl:grid-cols-3">
        <CompactList title="Plugin registry" items={plugins.map((plugin) => ({ title: plugin.name, detail: `v${plugin.version} - ${plugin.permissions}`, badge: plugin.status }))} empty="No plugins registered." />
        <CompactList title="API clients" items={apiClients.map((client) => ({ title: client.name, detail: client.scope, badge: client.status }))} empty="No API clients registered." />
      </div>
      <CompactList title="Recent audit activity" items={auditLogs.slice(0, 6).map((log) => ({ title: log.action, detail: `${log.actor} - ${log.resource}`, badge: log.severity }))} empty="No audit events yet." />
    </div>
  );
}

export function InsightsView({ stats, dailySummary, weeklyReport, graph, health }: { stats: ClipboardStats; dailySummary: KnowledgeDigest | null; weeklyReport: KnowledgeDigest | null; graph: KnowledgeGraph; health: KnowledgeHealth }) {
  return (
    <div className="grid gap-5">
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <Metric label="Total memories" value={stats.total_items} supporting={`${stats.favorite_items} favorites`} tone="sky" />
        <Metric label="Active topics" value={graph.clusters.length} supporting="Topic clusters" tone="violet" />
        <Metric label="Graph links" value={graph.edges.length} supporting="Knowledge relationships" tone="emerald" />
        <Metric label="AI activity" value={health.ai_activity} supporting="Local intelligence events" tone="amber" />
      </div>
      <div className="grid gap-5 xl:grid-cols-2"><DigestCard digest={dailySummary} icon={<Sparkles size={17} />} /><DigestCard digest={weeklyReport} icon={<ClockIcon />} /></div>
      <Panel>
        <SectionHeading title="Memory by content" description="The kinds of knowledge you are capturing." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <Bar label="Text" value={stats.text_items} total={stats.total_items} color="bg-sky-500" />
          <Bar label="Code" value={stats.code_items} total={stats.total_items} color="bg-violet-500" />
          <Bar label="URLs" value={stats.url_items} total={stats.total_items} color="bg-emerald-500" />
          <Bar label="Files" value={stats.file_items} total={stats.total_items} color="bg-amber-500" />
        </div>
      </Panel>
    </div>
  );
}

function Metric({ label, value, supporting, tone }: { label: string; value: string | number; supporting: string; tone: "sky" | "violet" | "emerald" | "amber" | "rose" }) {
  const tones = { sky: "border-sky-100 bg-sky-50", violet: "border-violet-100 bg-violet-50", emerald: "border-emerald-100 bg-emerald-50", amber: "border-amber-100 bg-amber-50", rose: "border-rose-100 bg-rose-50" };
  return <div className={`rounded-lg border p-4 ${tones[tone]}`}><p className="text-xs font-medium text-slate-500">{label}</p><p className="mt-2 truncate text-xl font-semibold tabular-nums text-slate-950">{value}</p><p className="mt-1 truncate text-xs text-slate-500">{supporting}</p></div>;
}

function HeroMetric({ icon, label, value }: { icon: ReactNode; label: string; value: string | number }) {
  return <div className="flex items-center gap-3"><span className="grid h-8 w-8 place-items-center rounded-md bg-white/10 text-sky-200">{icon}</span><div><p className="text-xl font-semibold tabular-nums">{value}</p><p className="text-xs text-slate-400">{label}</p></div></div>;
}

function Pulse({ label, value, icon }: { label: string; value: string | number; icon: ReactNode }) {
  return <div className="flex items-center gap-3 rounded-md bg-slate-50 px-3 py-2.5"><span className="text-sky-700">{icon}</span><span className="flex-1 text-sm text-slate-600">{label}</span><span className="text-sm font-semibold tabular-nums text-slate-900">{value}</span></div>;
}

function DigestCard({ digest, icon }: { digest: KnowledgeDigest | null; icon: ReactNode }) {
  if (!digest) return <Panel><p className="text-sm text-slate-500">Insights will appear as your memory grows.</p></Panel>;
  return <Panel><div className="flex items-center gap-2"><span className="text-sky-700">{icon}</span><h2 className="text-base font-semibold text-slate-950">{digest.title}</h2></div><ul className="mt-4 grid gap-2">{digest.bullets.slice(0, 4).map((bullet) => <li className="text-sm leading-6 text-slate-600" key={bullet}>- {bullet}</li>)}</ul><div className="mt-4 flex flex-wrap gap-2">{digest.active_topics.slice(0, 8).map((topic) => <span className="rounded-md bg-slate-100 px-2 py-1 text-xs font-medium text-slate-600" key={topic}>{topic}</span>)}</div></Panel>;
}

function Select({ label, value, onChange, children }: { label: string; value: string; onChange: (value: string) => void; children: ReactNode }) {
  return <select aria-label={label} className="h-9 min-w-0 rounded-md border border-slate-200 bg-white px-2.5 text-sm text-slate-700 outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" value={value} onChange={(event) => onChange(event.target.value)}>{children}</select>;
}

function Toggle({ label, value, onChange }: { label: string; value: boolean; onChange: (value: boolean) => void }) {
  return <label className="flex cursor-pointer items-center gap-1.5 text-xs font-medium whitespace-nowrap"><input className="h-3.5 w-3.5 accent-sky-600" type="checkbox" checked={value} onChange={(event) => onChange(event.target.checked)} />{label}</label>;
}

function SmallStat({ label, value }: { label: string; value: string | number }) {
  return <div className="flex items-center justify-between"><span className="text-sm text-slate-500">{label}</span><span className="text-sm font-semibold tabular-nums text-slate-800">{value}</span></div>;
}

function CompactList({ title, items, empty }: { title: string; items: Array<{ title: string; detail: string; badge: string }>; empty: string }) {
  return <Panel><p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">{title}</p><div className="mt-3 grid gap-2.5">{items.length > 0 ? items.slice(0, 8).map((item, index) => <div className="rounded-md bg-slate-50 p-3" key={`${item.title}-${index}`}><div className="flex items-start justify-between gap-2"><p className="min-w-0 truncate text-sm font-medium text-slate-700">{item.title}</p><span className="shrink-0 rounded bg-white px-1.5 py-0.5 text-[11px] font-medium text-slate-500">{item.badge}</span></div><p className="mt-1 line-clamp-2 text-xs leading-5 text-slate-500">{item.detail}</p></div>) : <p className="text-sm leading-6 text-slate-500">{empty}</p>}</div></Panel>;
}

function ErrorMessage({ message }: { message: string }) { return <div className="rounded-lg border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">{message}</div>; }
function EmptyRow({ label }: { label: string }) { return <p className="py-6 text-sm text-slate-500">{label}</p>; }

function MemoryTypeIcon({ type }: { type: ClipboardItem["content_type"] }) {
  const Icon = type === "Code" ? Braces : type === "URL" ? Link2 : type === "File" || type === "Folder" ? FileText : Grid2X2;
  return <span className="grid h-8 w-8 shrink-0 place-items-center rounded-md bg-slate-100 text-slate-600"><Icon size={16} /></span>;
}

function previewTitle(item: ClipboardItem) { return item.ai_summary || item.content.split("\n")[0] || item.content_type; }

function AssistantResponseCard({ response }: { response: AssistantResponse }) {
  return <div className="mt-5"><div className="rounded-lg bg-sky-50 p-5"><div className="flex items-center gap-2 text-xs font-medium text-sky-700"><CircleDot size={14} />{response.model}<span className="text-sky-600/70">- {response.retrieval_summary}</span></div><p className="mt-3 whitespace-pre-wrap text-sm leading-7 text-slate-800">{response.answer}</p></div><div className="mt-4 grid gap-4 sm:grid-cols-2"><div><p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Sources</p><div className="mt-2 grid gap-2">{response.sources.map((source) => <div className="rounded-md border border-slate-200 px-3 py-2" key={source.id}><div className="flex justify-between gap-2"><span className="truncate text-sm font-medium text-slate-700">{source.category}</span><span className="text-xs text-sky-700">{Math.round(source.score * 100)}%</span></div><p className="mt-1 line-clamp-2 text-xs text-slate-500">{source.title}</p></div>)}</div></div><div><p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Related topics</p><div className="mt-2 flex flex-wrap gap-2">{response.related_topics.map((topic) => <span className="rounded-md bg-slate-100 px-2 py-1 text-xs font-medium text-slate-600" key={topic}>{topic}</span>)}</div></div></div></div>;
}

function AssistantEmpty() { return <div className="grid min-h-[280px] place-items-center text-center"><div><BrainCircuit className="mx-auto text-slate-300" size={34} /><p className="mt-3 text-sm font-medium text-slate-600">Ask a question to search your memory.</p><p className="mt-1 text-sm text-slate-400">The assistant will show the sources it used.</p></div></div>; }
function AssistantPrompts({ onPick }: { onPick: (value: string) => void }) { const prompts = ["What Docker commands have I saved?", "Show my notes related to BSNL.", "What did I learn about FastAPI?"]; return <Panel><p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Try asking</p><div className="mt-3 grid gap-2">{prompts.map((prompt) => <button className="rounded-md bg-slate-50 px-3 py-2.5 text-left text-sm leading-5 text-slate-600 hover:bg-sky-50 hover:text-sky-800" key={prompt} onClick={() => onPick(prompt)} type="button">{prompt}</button>)}</div></Panel>; }

function WorkflowResult({ workflow }: { workflow: AgentWorkflow }) { return <div className="mt-5 grid gap-4"><div className="rounded-lg bg-violet-50 p-5"><div className="flex flex-wrap gap-2">{workflow.agents.map((agent) => <span className="rounded-md bg-white px-2 py-1 text-xs font-medium text-violet-700" key={agent}>{agent}</span>)}</div><p className="mt-3 whitespace-pre-wrap text-sm leading-7 text-slate-800">{workflow.answer}</p></div><div className="grid gap-4 md:grid-cols-2"><div><p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Plan</p><div className="mt-2 grid gap-2">{workflow.plan.map((step, index) => <div className="rounded-md bg-slate-50 p-3" key={`${step.agent}-${index}`}><p className="text-sm font-medium text-slate-700">{step.agent}</p><p className="mt-1 text-xs font-semibold text-violet-700">{step.action}</p><p className="mt-1 text-xs leading-5 text-slate-500">{step.output}</p></div>)}</div></div><div><p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">Activity log</p><div className="mt-2 grid gap-2">{workflow.logs.map((log, index) => <div className="rounded-md bg-slate-50 p-3" key={`${log.agent}-${index}`}><p className="text-xs font-semibold text-slate-700">{log.agent}</p><p className="mt-1 text-xs leading-5 text-slate-500">{log.message}</p></div>)}</div></div></div></div>; }
function AgentEmpty() { return <div className="grid min-h-[320px] place-items-center text-center"><div><Bot className="mx-auto text-slate-300" size={34} /><p className="mt-3 text-sm font-medium text-slate-600">A focused goal is all the agents need.</p><p className="mt-1 text-sm text-slate-400">They will use relevant local memory as their starting context.</p></div></div>; }
function HistoryEntry({ entry }: { entry: AgentWorkflowRecord }) { return <div className="rounded-md bg-slate-50 p-3"><div className="flex gap-2"><p className="min-w-0 flex-1 truncate text-sm font-medium text-slate-700">{entry.goal}</p><span className="text-[11px] text-slate-400">{entry.created_at}</span></div><p className="mt-1 line-clamp-2 text-xs leading-5 text-slate-500">{entry.answer}</p></div>; }

function GraphMap({ graph }: { graph: KnowledgeGraph }) { return <div className="grid w-full max-w-xl grid-cols-2 gap-3 sm:grid-cols-3">{graph.nodes.slice(0, 9).map((node, index) => <div className={`rounded-lg border p-3 text-center ${index % 3 === 1 ? "border-sky-200 bg-sky-50" : "border-slate-200 bg-white"}`} key={node.id}><p className="truncate text-sm font-medium text-slate-700">{node.name}</p><p className="mt-1 text-xs text-slate-400">{node.cluster || node.entity_type}</p></div>)}</div>; }
function Bar({ label, value, total, color }: { label: string; value: number; total: number; color: string }) { const percent = total ? Math.round((value / total) * 100) : 0; return <div><div className="flex items-center justify-between text-sm"><span className="text-slate-600">{label}</span><span className="font-medium text-slate-800">{value}</span></div><div className="mt-2 h-2 overflow-hidden rounded-full bg-slate-100"><div className={`h-full rounded-full ${color}`} style={{ width: `${percent}%` }} /></div><p className="mt-1 text-xs text-slate-400">{percent}% of your vault</p></div>; }
function ClockIcon() { return <Clock3 size={17} />; }
function formatBytes(value: number) { if (value < 1024) return `${value} B`; if (value < 1024 * 1024) return `${Math.ceil(value / 1024)} KB`; return `${(value / (1024 * 1024)).toFixed(1)} MB`; }
function messageFor(cause: unknown) { return cause instanceof Error ? cause.message : String(cause); }
