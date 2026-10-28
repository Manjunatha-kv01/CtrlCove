import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Statistics } from "../types";
import { CONTENT_TYPE_CONFIG } from "../types";
import {
  Clipboard, Star, FolderOpen, Tag, Code, Link,
  Image, FileText, Database, Hash, Mail, Palette,
  BarChart2,
} from "lucide-react";

export function StatisticsPage() {
  const [stats, setStats] = useState<Statistics | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<Statistics>("get_statistics")
      .then(setStats)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  const formatBytes = (b: number) => {
    if (b < 1024) return `${b} B`;
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
    return `${(b / 1024 / 1024).toFixed(1)} MB`;
  };

  if (loading) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", flexDirection: "column", gap: 12 }}>
        <div style={{ width: 24, height: 24, borderRadius: "50%", border: "2px solid var(--border-base)", borderTopColor: "#3b82f6", animation: "ping 0.8s linear infinite" }} />
        <span style={{ fontSize: 12, color: "var(--text-muted)" }}>Calculating statistics...</span>
      </div>
    );
  }

  if (!stats) return null;

  const typeStats = [
    { label: "Text",   count: stats.text_items,   color: CONTENT_TYPE_CONFIG.Text.color,  icon: <FileText size={14} /> },
    { label: "URLs",   count: stats.url_items,    color: CONTENT_TYPE_CONFIG.URL.color,   icon: <Link size={14} /> },
    { label: "Code",   count: stats.code_items,   color: CONTENT_TYPE_CONFIG.Code.color,  icon: <Code size={14} /> },
    { label: "Images", count: stats.image_items,  color: CONTENT_TYPE_CONFIG.Image.color, icon: <Image size={14} /> },
    { label: "HTML",   count: stats.html_items,   color: CONTENT_TYPE_CONFIG.HTML.color,  icon: <FileText size={14} /> },
    { label: "JSON",   count: stats.json_items,   color: CONTENT_TYPE_CONFIG.JSON.color,  icon: <Hash size={14} /> },
    { label: "Emails", count: stats.email_items,  color: CONTENT_TYPE_CONFIG.Email.color, icon: <Mail size={14} /> },
    { label: "Colors", count: stats.color_items,  color: CONTENT_TYPE_CONFIG.Color.color, icon: <Palette size={14} /> },
  ];

  const total = stats.total_items || 1;
  const maxCount = Math.max(...typeStats.map((t) => t.count), 1);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div className="page-header">
        <div className="page-title">Statistics</div>
        <div className="page-subtitle">Analytics for your digital memory</div>
      </div>

      <div style={{ flex: 1, overflowY: "auto" }}>
        <div style={{ padding: 16, maxWidth: 800, margin: "0 auto" }}>

          {/* Primary Stats Row */}
          <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 10, marginBottom: 16 }}>
            {[
              { label: "Total Captured", value: stats.total_items, color: "#3b82f6", icon: <Clipboard size={16} /> },
              { label: "Favorites",      value: stats.favorites_count, color: "#f59e0b", icon: <Star size={16} /> },
              { label: "Collections",    value: stats.collections_count, color: "#38bdf8", icon: <FolderOpen size={16} /> },
              { label: "Tags",           value: stats.tags_count, color: "#fb923c", icon: <Tag size={16} /> },
            ].map((card) => (
              <div key={card.label} className="stat-card">
                <div style={{
                  width: 32, height: 32, borderRadius: 8,
                  background: `${card.color}18`, border: `1px solid ${card.color}30`,
                  display: "flex", alignItems: "center", justifyContent: "center",
                  color: card.color,
                }}>
                  {card.icon}
                </div>
                <div className="stat-number" style={{ color: card.color, fontSize: 24 }}>
                  {card.value.toLocaleString()}
                </div>
                <div className="stat-label">{card.label}</div>
              </div>
            ))}
          </div>

          {/* Storage Card */}
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", padding: 16, marginBottom: 16,
          }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
              <Database size={16} style={{ color: "#94a3b8" }} />
              <span style={{ fontSize: 13, fontWeight: 600, color: "var(--text-secondary)" }}>Storage Used</span>
            </div>
            <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginBottom: 10 }}>
              <span style={{ fontSize: 28, fontWeight: 800, color: "#94a3b8", letterSpacing: "-0.03em" }}>
                {formatBytes(stats.storage_bytes)}
              </span>
              <span style={{ fontSize: 12, color: "var(--text-muted)" }}>in local SQLite database</span>
            </div>
            <div className="progress-bar" style={{ height: 6 }}>
              <div
                className="progress-fill"
                style={{
                  width: `${Math.min(100, (stats.storage_bytes / (50 * 1024 * 1024)) * 100)}%`,
                  background: "linear-gradient(90deg, #3b82f6, #8b5cf6)",
                }}
              />
            </div>
            <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 4 }}>
              of 50 MB guideline
            </div>
          </div>

          {/* Content Type Breakdown */}
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", padding: 16, marginBottom: 16,
          }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 14 }}>
              <BarChart2 size={16} style={{ color: "var(--text-muted)" }} />
              <span style={{ fontSize: 13, fontWeight: 600, color: "var(--text-secondary)" }}>Content Breakdown</span>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {typeStats.map((t) => (
                <div key={t.label} style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, width: 80, flexShrink: 0 }}>
                    <span style={{ color: t.color }}>{t.icon}</span>
                    <span style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 500 }}>{t.label}</span>
                  </div>
                  <div className="progress-bar" style={{ flex: 1, height: 6 }}>
                    <div
                      className="progress-fill"
                      style={{
                        width: `${Math.max(t.count === 0 ? 0 : 2, (t.count / maxCount) * 100)}%`,
                        background: t.color,
                        opacity: t.count === 0 ? 0 : 1,
                      }}
                    />
                  </div>
                  <div style={{ fontSize: 12, color: t.count > 0 ? t.color : "var(--text-muted)", width: 48, textAlign: "right", fontWeight: 600 }}>
                    {t.count}
                    {total > 0 && t.count > 0 && (
                      <span style={{ fontSize: 10, fontWeight: 400, color: "var(--text-muted)", marginLeft: 3 }}>
                        ({Math.round((t.count / total) * 100)}%)
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Summary table */}
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", padding: 16,
          }}>
            <div style={{ fontSize: 13, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 12 }}>
              Quick Summary
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "6px 20px" }}>
              {[
                ["Total Items",      stats.total_items.toLocaleString()],
                ["Storage Used",     formatBytes(stats.storage_bytes)],
                ["Favorites",        stats.favorites_count.toLocaleString()],
                ["Collections",      stats.collections_count.toLocaleString()],
                ["Tags",             stats.tags_count.toLocaleString()],
                ["Code Snippets",    stats.code_items.toLocaleString()],
                ["URLs",             stats.url_items.toLocaleString()],
                ["Images",           stats.image_items.toLocaleString()],
                ["JSON",             stats.json_items.toLocaleString()],
                ["Emails",           stats.email_items.toLocaleString()],
              ].map(([label, value]) => (
                <div key={label} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "5px 0", borderBottom: "1px solid var(--border-dim)" }}>
                  <span style={{ fontSize: 12, color: "var(--text-muted)" }}>{label}</span>
                  <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{value}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default StatisticsPage;
