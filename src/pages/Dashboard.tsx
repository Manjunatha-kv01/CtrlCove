import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Statistics, ClipboardItem, MemoryInsights, EmbeddingStatus } from "../types";
import { CONTENT_TYPE_CONFIG, AI_CATEGORY_CONFIG } from "../types";
import {
  Clipboard, Star, FolderOpen, Tag,
  FileText, Code, Link, Image,
  ArrowRight, Brain,
} from "lucide-react";

interface DashboardProps {
  onNavigate: (view: string) => void;
}

export function Dashboard({ onNavigate }: DashboardProps) {
  const [stats, setStats] = useState<Statistics | null>(null);
  const [recent, setRecent] = useState<ClipboardItem[]>([]);
  const [insights, setInsights] = useState<MemoryInsights | null>(null);
  const [embedStatus, setEmbedStatus] = useState<EmbeddingStatus | null>(null);
  const [loading, setLoading] = useState(true);

  const loadData = async () => {
    setLoading(true);
    try {
      const [s, r, ins, e] = await Promise.all([
        invoke<Statistics>("get_statistics"),
        invoke<ClipboardItem[]>("get_clipboard_items", { limit: 5 }),
        invoke<MemoryInsights>("get_memory_insights").catch(() => null),
        invoke<EmbeddingStatus>("get_embedding_status").catch(() => null),
      ]);
      setStats(s);
      setRecent(r);
      setInsights(ins);
      setEmbedStatus(e);
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
    // Refresh when AI enrichment fires
    let unsub: (() => void) | null = null;
    listen("ai-enriched", () => {
      // Debounce: don't reload on every single enrichment
    }).then((fn) => { unsub = fn; });
    return () => { if (unsub) unsub(); };
  }, []);


  const formatTime = (ts: string) => {
    try {
      const d = new Date(ts.replace(" ", "T") + "Z");
      if (isNaN(d.getTime())) return ts;
      const now = new Date();
      const diff = (now.getTime() - d.getTime()) / 1000;
      if (diff < 60) return "just now";
      if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
      if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
      return d.toLocaleDateString([], { month: "short", day: "numeric" });
    } catch { return ts; }
  };

  if (loading) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", flexDirection: "column", gap: 12 }}>
        <div style={{ width: 24, height: 24, borderRadius: "50%", border: "2px solid var(--border-base)", borderTopColor: "#3b82f6", animation: "ping 0.8s linear infinite" }} />
        <span style={{ fontSize: 12, color: "var(--text-muted)" }}>Loading dashboard...</span>
      </div>
    );
  }

  const statCards = [
    { label: "Total Items", value: stats?.total_items ?? 0, icon: <Clipboard size={16} />, color: "#3b82f6", view: "history" },
    { label: "Favorites",   value: stats?.favorites_count ?? 0, icon: <Star size={16} />, color: "#f59e0b", view: "favorites" },
    { label: "Code Snippets", value: stats?.code_items ?? 0, icon: <Code size={16} />, color: "#a78bfa", view: "history" },
    { label: "URLs",         value: stats?.url_items ?? 0, icon: <Link size={16} />, color: "#34d399", view: "history" },
    { label: "Images",       value: stats?.image_items ?? 0, icon: <Image size={16} />, color: "#f59e0b", view: "history" },
    { label: "Collections",  value: stats?.collections_count ?? 0, icon: <FolderOpen size={16} />, color: "#38bdf8", view: "collections" },
    { label: "Tags",         value: stats?.tags_count ?? 0, icon: <Tag size={16} />, color: "#fb923c", view: "tags" },
    { label: "AI Enriched",  value: stats?.ai_processed_count ?? 0, icon: <Brain size={16} />, color: "#f472b6", view: "statistics" },
    { label: "Embedded",     value: embedStatus?.total_embedded ?? 0, icon: <Brain size={16} />, color: "#a78bfa", view: "semantic" },
  ];

  const typeBreakdown = [
    { label: "Text",  count: stats?.text_items ?? 0,  color: CONTENT_TYPE_CONFIG.Text.color },
    { label: "URL",   count: stats?.url_items ?? 0,   color: CONTENT_TYPE_CONFIG.URL.color },
    { label: "Code",  count: stats?.code_items ?? 0,  color: CONTENT_TYPE_CONFIG.Code.color },
    { label: "Image", count: stats?.image_items ?? 0, color: CONTENT_TYPE_CONFIG.Image.color },
    { label: "HTML",  count: stats?.html_items ?? 0,  color: CONTENT_TYPE_CONFIG.HTML.color },
    { label: "JSON",  count: stats?.json_items ?? 0,  color: CONTENT_TYPE_CONFIG.JSON.color },
    { label: "Email", count: stats?.email_items ?? 0, color: CONTENT_TYPE_CONFIG.Email.color },
    { label: "Color", count: stats?.color_items ?? 0, color: CONTENT_TYPE_CONFIG.Color.color },
  ].filter((t) => t.count > 0);

  const total = stats?.total_items || 1;

  return (
    <div style={{ height: "100%", overflowY: "auto" }}>
      <div style={{ padding: 20, maxWidth: 800, margin: "0 auto" }}>
        {/* Welcome */}
        <div style={{ marginBottom: 20, display: "flex", justifyContent: "space-between", alignItems: "flex-end" }}>
          <div>
            <div className="page-title" style={{ marginBottom: 4 }}>
              Welcome to CYMOS
            </div>
            <div className="page-subtitle">
              Your personal Digital Memory Engine — capturing everything, always.
            </div>
          </div>
          <button
            className="btn"
            onClick={() => onNavigate("semantic")}
            style={{
              gap: 6, padding: "8px 16px",
              background: "linear-gradient(135deg, rgba(167,139,250,0.15), rgba(236,72,153,0.15))",
              border: "1px solid rgba(167,139,250,0.3)", color: "#a78bfa"
            }}
          >
            <Brain size={14} /> Semantic Search
          </button>
        </div>

        {/* Stat Grid */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(170px, 1fr))", gap: 10, marginBottom: 20 }}>
          {statCards.map((card) => (
            <button
              key={card.label}
              className="stat-card"
              onClick={() => onNavigate(card.view)}
              style={{ cursor: "pointer", textAlign: "left", border: "none" }}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <div style={{
                  width: 32, height: 32, borderRadius: 8,
                  background: `${card.color}18`,
                  border: `1px solid ${card.color}30`,
                  display: "flex", alignItems: "center", justifyContent: "center",
                  color: card.color,
                }}>
                  {card.icon}
                </div>
                <ArrowRight size={12} style={{ color: "var(--text-muted)" }} />
              </div>
              <div className="stat-number" style={{ color: card.color, fontSize: 22 }}>
                {typeof card.value === "number" ? card.value.toLocaleString() : card.value}
              </div>
              <div className="stat-label">{card.label}</div>
            </button>
          ))}
        </div>

        {/* Content Breakdown */}
        {typeBreakdown.length > 0 && (
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", padding: 16, marginBottom: 20,
          }}>
            <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 12, textTransform: "uppercase", letterSpacing: "0.06em" }}>
              Content Breakdown
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {typeBreakdown.map((t) => (
                <div key={t.label} style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <div style={{ fontSize: 11, color: t.color, width: 48, flexShrink: 0, fontWeight: 600 }}>{t.label}</div>
                  <div className="progress-bar" style={{ flex: 1 }}>
                    <div
                      className="progress-fill"
                      style={{ width: `${Math.max(2, (t.count / total) * 100)}%`, background: t.color }}
                    />
                  </div>
                  <div style={{ fontSize: 11, color: "var(--text-muted)", width: 32, textAlign: "right" }}>{t.count}</div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* AI Memory Insights */}
        {insights && (insights.top_categories.length > 0 || insights.top_keywords.length > 0) && (
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", padding: 16, marginBottom: 20,
          }}>
            <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 14 }}>
              <Brain size={13} style={{ color: "#f472b6" }} />
              <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.06em" }}>
                AI Memory Insights
              </div>
              <span style={{
                fontSize: 9, fontWeight: 700, padding: "1px 6px", borderRadius: 8,
                background: "rgba(244,114,182,0.12)", border: "1px solid rgba(244,114,182,0.25)",
                color: "#f472b6"
              }}>
                {insights.ai_processed_count} enriched
              </span>
            </div>

            {/* Category Bars */}
            {insights.top_categories.length > 0 && (
              <div style={{ marginBottom: 14 }}>
                <div style={{ fontSize: 10, color: "var(--text-muted)", fontWeight: 600, marginBottom: 8, textTransform: "uppercase", letterSpacing: "0.05em" }}>Top Categories</div>
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {insights.top_categories.map((cat) => {
                    const cfg = AI_CATEGORY_CONFIG[cat.category] || AI_CATEGORY_CONFIG["Other"];
                    const maxCount = insights.top_categories[0]?.count || 1;
                    return (
                      <div key={cat.category} style={{ display: "flex", alignItems: "center", gap: 10 }}>
                        <div style={{ fontSize: 10, color: cfg.color, width: 88, flexShrink: 0, fontWeight: 700 }}>
                          {cfg.emoji} {cat.category}
                        </div>
                        <div className="progress-bar" style={{ flex: 1 }}>
                          <div className="progress-fill" style={{ width: `${Math.max(3, (cat.count / maxCount) * 100)}%`, background: cfg.color }} />
                        </div>
                        <div style={{ fontSize: 10, color: "var(--text-muted)", width: 28, textAlign: "right" }}>{cat.count}</div>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            {/* Keyword Cloud */}
            {insights.top_keywords.length > 0 && (
              <div>
                <div style={{ fontSize: 10, color: "var(--text-muted)", fontWeight: 600, marginBottom: 8, textTransform: "uppercase", letterSpacing: "0.05em" }}>Top Keywords</div>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 5 }}>
                  {insights.top_keywords.slice(0, 16).map((kw, i) => (
                    <span key={kw} style={{
                      fontSize: 10, fontWeight: 600,
                      padding: "2px 8px", borderRadius: 12,
                      background: `rgba(96,165,250,${0.08 + (16 - i) * 0.005})`,
                      border: "1px solid rgba(96,165,250,0.2)",
                      color: "#60a5fa",
                    }}>{kw}</span>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {/* Recent Items */}
        {recent.length > 0 && (
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", padding: 16,
          }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 12 }}>
              <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.06em" }}>
                Recent Items
              </div>
              <button
                onClick={() => onNavigate("history")}
                className="btn btn-ghost"
                style={{ fontSize: 11, padding: "3px 8px", display: "flex", alignItems: "center", gap: 4 }}
              >
                View all <ArrowRight size={11} />
              </button>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              {recent.map((item) => {
                const cfg = CONTENT_TYPE_CONFIG[item.content_type] || CONTENT_TYPE_CONFIG.Text;
                return (
                  <div key={item.id} style={{
                    display: "flex", alignItems: "center", gap: 10,
                    padding: "8px 10px", borderRadius: 8,
                    background: "var(--bg-elevated)",
                    border: "1px solid var(--border-dim)",
                  }}>
                    <span className="type-badge" style={{ color: cfg.color, background: cfg.bgColor, borderColor: cfg.borderColor, flexShrink: 0 }}>
                      {item.content_type}
                    </span>
                    <span style={{ fontSize: 12, color: "var(--text-secondary)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {item.content_type === "Image" ? item.content : item.content.substring(0, 80)}
                    </span>
                    <span style={{ fontSize: 10, color: "var(--text-muted)", flexShrink: 0 }}>
                      {formatTime(item.created_at)}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {recent.length === 0 && (
          <div className="empty-state">
            <div className="empty-icon"><FileText size={24} strokeWidth={1.5} /></div>
            <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text-secondary)" }}>Your memory is empty</div>
            <div style={{ fontSize: 12, color: "var(--text-muted)" }}>Start copying things — CYMOS will capture them automatically.</div>
          </div>
        )}
      </div>
    </div>
  );
}

export default Dashboard;
