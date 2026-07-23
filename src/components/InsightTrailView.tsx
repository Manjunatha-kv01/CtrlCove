import { convertFileSrc } from "@tauri-apps/api/core";
import {
  Activity,
  Check,
  Clipboard,
  FileText,
  Image as ImageIcon,
  Pause,
  Play,
  Search,
  Settings2,
  ShieldAlert,
  Terminal,
  Trash2
} from "lucide-react";
import { type FormEvent, useEffect, useMemo, useState } from "react";
import { Panel, SectionHeading } from "./AppShell";
import type {
  InsightIncident,
  InsightTrailEvent,
  InsightTrailNoteRequest,
  InsightTrailOverview,
  InsightTrailSettings
} from "../types/cymos";

const eventTypes = ["All", "Clipboard", "Terminal", "Screenshot", "Error", "Note"] as const;

export function InsightTrailView({
  overview,
  settings,
  events,
  incidents,
  onSaveSettings,
  onRecordNote,
  onResolveIncident,
  onApplyRetention
}: {
  overview: InsightTrailOverview;
  settings: InsightTrailSettings;
  events: InsightTrailEvent[];
  incidents: InsightIncident[];
  onSaveSettings: (settings: InsightTrailSettings) => Promise<void>;
  onRecordNote: (request: InsightTrailNoteRequest) => Promise<void>;
  onResolveIncident: (incidentId: number) => Promise<void>;
  onApplyRetention: () => Promise<number>;
}) {
  const [query, setQuery] = useState("");
  const [eventType, setEventType] = useState<(typeof eventTypes)[number]>("All");
  const [draft, setDraft] = useState(settings);
  const [noteTitle, setNoteTitle] = useState("");
  const [noteDetails, setNoteDetails] = useState("");
  const [noteTags, setNoteTags] = useState("");
  const [saving, setSaving] = useState(false);
  const [savingNote, setSavingNote] = useState(false);
  const [pruning, setPruning] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => setDraft(settings), [settings]);

  const filteredEvents = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return events.filter((event) => {
      const typeMatches = eventType === "All" || event.event_type === eventType;
      if (!typeMatches || !normalizedQuery) return typeMatches;
      return [event.title, event.details, event.source_application, ...event.tags]
        .join(" ")
        .toLowerCase()
        .includes(normalizedQuery);
    });
  }, [eventType, events, query]);

  async function saveSettings() {
    setSaving(true);
    setError(null);
    try {
      await onSaveSettings(draft);
      setMessage("Capture controls saved locally.");
    } catch (cause) {
      setError(messageFor(cause));
    } finally {
      setSaving(false);
    }
  }

  async function addNote(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSavingNote(true);
    setError(null);
    try {
      await onRecordNote({
        title: noteTitle,
        details: noteDetails,
        tags: parseTags(noteTags)
      });
      setNoteTitle("");
      setNoteDetails("");
      setNoteTags("");
      setMessage("Timeline note saved locally.");
    } catch (cause) {
      setError(messageFor(cause));
    } finally {
      setSavingNote(false);
    }
  }

  async function resolveIncident(incidentId: number) {
    setError(null);
    try {
      await onResolveIncident(incidentId);
      setMessage("Incident marked as resolved.");
    } catch (cause) {
      setError(messageFor(cause));
    }
  }

  async function applyRetention() {
    setPruning(true);
    setError(null);
    try {
      const removed = await onApplyRetention();
      setMessage(removed ? `${removed} expired timeline events removed.` : "No expired timeline events found.");
    } catch (cause) {
      setError(messageFor(cause));
    } finally {
      setPruning(false);
    }
  }

  return (
    <div className="grid gap-5">
      <section className="glass-hero overflow-hidden rounded-lg px-5 py-6 text-white sm:px-7 sm:py-8">
        <div className="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
          <div className="max-w-2xl">
            <div className="inline-flex items-center gap-2 rounded-md border border-white/15 bg-white/10 px-2.5 py-1 text-xs font-medium text-sky-100">
              <span className={`h-1.5 w-1.5 rounded-full ${overview.capture_state === "Active" ? "bg-emerald-400" : "bg-amber-400"}`} />
              InsightTrail {overview.capture_state.toLowerCase()}
            </div>
            <h2 className="mt-5 text-2xl font-semibold sm:text-3xl">Operational context, kept in sequence.</h2>
            <p className="mt-3 text-sm leading-6 text-slate-300">Local capture events, copied images, and incident signals stay attached to the memory that produced them.</p>
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              className="inline-flex h-10 items-center gap-2 rounded-md bg-white px-4 text-sm font-medium text-slate-950 transition-colors hover:bg-sky-50"
              onClick={() => setDraft((current) => ({ ...current, enabled: !current.enabled }))}
              type="button"
            >
              {draft.enabled ? <Pause size={16} /> : <Play size={16} />}
              {draft.enabled ? "Pause capture" : "Resume capture"}
            </button>
            <button
              className="inline-flex h-10 items-center gap-2 rounded-md border border-white/20 px-4 text-sm font-medium text-white transition-colors hover:bg-white/10"
              disabled={saving}
              onClick={() => void saveSettings()}
              type="button"
            >
              <Settings2 size={16} />
              {saving ? "Saving" : "Save controls"}
            </button>
          </div>
        </div>
      </section>

      {message ? <Feedback tone="success" message={message} /> : null}
      {error ? <Feedback tone="error" message={error} /> : null}

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <TrailMetric label="Timeline events" value={overview.event_count} supporting="Local event journal" tone="sky" />
        <TrailMetric label="Open incidents" value={overview.active_incident_count} supporting="Needs review" tone="rose" />
        <TrailMetric label="Copied images" value={overview.screenshot_count} supporting="Attached to events" tone="violet" />
        <TrailMetric label="Error signals" value={overview.error_signal_count} supporting={`${overview.retention_days} day retention`} tone="amber" />
      </div>

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_340px]">
        <Panel className="min-w-0">
          <SectionHeading title="Timeline" description="Recent local events, newest first." />
          <div className="grid gap-2 sm:grid-cols-[minmax(0,1fr)_160px]">
            <label className="flex h-10 items-center gap-2 rounded-md border border-slate-200 bg-slate-50 px-3 focus-within:border-sky-400 focus-within:bg-white focus-within:ring-2 focus-within:ring-sky-100">
              <Search className="shrink-0 text-slate-400" size={17} />
              <span className="sr-only">Search InsightTrail</span>
              <input className="min-w-0 flex-1 bg-transparent text-sm text-slate-900 outline-none placeholder:text-slate-400" onChange={(event) => setQuery(event.target.value)} placeholder="Search timeline" value={query} />
            </label>
            <select aria-label="Filter timeline event type" className="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-700 outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" onChange={(event) => setEventType(event.target.value as (typeof eventTypes)[number])} value={eventType}>
              {eventTypes.map((type) => <option key={type}>{type}</option>)}
            </select>
          </div>

          <div className="mt-5 grid gap-3">
            {filteredEvents.map((event) => <TimelineEventCard event={event} key={event.id} />)}
            {filteredEvents.length === 0 ? <EmptyTimeline hasEvents={events.length > 0} /> : null}
          </div>
        </Panel>

        <div className="grid h-fit gap-5">
          <Panel>
            <SectionHeading title="Incident memory" description="Recurring local error signals." />
            <div className="grid gap-3">
              {incidents.map((incident) => (
                <article className="rounded-md border border-slate-200 bg-slate-50 p-3" key={incident.id}>
                  <div className="flex items-start gap-2">
                    <ShieldAlert className={incident.status === "Open" ? "mt-0.5 shrink-0 text-rose-600" : "mt-0.5 shrink-0 text-emerald-600"} size={17} />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-start justify-between gap-2">
                        <p className="min-w-0 text-sm font-semibold capitalize text-slate-800">{incident.title}</p>
                        <StatusBadge status={incident.status} />
                      </div>
                      <p className="mt-1 text-xs leading-5 text-slate-500">{incident.summary}</p>
                    </div>
                  </div>
                  <ol className="mt-3 grid gap-1.5 text-xs leading-5 text-slate-600">
                    {incident.recommended_steps.slice(0, 3).map((step, index) => <li className="flex gap-2" key={step}><span className="font-semibold text-sky-700">{index + 1}</span><span>{step}</span></li>)}
                  </ol>
                  <div className="mt-3 flex items-center justify-between gap-2 border-t border-slate-200 pt-2.5">
                    <span className="text-xs text-slate-400">{incident.event_count} signals</span>
                    {incident.status === "Open" ? <button className="inline-flex h-8 items-center gap-1.5 rounded-md border border-emerald-200 bg-white px-2.5 text-xs font-medium text-emerald-700 hover:bg-emerald-50" onClick={() => void resolveIncident(incident.id)} type="button"><Check size={14} /> Resolve</button> : null}
                  </div>
                </article>
              ))}
              {incidents.length === 0 ? <p className="text-sm leading-6 text-slate-500">No incident signals recorded.</p> : null}
            </div>
          </Panel>
        </div>
      </div>

      <div className="grid gap-5 xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
        <Panel>
          <SectionHeading title="Add timeline note" description="Save a decision, observation, or handoff alongside your captured context." />
          <form className="grid gap-3" onSubmit={(event) => void addNote(event)}>
            <label className="grid gap-1.5 text-sm font-medium text-slate-700">
              Title
              <input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={120} onChange={(event) => setNoteTitle(event.target.value)} required value={noteTitle} />
            </label>
            <label className="grid gap-1.5 text-sm font-medium text-slate-700">
              Context
              <textarea className="min-h-28 resize-y rounded-md border border-slate-200 p-3 text-sm leading-6 outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={1500} onChange={(event) => setNoteDetails(event.target.value)} required value={noteDetails} />
            </label>
            <label className="grid gap-1.5 text-sm font-medium text-slate-700">
              Tags
              <input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={300} onChange={(event) => setNoteTags(event.target.value)} placeholder="nginx, incident, rhel" value={noteTags} />
            </label>
            <button className="inline-flex h-10 w-full items-center justify-center gap-2 rounded-md bg-slate-950 px-4 text-sm font-medium text-white hover:bg-slate-800 disabled:cursor-wait disabled:opacity-60 sm:w-fit" disabled={savingNote} type="submit"><FileText size={16} />{savingNote ? "Saving note" : "Save note"}</button>
          </form>
        </Panel>

        <Panel>
          <SectionHeading title="Capture controls" description="All settings are stored locally on this device." />
          <div className="grid gap-3">
            <ToggleControl checked={draft.enabled} label="InsightTrail capture" onChange={(enabled) => setDraft((current) => ({ ...current, enabled }))} />
            <ToggleControl checked={draft.capture_clipboard} label="Clipboard events" onChange={(capture_clipboard) => setDraft((current) => ({ ...current, capture_clipboard }))} />
            <ToggleControl checked={draft.capture_terminal_history} label="Imported terminal history" onChange={(capture_terminal_history) => setDraft((current) => ({ ...current, capture_terminal_history }))} />
            <ToggleControl checked={draft.capture_copied_images} label="Copied images and screenshots" onChange={(capture_copied_images) => setDraft((current) => ({ ...current, capture_copied_images }))} />
            <ToggleControl checked={draft.create_incidents} label="Error signal incident grouping" onChange={(create_incidents) => setDraft((current) => ({ ...current, create_incidents }))} />
          </div>
          <div className="mt-5 grid gap-3 sm:grid-cols-2">
            <label className="grid gap-1.5 text-sm font-medium text-slate-700">
              Retention days
              <input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" max={3650} min={1} onChange={(event) => setDraft((current) => ({ ...current, retention_days: Number(event.target.value) || 1 }))} type="number" value={draft.retention_days} />
            </label>
            <label className="grid gap-1.5 text-sm font-medium text-slate-700">
              Storage limit (MB)
              <input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" max={102400} min={64} onChange={(event) => setDraft((current) => ({ ...current, max_storage_mb: Number(event.target.value) || 64 }))} type="number" value={draft.max_storage_mb} />
            </label>
          </div>
          <label className="mt-3 grid gap-1.5 text-sm font-medium text-slate-700">
            Excluded application names
            <input className="h-10 rounded-md border border-slate-200 px-3 text-sm outline-none focus:border-sky-400 focus:ring-2 focus:ring-sky-100" maxLength={960} onChange={(event) => setDraft((current) => ({ ...current, excluded_applications: parseTags(event.target.value) }))} placeholder="Password manager, private browser" value={draft.excluded_applications.join(", ")} />
          </label>
          <div className="mt-5 flex flex-wrap gap-2">
            <button className="inline-flex h-9 items-center gap-2 rounded-md bg-slate-950 px-3 text-sm font-medium text-white hover:bg-slate-800 disabled:cursor-wait disabled:opacity-60" disabled={saving} onClick={() => void saveSettings()} type="button"><Settings2 size={15} />{saving ? "Saving" : "Save controls"}</button>
            <button className="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 px-3 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:cursor-wait disabled:opacity-60" disabled={pruning} onClick={() => void applyRetention()} type="button"><Trash2 size={15} />{pruning ? "Applying" : "Apply retention"}</button>
          </div>
          <p className="mt-4 border-t border-slate-100 pt-4 text-xs leading-5 text-slate-500">Continuous screen recording is not enabled. Copied images are recorded only when local image capture is allowed.</p>
        </Panel>
      </div>
    </div>
  );
}

function TimelineEventCard({ event }: { event: InsightTrailEvent }) {
  const Icon = eventIcon(event.event_type);
  const screenshotUrl = event.screenshot_path ? convertFileSrc(event.screenshot_path) : null;
  return (
    <article className="overflow-hidden rounded-lg border border-slate-200 bg-white shadow-sm">
      <div className="flex flex-col gap-3 p-4 sm:flex-row sm:items-start">
        <span className={`grid h-9 w-9 shrink-0 place-items-center rounded-md ${eventTone(event.event_type)}`}><Icon size={17} /></span>
        <div className="min-w-0 flex-1">
          <div className="flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between sm:gap-3">
            <div className="min-w-0">
              <p className="truncate text-sm font-semibold text-slate-800">{event.title}</p>
              <p className="mt-1 text-sm leading-6 text-slate-600">{event.details}</p>
            </div>
            <time className="shrink-0 text-xs tabular-nums text-slate-400">{event.created_at}</time>
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-2 text-xs">
            <span className="rounded-md bg-slate-100 px-2 py-1 font-medium text-slate-600">{event.event_type}</span>
            <span className={severityClass(event.severity)}>{event.severity}</span>
            <span className="truncate text-slate-400">{event.source_application}</span>
            {event.incident_id ? <span className="rounded-md bg-rose-50 px-2 py-1 font-medium text-rose-700">Incident linked</span> : null}
            {event.tags.map((tag) => <span className="rounded-md bg-sky-50 px-2 py-1 text-sky-700" key={`${event.id}-${tag}`}>{tag}</span>)}
          </div>
        </div>
      </div>
      {screenshotUrl ? <img alt="Captured local image" className="max-h-72 w-full border-t border-slate-100 bg-slate-50 object-contain" src={screenshotUrl} /> : null}
    </article>
  );
}

function ToggleControl({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) {
  return (
    <label className="flex min-h-10 cursor-pointer items-center justify-between gap-4 rounded-md border border-slate-200 bg-slate-50 px-3 text-sm font-medium text-slate-700">
      <span>{label}</span>
      <input aria-label={label} checked={checked} className="h-4 w-4 shrink-0 accent-sky-600" onChange={(event) => onChange(event.target.checked)} type="checkbox" />
    </label>
  );
}

function TrailMetric({ label, value, supporting, tone }: { label: string; value: number; supporting: string; tone: "sky" | "rose" | "violet" | "amber" }) {
  const tones = {
    sky: "border-sky-100 bg-sky-50",
    rose: "border-rose-100 bg-rose-50",
    violet: "border-violet-100 bg-violet-50",
    amber: "border-amber-100 bg-amber-50"
  };
  return <div className={`rounded-lg border p-4 ${tones[tone]}`}><p className="text-xs font-medium text-slate-500">{label}</p><p className="mt-2 text-xl font-semibold tabular-nums text-slate-950">{value}</p><p className="mt-1 truncate text-xs text-slate-500">{supporting}</p></div>;
}

function Feedback({ tone, message }: { tone: "success" | "error"; message: string }) {
  return <div className={`rounded-lg border px-4 py-3 text-sm ${tone === "success" ? "border-emerald-200 bg-emerald-50 text-emerald-900" : "border-rose-200 bg-rose-50 text-rose-800"}`}>{message}</div>;
}

function EmptyTimeline({ hasEvents }: { hasEvents: boolean }) {
  return <div className="grid min-h-52 place-items-center rounded-lg border border-dashed border-slate-200 bg-slate-50 p-6 text-center"><div><Activity className="mx-auto text-slate-300" size={28} /><p className="mt-3 text-sm font-medium text-slate-600">{hasEvents ? "No events match these filters." : "No InsightTrail events recorded yet."}</p></div></div>;
}

function StatusBadge({ status }: { status: InsightIncident["status"] }) {
  return <span className={`shrink-0 rounded-md px-2 py-1 text-[11px] font-semibold ${status === "Open" ? "bg-rose-100 text-rose-700" : "bg-emerald-100 text-emerald-700"}`}>{status}</span>;
}

function eventIcon(eventType: InsightTrailEvent["event_type"]) {
  if (eventType === "Terminal") return Terminal;
  if (eventType === "Screenshot") return ImageIcon;
  if (eventType === "Error") return ShieldAlert;
  if (eventType === "Note") return FileText;
  return Clipboard;
}

function eventTone(eventType: InsightTrailEvent["event_type"]) {
  if (eventType === "Error") return "bg-rose-50 text-rose-700";
  if (eventType === "Terminal") return "bg-violet-50 text-violet-700";
  if (eventType === "Screenshot") return "bg-sky-50 text-sky-700";
  if (eventType === "Note") return "bg-amber-50 text-amber-700";
  return "bg-slate-100 text-slate-700";
}

function severityClass(severity: InsightTrailEvent["severity"]) {
  if (severity === "Critical") return "rounded-md bg-rose-100 px-2 py-1 font-medium text-rose-800";
  if (severity === "Warning") return "rounded-md bg-amber-100 px-2 py-1 font-medium text-amber-800";
  return "rounded-md bg-emerald-50 px-2 py-1 font-medium text-emerald-700";
}

function parseTags(value: string) {
  return Array.from(new Set(value.split(",").map((item) => item.trim()).filter(Boolean))).slice(0, 12);
}

function messageFor(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}
