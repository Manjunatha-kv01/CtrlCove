import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Search, Zap, Brain, X, Loader } from "lucide-react";
import type { ClipboardItem } from "../types";

export type SearchMode = "keyword" | "semantic" | "hybrid";

interface SemanticSearchBarProps {
  onResults: (items: ClipboardItem[], mode: SearchMode, query: string) => void;
  onClear: () => void;
  placeholder?: string;
}

export function SemanticSearchBar({ onResults, onClear, placeholder }: SemanticSearchBarProps) {
  const [query, setQuery]   = useState("");
  const [mode, setMode]     = useState<SearchMode>("hybrid");
  const [loading, setLoading] = useState(false);
  const [error, setError]   = useState<string | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const runSearch = useCallback(async (q: string, m: SearchMode) => {
    if (!q.trim()) { onClear(); return; }
    setLoading(true);
    setError(null);
    try {
      let results: ClipboardItem[];
      if (m === "semantic") {
        results = await invoke<ClipboardItem[]>("semantic_search", { query: q, limit: 30 });
      } else if (m === "keyword") {
        results = await invoke<ClipboardItem[]>("search_clipboard_items", { query: q });
      } else {
        // Hybrid: run both in parallel, merge and dedupe
        const [semantic, keyword] = await Promise.allSettled([
          invoke<ClipboardItem[]>("semantic_search", { query: q, limit: 20 }),
          invoke<ClipboardItem[]>("search_clipboard_items", { query: q }),
        ]);
        const semItems = semantic.status === "fulfilled" ? semantic.value : [];
        const kwItems  = keyword.status  === "fulfilled" ? keyword.value  : [];
        // Merge: semantic results first (ranked), then keyword-only additions
        const seen = new Set<number>(semItems.map(i => i.id));
        const extra = kwItems.filter(i => !seen.has(i.id));
        results = [...semItems, ...extra];
      }
      onResults(results, m, q);
    } catch (e: unknown) {
      // Semantic failed (Ollama offline?) — fall back to keyword
      if (m !== "keyword") {
        try {
          const fallback = await invoke<ClipboardItem[]>("search_clipboard_items", { query: q });
          onResults(fallback, "keyword", q);
          setError("Semantic offline — showing keyword results");
        } catch { setError("Search failed"); }
      } else {
        setError(String(e));
      }
    } finally {
      setLoading(false);
    }
  }, [onResults, onClear]);

  // Debounced search on query or mode change
  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => runSearch(query, mode), 350);
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current); };
  }, [query, mode, runSearch]);

  const handleClear = () => {
    setQuery("");
    setError(null);
    onClear();
    inputRef.current?.focus();
  };

  const modeConfig = {
    keyword:  { label: "Keyword", icon: <Search size={10} />,  color: "#60a5fa", title: "Exact keyword matching" },
    semantic: { label: "Semantic", icon: <Brain size={10} />,  color: "#f472b6", title: "Meaning-based search via AI embeddings" },
    hybrid:   { label: "Hybrid",  icon: <Zap size={10} />,     color: "#a78bfa", title: "Combines keyword + semantic (recommended)" },
  };
  const mc = modeConfig[mode];

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {/* Search input row */}
      <div style={{
        display: "flex", alignItems: "center", gap: 6,
        background: "var(--bg-elevated)",
        border: `1px solid ${query ? mc.color + "60" : "var(--border-base)"}`,
        borderRadius: 10,
        padding: "6px 10px",
        transition: "border-color 0.2s",
      }}>
        {loading
          ? <Loader size={14} style={{ color: mc.color, animation: "spin 1s linear infinite", flexShrink: 0 }} />
          : <Search size={14} style={{ color: "var(--text-muted)", flexShrink: 0 }} />
        }
        <input
          ref={inputRef}
          value={query}
          onChange={e => setQuery(e.target.value)}
          placeholder={placeholder ?? `Search memories... (${mode} mode)`}
          style={{
            flex: 1, background: "transparent", border: "none", outline: "none",
            color: "var(--text-primary)", fontSize: 13,
          }}
        />
        {query && (
          <button onClick={handleClear} style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-muted)", padding: 0 }}>
            <X size={13} />
          </button>
        )}
      </div>

      {/* Mode toggles + error */}
      <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
        {(["hybrid", "keyword", "semantic"] as SearchMode[]).map(m => {
          const cfg = modeConfig[m];
          const active = mode === m;
          return (
            <button
              key={m}
              onClick={() => setMode(m)}
              title={cfg.title}
              style={{
                display: "flex", alignItems: "center", gap: 4,
                padding: "3px 8px", borderRadius: 8, cursor: "pointer",
                fontSize: 10, fontWeight: active ? 700 : 500,
                background: active ? `${cfg.color}18` : "transparent",
                border: `1px solid ${active ? cfg.color + "50" : "var(--border-dim)"}`,
                color: active ? cfg.color : "var(--text-muted)",
                transition: "all 0.15s",
              }}
            >
              {cfg.icon}
              {cfg.label}
            </button>
          );
        })}
        {error && (
          <span style={{ fontSize: 10, color: "#f59e0b", marginLeft: "auto" }}>⚠ {error}</span>
        )}
      </div>
    </div>
  );
}
