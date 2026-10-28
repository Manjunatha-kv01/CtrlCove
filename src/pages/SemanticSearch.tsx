import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Brain, Zap, Search, Cpu, Star, Copy, Check } from "lucide-react";
import type { ClipboardItem } from "../types";
import { CONTENT_TYPE_CONFIG, AI_CATEGORY_CONFIG } from "../types";
import { SemanticSearchBar, SearchMode } from "../components/SemanticSearchBar";
import { SimilarMemories } from "../components/SimilarMemories";

interface Props {
  onNavigate?: (view: string) => void;
}

export function SemanticSearch({ onNavigate: _ }: Props) {
  const [results, setResults]   = useState<ClipboardItem[]>([]);
  const [mode, setMode]         = useState<SearchMode>("hybrid");
  const [query, setQuery]       = useState("");
  const [hasSearched, setHasSearched] = useState(false);
  const [copiedId, setCopiedId] = useState<number | null>(null);

  const handleResults = (items: ClipboardItem[], m: SearchMode, q: string) => {
    setResults(items);
    setMode(m);
    setQuery(q);
    setHasSearched(true);
  };

  const handleClear = () => {
    setResults([]);
    setHasSearched(false);
    setQuery("");
  };

  const handleCopy = async (content: string, id: number) => {
    try {
      await invoke("write_to_clipboard", { content });
    } catch (err) {
      console.error("write_to_clipboard failed:", err);
      try {
        await navigator.clipboard.writeText(content);
      } catch (nativeErr) {
        console.error("navigator.clipboard.writeText failed:", nativeErr);
        return;
      }
    }

    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 1500);
  };

  const modeLabels: Record<SearchMode, { label: string; icon: React.ReactNode; color: string; description: string }> = {
    keyword:  { label: "Keyword",  icon: <Search size={14} />,  color: "#60a5fa", description: "Exact keyword matching" },
    semantic: { label: "Semantic", icon: <Brain size={14} />,   color: "#f472b6", description: "Search by meaning using AI embeddings" },
    hybrid:   { label: "Hybrid",   icon: <Zap size={14} />,     color: "#a78bfa", description: "Best of both — semantic + keyword" },
  };

  return (
    <div style={{ flex: 1, overflow: "hidden", display: "flex", flexDirection: "column", padding: "24px 28px" }}>
      {/* Header */}
      <div style={{ marginBottom: 24 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 6 }}>
          <div style={{
            width: 36, height: 36, borderRadius: 10,
            background: "linear-gradient(135deg, #a78bfa, #ec4899)",
            display: "flex", alignItems: "center", justifyContent: "center",
          }}>
            <Brain size={18} color="white" />
          </div>
          <div>
            <h1 style={{ fontSize: 20, fontWeight: 700, color: "var(--text-primary)", margin: 0 }}>
              Semantic Search
            </h1>
            <p style={{ fontSize: 12, color: "var(--text-muted)", margin: 0 }}>
              Search by meaning, not just keywords. Powered by local AI embeddings.
            </p>
          </div>
        </div>

        {/* Active mode chip */}
        {hasSearched && (
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
            <div style={{
              display: "inline-flex", alignItems: "center", gap: 5,
              padding: "4px 10px", borderRadius: 20,
              background: `${modeLabels[mode].color}15`,
              border: `1px solid ${modeLabels[mode].color}40`,
              color: modeLabels[mode].color, fontSize: 11, fontWeight: 600,
            }}>
              {modeLabels[mode].icon}
              {modeLabels[mode].label} mode
            </div>
            <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
              {results.length} result{results.length !== 1 ? "s" : ""} for "{query}"
            </span>
          </div>
        )}
      </div>

      {/* Search bar */}
      <div style={{ marginBottom: 20 }}>
        <SemanticSearchBar
          onResults={handleResults}
          onClear={handleClear}
          placeholder='Try: "Rust ownership notes", "Docker command", "networking architecture"…'
        />
      </div>

      {/* Empty state */}
      {!hasSearched && (
        <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 20 }}>
          {/* Mode cards */}
          <div style={{ display: "flex", gap: 12, flexWrap: "wrap", justifyContent: "center" }}>
            {(Object.entries(modeLabels) as [SearchMode, typeof modeLabels[SearchMode]][]).map(([key, cfg]) => (
              <div key={key} style={{
                padding: "16px 20px", borderRadius: 12, minWidth: 160, textAlign: "center",
                background: `${cfg.color}08`, border: `1px solid ${cfg.color}25`,
                cursor: "default",
              }}>
                <div style={{ color: cfg.color, marginBottom: 6 }}>{cfg.icon}</div>
                <div style={{ fontSize: 13, fontWeight: 700, color: cfg.color, marginBottom: 4 }}>{cfg.label}</div>
                <div style={{ fontSize: 11, color: "var(--text-muted)" }}>{cfg.description}</div>
              </div>
            ))}
          </div>
          <div style={{ textAlign: "center" }}>
            <p style={{ fontSize: 12, color: "var(--text-muted)", margin: 0 }}>
              Start typing above to search your memories
            </p>
            <p style={{ fontSize: 11, color: "var(--text-dim)", margin: "4px 0 0" }}>
              Semantic search requires Ollama + <code style={{ fontSize: 10 }}>nomic-embed-text</code> running locally
            </p>
          </div>
        </div>
      )}

      {/* No results */}
      {hasSearched && results.length === 0 && (
        <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", flexDirection: "column", gap: 8 }}>
          <Cpu size={36} style={{ color: "var(--text-dim)" }} />
          <p style={{ color: "var(--text-muted)", fontSize: 13, margin: 0 }}>No memories matched "{query}"</p>
          <p style={{ color: "var(--text-dim)", fontSize: 11, margin: 0 }}>Try different keywords or switch to Keyword mode</p>
        </div>
      )}

      {/* Results */}
      {hasSearched && results.length > 0 && (
        <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: 10 }}>
          {results.map((item, idx) => {
            const typeCfg = CONTENT_TYPE_CONFIG[item.content_type] || CONTENT_TYPE_CONFIG.Text;
            const aiCatCfg = item.ai_category ? AI_CATEGORY_CONFIG[item.ai_category] : null;
            const isImage = item.content_type === "Image";
            const preview = isImage ? null : item.content.substring(0, 300);
            const isCopied = copiedId === item.id;

            return (
              <div key={item.id} style={{
                background: "var(--bg-elevated)",
                border: "1px solid var(--border-base)",
                borderRadius: 12,
                padding: "14px 16px",
                transition: "border-color 0.2s, box-shadow 0.2s",
                position: "relative",
              }}>
                {/* Rank badge */}
                <div style={{
                  position: "absolute", top: 10, right: 12,
                  fontSize: 10, color: "var(--text-dim)", fontWeight: 600,
                }}>
                  #{idx + 1}
                </div>

                {/* Top row */}
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                  <span style={{
                    fontSize: 9, fontWeight: 700, padding: "2px 6px", borderRadius: 5,
                    background: typeCfg.bgColor, border: `1px solid ${typeCfg.borderColor}`, color: typeCfg.color,
                  }}>
                    {item.content_type}
                  </span>
                  {aiCatCfg && (
                    <span style={{
                      fontSize: 9, fontWeight: 600, padding: "2px 6px", borderRadius: 5,
                      background: aiCatCfg.bgColor, border: `1px solid ${aiCatCfg.borderColor}`, color: aiCatCfg.color,
                    }}>
                      {aiCatCfg.emoji} {item.ai_category}
                    </span>
                  )}
                  {item.is_favorite === 1 && <Star size={11} fill="#f59e0b" color="#f59e0b" />}
                  {item.embedding_status === 1 && (
                    <span title="Embedded" style={{
                      width: 7, height: 7, borderRadius: "50%",
                      background: "#a78bfa", display: "inline-block",
                    }} />
                  )}
                  <span style={{ fontSize: 10, color: "var(--text-dim)", marginLeft: "auto", paddingRight: 24 }}>
                    {item.created_at?.substring(0, 10)}
                  </span>
                </div>

                {/* Content preview */}
                {isImage ? (
                  <div style={{ fontSize: 12, color: "var(--text-muted)", fontStyle: "italic" }}>[Image]</div>
                ) : (
                  <div style={{
                    fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.6,
                    whiteSpace: "pre-wrap", wordBreak: "break-word",
                    maxHeight: 100, overflow: "hidden",
                    maskImage: preview && preview.length >= 300 ? "linear-gradient(to bottom, black 70%, transparent 100%)" : undefined,
                  }}>
                    {preview}
                  </div>
                )}

                {/* AI summary */}
                {item.ai_summary && (
                  <div style={{
                    marginTop: 8, padding: "6px 8px",
                    background: "rgba(139,92,246,0.06)", border: "1px solid rgba(139,92,246,0.15)",
                    borderRadius: 7, fontSize: 11, color: "var(--text-muted)", lineHeight: 1.5,
                  }}>
                    🧠 {item.ai_summary}
                  </div>
                )}

                {/* Actions row */}
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 10 }}>
                  <button
                    onClick={() => handleCopy(item.content, item.id)}
                    style={{
                      display: "inline-flex", alignItems: "center", gap: 5,
                      padding: "4px 10px", borderRadius: 7, cursor: "pointer", fontSize: 11,
                      background: isCopied ? "rgba(52,211,153,0.12)" : "var(--bg-surface)",
                      border: `1px solid ${isCopied ? "rgba(52,211,153,0.3)" : "var(--border-dim)"}`,
                      color: isCopied ? "#34d399" : "var(--text-secondary)",
                      transition: "all 0.2s",
                    }}
                  >
                    {isCopied ? <><Check size={11} /> Copied</> : <><Copy size={11} /> Copy</>}
                  </button>

                  {item.embedding_status === 1 && (
                    <SimilarMemories
                      itemId={item.id}
                      onCopyItem={c => handleCopy(c, -1)}
                    />
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
