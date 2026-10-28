import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Cpu, ExternalLink, Loader } from "lucide-react";
import type { ClipboardItem } from "../types";
import { CONTENT_TYPE_CONFIG } from "../types";

interface SimilarMemoriesProps {
  itemId: number;
  onCopyItem?: (content: string) => void;
}

export function SimilarMemories({ itemId, onCopyItem }: SimilarMemoriesProps) {
  const [open, setOpen]         = useState(false);
  const [items, setItems]       = useState<ClipboardItem[]>([]);
  const [loading, setLoading]   = useState(false);
  const [loaded, setLoaded]     = useState(false);
  const [error, setError]       = useState<string | null>(null);

  const load = async () => {
    if (loaded) return;
    setLoading(true);
    setError(null);
    try {
      const results = await invoke<ClipboardItem[]>("get_similar_items", { itemId, limit: 5 });
      setItems(results);
      setLoaded(true);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const toggle = () => {
    const next = !open;
    setOpen(next);
    if (next) load();
  };

  return (
    <div style={{ marginTop: 6 }}>
      <button
        onClick={toggle}
        style={{
          display: "inline-flex", alignItems: "center", gap: 5,
          background: open ? "rgba(167,139,250,0.12)" : "rgba(167,139,250,0.06)",
          border: "1px solid rgba(167,139,250,0.2)",
          borderRadius: 8, padding: "4px 8px",
          cursor: "pointer", color: "#a78bfa", fontSize: 11, fontWeight: 600,
          transition: "all 0.2s",
        }}
      >
        <Cpu size={11} />
        {open ? "Hide" : "Similar →"}
        {loading && <Loader size={10} style={{ animation: "spin 1s linear infinite" }} />}
      </button>

      {open && (
        <div style={{ marginTop: 6, display: "flex", flexDirection: "column", gap: 5 }}>
          {error && (
            <div style={{ fontSize: 11, color: "#f59e0b", padding: "4px 8px", background: "rgba(245,158,11,0.08)", borderRadius: 6, border: "1px solid rgba(245,158,11,0.2)" }}>
              {error.includes("no embedding") ? "⚡ Embedding in progress — check back shortly" : `⚠ ${error}`}
            </div>
          )}

          {!loading && !error && items.length === 0 && loaded && (
            <div style={{ fontSize: 11, color: "var(--text-muted)", padding: "4px 8px" }}>
              No similar memories found yet.
            </div>
          )}

          {items.map(item => {
            const cfg = CONTENT_TYPE_CONFIG[item.content_type] || CONTENT_TYPE_CONFIG.Text;
            const preview = item.content_type === "Image"
              ? "[Image]"
              : item.content.substring(0, 90);
            return (
              <div
                key={item.id}
                style={{
                  display: "flex", alignItems: "flex-start", gap: 8,
                  padding: "7px 9px", borderRadius: 8,
                  background: "var(--bg-elevated)",
                  border: "1px solid var(--border-dim)",
                  cursor: "pointer",
                  transition: "border-color 0.15s",
                }}
                onClick={() => onCopyItem?.(item.content)}
                title="Click to copy"
              >
                <span style={{
                  fontSize: 9, fontWeight: 700, padding: "2px 5px", borderRadius: 5,
                  flexShrink: 0, marginTop: 1,
                  background: cfg.bgColor, border: `1px solid ${cfg.borderColor}`, color: cfg.color,
                }}>
                  {item.content_type}
                </span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{
                    fontSize: 11, color: "var(--text-secondary)",
                    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                  }}>
                    {preview}
                  </div>
                  {item.ai_summary && (
                    <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 2 }}>
                      {item.ai_summary.substring(0, 70)}
                    </div>
                  )}
                </div>
                <ExternalLink size={10} style={{ color: "var(--text-muted)", flexShrink: 0, marginTop: 3 }} />
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
