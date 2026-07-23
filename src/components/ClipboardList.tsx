import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { Copy, Download, FolderInput, Sparkles, Star, Trash2 } from "lucide-react";
import type { ReactNode } from "react";
import type { ClipboardItem, ClipboardType, Collection } from "../types/cymos";

export type { ClipboardItem, ClipboardType, Collection } from "../types/cymos";

type ClipboardListProps = {
  items: ClipboardItem[];
  collections: Collection[];
  loading: boolean;
  query: string;
  onChanged: () => void;
  onFindSimilar: (itemId: number) => void;
};

export default function ClipboardList({
  items,
  collections,
  loading,
  query,
  onChanged,
  onFindSimilar
}: ClipboardListProps) {
  if (loading) {
    return (
      <div className="rounded-lg border border-slate-200 bg-white px-4 py-12 text-center text-sm text-slate-500 shadow-sm">
        Loading your memory...
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-slate-300 bg-white px-4 py-14 text-center shadow-sm">
        <p className="text-sm font-semibold text-slate-800">
          {query ? "No matching memory items." : "Copy text, code, URLs, files, colors, tables, or images."}
        </p>
      </div>
    );
  }

  return (
    <div className="grid gap-3">
      {items.map((item) => (
        <article key={item.id} className="group rounded-lg border border-slate-200 bg-white p-4 shadow-sm transition-shadow hover:shadow-md">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
            <div className="min-w-0 flex-1">
              <div className="mb-3 flex flex-wrap items-center gap-2">
                <span className={badgeClassName(item.content_type)}>{item.content_type}</span>
                {item.language ? <span className="rounded-md bg-slate-100 px-2 py-1 text-xs font-semibold text-slate-700">{item.language}</span> : null}
                <span className="rounded-md bg-sky-50 px-2 py-1 text-xs font-semibold text-sky-700">{item.category}</span>
                {item.collection_name ? (
                  <span className="rounded-md px-2 py-1 text-xs font-semibold text-white" style={{ backgroundColor: item.collection_color ?? "#52525b" }}>
                    {item.collection_name}
                  </span>
                ) : null}
                {item.is_favorite ? <span className="rounded-md bg-amber-50 px-2 py-1 text-xs font-semibold text-amber-700">Favorite</span> : null}
                <span className="rounded-md bg-emerald-50 px-2 py-1 text-xs font-semibold text-emerald-700">
                  {(item.semantic_score * 100).toFixed(0)}% {item.rank_reason}
                </span>
                <time className="ml-auto text-xs tabular-nums text-slate-400">{item.created_at}</time>
              </div>

              <Preview item={item} />

              <div className="mt-3 rounded-md border border-sky-100 bg-sky-50/70 px-3 py-2.5">
                <p className="text-xs font-semibold uppercase tracking-[0.14em] text-sky-700">Memory insight</p>
                <p className="mt-1 text-sm leading-6 text-slate-800">{item.ai_summary || "No summary yet."}</p>
              </div>

              <div className="mt-3 flex flex-wrap gap-2">
                {item.keywords.map((keyword) => (
                  <span key={`${item.id}-keyword-${keyword}`} className="rounded-md bg-sky-100 px-2 py-1 text-xs font-medium text-sky-800">
                    {keyword}
                  </span>
                ))}
                {item.tags.map((tag) => (
                  <span key={`${item.id}-${tag}`} className="rounded-md bg-slate-100 px-2 py-1 text-xs text-slate-600">
                    {tag}
                  </span>
                ))}
              </div>

              {item.operational_context.kind ? (
                <div className="mt-3 flex flex-wrap gap-2">
                  <span className="rounded-md bg-violet-50 px-2 py-1 text-xs font-medium text-violet-700">{item.operational_context.kind}</span>
                  {item.operational_context.services.slice(0, 3).map((service) => (
                    <span className="rounded-md bg-slate-100 px-2 py-1 text-xs text-slate-600" key={`${item.id}-service-${service}`}>{service}</span>
                  ))}
                  {item.operational_context.hostnames.slice(0, 2).map((host) => (
                    <span className="rounded-md bg-slate-100 px-2 py-1 font-mono text-xs text-slate-600" key={`${item.id}-host-${host}`}>{host}</span>
                  ))}
                  {item.operational_context.ip_addresses.slice(0, 2).map((address) => (
                    <span className="rounded-md bg-slate-100 px-2 py-1 font-mono text-xs text-slate-600" key={`${item.id}-ip-${address}`}>{address}</span>
                  ))}
                </div>
              ) : null}

              <dl className="mt-3 grid gap-2 text-xs text-slate-500 sm:grid-cols-2 xl:grid-cols-4">
                <Meta label="Chars" value={item.character_count.toString()} />
                <Meta label="Words" value={item.word_count.toString()} />
                <Meta label="Read" value={`${item.reading_time_minutes} min`} />
                <Meta label="Copies" value={item.copy_count.toString()} />
                <Meta label="Vector" value={item.embedding_source} />
                <Meta label="Size" value={formatBytes(item.file_size)} />
                <Meta label="Source" value={item.source_application} />
              </dl>
            </div>

            <div className="flex min-w-[216px] flex-row flex-wrap gap-2 lg:flex-col lg:items-stretch">
              <div className="flex gap-2">
                <IconButton label="Copy to clipboard" onClick={() => void copyItem(item.id)}>
                  <Copy size={16} />
                </IconButton>
                <IconButton label={item.is_favorite ? "Remove from favorites" : "Add to favorites"} onClick={() => void mutate("toggle_favorite", { itemId: item.id }, onChanged)}>
                  <Star size={16} className={item.is_favorite ? "fill-amber-400 text-amber-500" : undefined} />
                </IconButton>
                <IconButton label="Find similar memories" onClick={() => onFindSimilar(item.id)}>
                  <Sparkles size={16} />
                </IconButton>
                <IconButton label="Delete memory" danger onClick={() => void mutate("delete_clipboard_item", { itemId: item.id }, onChanged)}>
                  <Trash2 size={16} />
                </IconButton>
              </div>
              <select
                aria-label="Move memory to collection"
                className="h-9 min-w-0 flex-1 rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-700 outline-none focus:border-sky-500 focus:ring-2 focus:ring-sky-100"
                value={item.collection_id ?? ""}
                onChange={(event) =>
                  void mutate(
                    "move_item_to_collection",
                    {
                      itemId: item.id,
                      collectionId: event.target.value ? Number(event.target.value) : null
                    },
                    onChanged
                  )
                }
              >
                <option value="">No collection</option>
                {collections.map((collection) => (
                  <option key={collection.id} value={collection.id}>
                    {collection.name}
                  </option>
                ))}
              </select>
              <div className="flex gap-2">
                <button className="inline-flex h-9 flex-1 items-center justify-center gap-1.5 rounded-md border border-slate-200 px-2 text-xs font-medium text-slate-700 hover:bg-slate-50" onClick={() => void exportItem(item.id, "JSON")}>
                  <Download size={14} /> JSON
                </button>
                <button className="inline-flex h-9 flex-1 items-center justify-center gap-1.5 rounded-md border border-slate-200 px-2 text-xs font-medium text-slate-700 hover:bg-slate-50" onClick={() => void exportItem(item.id, "Markdown")}>
                  <FolderInput size={14} /> Markdown
                </button>
              </div>
            </div>
          </div>
        </article>
      ))}
    </div>
  );
}

function Preview({ item }: { item: ClipboardItem }) {
  if (item.content_type === "Image") {
    return (
      <div className="overflow-hidden rounded-md border border-slate-200 bg-slate-50">
        <img className="max-h-80 w-full object-contain" src={convertFileSrc(item.content)} alt={`Clipboard image ${item.id}`} />
      </div>
    );
  }

  if (item.content_type === "Color") {
    return (
      <div className="flex items-center gap-3">
        <span className="h-12 w-12 rounded-md border border-slate-200" style={{ backgroundColor: item.content }} />
        <code className="font-mono text-sm text-slate-900">{item.content}</code>
      </div>
    );
  }

  return (
    <pre className="max-h-52 overflow-auto whitespace-pre-wrap break-words rounded-md bg-slate-950 p-3 font-mono text-sm leading-6 text-slate-50">
      {item.content}
    </pre>
  );
}

function Meta({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md bg-slate-50 px-2 py-1">
      <dt className="font-medium text-slate-400">{label}</dt>
      <dd className="truncate text-slate-700">{value}</dd>
    </div>
  );
}

function badgeClassName(type: ClipboardItem["content_type"]) {
  const base = "rounded-md px-2 py-1 text-xs font-semibold";
  const styles: Record<string, string> = {
    URL: "bg-sky-50 text-sky-700",
    Code: "bg-indigo-50 text-indigo-700",
    Image: "bg-sky-50 text-sky-700",
    File: "bg-blue-50 text-blue-700",
    Folder: "bg-indigo-50 text-indigo-700",
    Color: "bg-amber-50 text-amber-800",
    Table: "bg-emerald-50 text-emerald-700",
    HTML: "bg-orange-50 text-orange-700",
    Text: "bg-slate-100 text-slate-700"
  };
  return `${base} ${styles[type] ?? styles.Text}`;
}

function IconButton({
  label,
  danger = false,
  children,
  onClick
}: {
  label: string;
  danger?: boolean;
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      aria-label={label}
      className={`inline-flex h-9 w-9 items-center justify-center rounded-md border transition-colors ${
        danger
          ? "border-rose-200 text-rose-600 hover:bg-rose-50"
          : "border-slate-200 text-slate-600 hover:border-sky-200 hover:bg-sky-50 hover:text-sky-700"
      }`}
      onClick={onClick}
      title={label}
      type="button"
    >
      {children}
    </button>
  );
}

async function mutate(command: string, args: Record<string, unknown>, onChanged: () => void) {
  await invoke(command, args);
  onChanged();
}

async function copyItem(itemId: number) {
  await invoke("copy_clipboard_item", { itemId });
}

async function exportItem(itemId: number, format: string) {
  await invoke("export_clipboard_item", { itemId, format });
}

function formatBytes(value: number | null) {
  if (!value) {
    return "0 B";
  }
  if (value < 1024) {
    return `${value} B`;
  }
  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`;
  }
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}
