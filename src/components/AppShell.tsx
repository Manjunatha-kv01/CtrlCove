import {
  Activity,
  Bot,
  BrainCircuit,
  Clock3,
  Command,
  Database,
  Globe2,
  History,
  LayoutDashboard,
  Map,
  Network,
  Briefcase,
  Settings2,
  Sparkles,
  Terminal
} from "lucide-react";
import type { ReactNode } from "react";

export type WorkspaceView = "overview" | "memory" | "operations" | "workspace" | "trail" | "assistant" | "agents" | "automation" | "graph" | "platform" | "insights";

const navigation: Array<{ id: WorkspaceView; label: string; icon: typeof LayoutDashboard }> = [
  { id: "overview", label: "Command center", icon: LayoutDashboard },
  { id: "memory", label: "Memory vault", icon: History },
  { id: "operations", label: "Operations", icon: Terminal },
  { id: "workspace", label: "Workspace", icon: Briefcase },
  { id: "trail", label: "InsightTrail", icon: Map },
  { id: "assistant", label: "Memory assistant", icon: BrainCircuit },
  { id: "agents", label: "Agent studio", icon: Bot },
  { id: "automation", label: "Automation", icon: Activity },
  { id: "graph", label: "Knowledge graph", icon: Network },
  { id: "platform", label: "Platform", icon: Globe2 },
  { id: "insights", label: "Insights", icon: Sparkles }
];

const viewMeta: Record<WorkspaceView, { eyebrow: string; title: string; description: string }> = {
  overview: {
    eyebrow: "CYMOS v1.0",
    title: "Command center",
    description: "A private, local-first view of the knowledge you are building."
  },
  memory: {
    eyebrow: "Memory vault",
    title: "Everything you captured",
    description: "Search, organize, and retrieve the moments worth keeping."
  },
  operations: {
    eyebrow: "Operational knowledge",
    title: "Operations memory",
    description: "Commands, incidents, configuration, and infrastructure context captured locally."
  },
  workspace: {
    eyebrow: "Cognitive workspace",
    title: "Work session memory",
    description: "Keep the project, timeline, decisions, and incidents that belong together in one local context."
  },
  trail: {
    eyebrow: "InsightTrail",
    title: "Operational memory trail",
    description: "A local timeline for captured context, copied images, and operational incident signals."
  },
  assistant: {
    eyebrow: "Memory assistant",
    title: "Ask your knowledge",
    description: "Answers are grounded in the memories stored on this device."
  },
  agents: {
    eyebrow: "Agent studio",
    title: "Turn context into work",
    description: "Plan a focused task against your saved knowledge."
  },
  automation: {
    eyebrow: "Automation",
    title: "A healthy memory, quietly maintained",
    description: "Review the background work that keeps CYMOS current."
  },
  graph: {
    eyebrow: "Knowledge graph",
    title: "See the connections",
    description: "Topics, entities, and relationships discovered in your memory."
  },
  platform: {
    eyebrow: "Universal platform",
    title: "Your memory, wherever you work",
    description: "Device readiness, integrations, APIs, and security posture."
  },
  insights: {
    eyebrow: "Knowledge insights",
    title: "Notice what is growing",
    description: "Daily signals and learning patterns from your personal knowledge base."
  }
};

export function AppShell({
  activeView,
  onViewChange,
  children,
  memoryCount,
  onRefresh,
  refreshing
}: {
  activeView: WorkspaceView;
  onViewChange: (view: WorkspaceView) => void;
  children: ReactNode;
  memoryCount: number;
  onRefresh: () => void;
  refreshing: boolean;
}) {
  const meta = viewMeta[activeView];

  return (
    <main className="cymos-app min-h-screen text-slate-950">
      <div className="mx-auto flex min-h-screen max-w-[1600px]">
        <aside className="glass-sidebar hidden w-64 shrink-0 flex-col border-r px-4 py-5 lg:flex">
          <button className="flex items-center gap-3 px-2 text-left" onClick={() => onViewChange("overview")} type="button">
            <span className="grid h-9 w-9 place-items-center rounded-lg bg-slate-950 text-white">
              <Command size={18} strokeWidth={2.25} />
            </span>
            <span>
              <span className="block text-sm font-bold tracking-[0.08em] text-slate-950">CYMOS</span>
              <span className="block text-xs text-slate-500">Personal memory OS</span>
            </span>
          </button>

          <nav className="mt-9 grid gap-1" aria-label="Main navigation">
            {navigation.map(({ id, label, icon: Icon }) => {
              const isActive = id === activeView;
              return (
                <button
                  aria-current={isActive ? "page" : undefined}
                  className={`flex h-10 items-center gap-3 rounded-md px-3 text-sm font-medium transition-colors ${
                    isActive ? "bg-slate-950 text-white" : "text-slate-600 hover:bg-white/70 hover:text-slate-950"
                  }`}
                  key={id}
                  onClick={() => onViewChange(id)}
                  type="button"
                >
                  <Icon size={17} strokeWidth={isActive ? 2.2 : 1.8} />
                  {label}
                </button>
              );
            })}
          </nav>

          <div className="glass-inset mt-auto rounded-lg border p-3">
            <div className="flex items-center justify-between gap-3">
              <span className="text-xs font-medium text-slate-600">Local vault</span>
              <span className="inline-flex h-2 w-2 rounded-full bg-emerald-500" title="Capture service active" />
            </div>
            <p className="mt-2 text-2xl font-semibold tabular-nums text-slate-950">{memoryCount}</p>
            <p className="text-xs text-slate-500">saved memories</p>
          </div>

          <button className="glass-interactive mt-3 flex h-9 items-center gap-2 rounded-md border px-3 text-sm text-slate-600" type="button" title="Application settings">
            <Settings2 size={16} />
            Settings
          </button>
        </aside>

        <section className="min-w-0 flex-1">
          <header className="glass-header sticky top-0 z-10 border-b px-5 py-4 sm:px-8">
            <div className="mx-auto flex max-w-[1280px] items-center justify-between gap-4">
              <div className="min-w-0">
                <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.16em] text-sky-700">
                  <span className="lg:hidden">CYMOS</span>
                  <span className="hidden lg:inline">{meta.eyebrow}</span>
                  <span className="h-1 w-1 rounded-full bg-sky-600" />
                  <span className="normal-case tracking-normal text-slate-400">Offline and private</span>
                </div>
                <h1 className="mt-1 truncate text-xl font-semibold text-slate-950 sm:text-2xl">{meta.title}</h1>
              </div>
              <button
                aria-label="Refresh workspace"
                className="glass-interactive inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-md border text-slate-600 transition-colors hover:text-sky-700 disabled:cursor-wait disabled:opacity-60"
                disabled={refreshing}
                onClick={onRefresh}
                title="Refresh workspace"
                type="button"
              >
                <Clock3 size={16} className={refreshing ? "animate-spin" : undefined} />
              </button>
            </div>

            <nav className="mx-auto mt-4 flex max-w-[1280px] gap-1 overflow-x-auto pb-1 lg:hidden" aria-label="Main navigation">
              {navigation.map(({ id, label, icon: Icon }) => {
                const isActive = id === activeView;
                return (
                  <button
                    aria-current={isActive ? "page" : undefined}
                    className={`inline-flex h-9 shrink-0 items-center gap-2 rounded-md px-3 text-sm font-medium transition-colors ${
                      isActive
                        ? "bg-slate-950 text-white"
                        : "glass-interactive border text-slate-600 hover:text-sky-700"
                    }`}
                    key={id}
                    onClick={() => onViewChange(id)}
                    title={label}
                    type="button"
                  >
                    <Icon size={16} strokeWidth={isActive ? 2.2 : 1.8} />
                    {label}
                  </button>
                );
              })}
            </nav>
          </header>

          <div className="mx-auto max-w-[1280px] px-5 py-6 sm:px-8 sm:py-8">
            <p className="mb-6 max-w-2xl text-sm leading-6 text-slate-500">{meta.description}</p>
            {children}
          </div>
        </section>
      </div>
    </main>
  );
}

export function SectionHeading({ title, description, action }: { title: string; description?: string; action?: ReactNode }) {
  return (
    <div className="mb-4 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <h2 className="text-base font-semibold text-slate-950">{title}</h2>
        {description ? <p className="mt-1 text-sm text-slate-500">{description}</p> : null}
      </div>
      {action}
    </div>
  );
}

export function Panel({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <section className={`glass-panel rounded-lg border p-4 ${className}`}>{children}</section>;
}
