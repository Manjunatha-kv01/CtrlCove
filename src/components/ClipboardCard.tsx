import { useState } from "react";
import {
  Copy, Check, Trash2, Star, ExternalLink,
  FolderOpen, FileText, Hash, Type, Clock,
  Monitor, Tag as TagIcon, X, Brain, Sparkles, BookOpen,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import type { ClipboardItem, Collection, Tag } from "../types";
import { CONTENT_TYPE_CONFIG, LANGUAGE_COLORS, AI_CATEGORY_CONFIG, parseJsonArray, readingTimeLabel } from "../types";
import { CodeBlock } from "./CodeBlock";
import { ImagePreview } from "./ImagePreview";
import { SimilarMemories } from "./SimilarMemories";

interface ClipboardCardProps {
  item: ClipboardItem;
  searchQuery?: string;
  collections: Collection[];
  allTags: Tag[];
  itemTags: Tag[];
  onDelete: (id: number) => void;
  onFavoriteToggle: (id: number, newState: boolean) => void;
  onTagAdd: (clipboardId: number, tagId: number) => void;
  onTagRemove: (clipboardId: number, tagId: number) => void;
  onMoveToCollection: (clipboardId: number, collectionId: number | null) => void;
}

export function ClipboardCard({
  item,
  searchQuery = "",
  collections,
  allTags,
  itemTags,
  onDelete,
  onFavoriteToggle,
  onTagAdd,
  onTagRemove,
  onMoveToCollection,
}: ClipboardCardProps) {
  const [copied, setCopied] = useState(false);
  const [showCollectionMenu, setShowCollectionMenu] = useState(false);
  const [showTagMenu, setShowTagMenu] = useState(false);

  const typeConfig = CONTENT_TYPE_CONFIG[item.content_type] || CONTENT_TYPE_CONFIG["Text"];
  const isFav = item.is_favorite === 1;
  const aiCategory = item.ai_category;
  const aiCategoryConfig = aiCategory ? (AI_CATEGORY_CONFIG[aiCategory] || AI_CATEGORY_CONFIG["Other"]) : null;
  const aiKeywords = parseJsonArray(item.ai_keywords);
  const aiAutoTags = parseJsonArray(item.ai_tags);
  const [showAiPanel, setShowAiPanel] = useState(false);
  const rtLabel = readingTimeLabel(item.reading_time);

  // ── Handlers ──
  const handleCopy = async () => {
    try {
      await invoke("write_to_clipboard", { content: item.content });
    } catch (err) {
      console.error("write_to_clipboard failed:", err);
      try {
        await navigator.clipboard.writeText(item.content);
      } catch (nativeErr) {
        console.error("navigator.clipboard.writeText failed:", nativeErr);
        return;
      }
    }

    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleFavorite = async () => {
    try {
      const newState = await invoke<boolean>("toggle_favorite", { id: item.id });
      onFavoriteToggle(item.id, newState);
    } catch (err) {
      console.error("Favorite toggle failed:", err);
    }
  };

  const handleMoveToCollection = async (collectionId: number | null) => {
    try {
      await invoke("move_to_collection", { clipboardId: item.id, collectionId });
      onMoveToCollection(item.id, collectionId);
      setShowCollectionMenu(false);
    } catch (err) {
      console.error("Move failed:", err);
    }
  };

  // ── Helpers ──
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
    } catch {
      return ts;
    }
  };

  const highlightText = (text: string, q: string) => {
    if (!q.trim()) return <span>{text}</span>;
    const regex = new RegExp(`(${q.replace(/[-\/\\^$*+?.()|[\]{}]/g, "\\$&")})`, "gi");
    const parts = text.split(regex);
    return (
      <span>
        {parts.map((part, i) =>
          regex.test(part) ? (
            <mark key={i} style={{ background: "rgba(59,130,246,0.3)", color: "#93c5fd", borderRadius: 3, padding: "0 2px" }}>
              {part}
            </mark>
          ) : part
        )}
      </span>
    );
  };

  const formatBytes = (b: number) => {
    if (b < 1024) return `${b}B`;
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)}KB`;
    return `${(b / 1024 / 1024).toFixed(1)}MB`;
  };

  const collection = collections.find((c) => c.id === item.collection_id);

  // ── Render ──
  return (
    <div
      className={`clip-card animate-fade-in ${isFav ? "is-favorite" : ""}`}
      style={{ position: "relative" }}
    >
      {/* Favorite accent line */}
      {isFav && (
        <div style={{
          position: "absolute", top: 0, left: 0, right: 0, height: 2,
          background: "linear-gradient(90deg, #f59e0b, #fbbf24)",
          borderRadius: "var(--radius-md) var(--radius-md) 0 0",
        }} />
      )}

      {/* ── Card Header ── */}
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8, marginBottom: 10 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
          {/* Type Badge */}
          <span
            className="type-badge"
            style={{
              color: typeConfig.color,
              background: typeConfig.bgColor,
              borderColor: typeConfig.borderColor,
            }}
          >
            {item.content_type}
          </span>

          {/* Language Badge */}
          {item.language && (
            <span
              className="lang-badge"
              style={{ color: LANGUAGE_COLORS[item.language] || "#c4b5fd" }}
            >
              {item.language}
            </span>
          )}

          {/* Collection badge */}
          {collection && (
            <span style={{
              fontSize: 10, padding: "2px 7px", borderRadius: 20,
              background: `${collection.color}18`,
              border: `1px solid ${collection.color}40`,
              color: collection.color || "var(--text-secondary)",
              display: "flex", alignItems: "center", gap: 3,
            }}>
              <FolderOpen size={9} />
              {collection.name}
            </span>
          )}

          {/* Timestamp */}
          <span style={{ fontSize: 11, color: "var(--text-muted)", display: "flex", alignItems: "center", gap: 4 }}>
            <Clock size={9} />
            {formatTime(item.created_at)}
            {item.embedding_status === 1 && (
              <span title="Embedded for Semantic Search" style={{
                width: 6, height: 6, borderRadius: "50%",
                background: "#a78bfa", display: "inline-block", marginLeft: 2
              }} />
            )}
          </span>

          {/* Source App */}
          {item.source_app && (
            <span style={{ fontSize: 11, color: "var(--text-muted)", display: "flex", alignItems: "center", gap: 3 }}>
              <Monitor size={9} />
              {item.source_app}
            </span>
          )}
        </div>

        {/* Actions */}
        <div className="clip-card-actions">
          {/* Favorite */}
          <button
            className={`icon-btn warning ${isFav ? "active-fav" : ""}`}
            onClick={handleFavorite}
            data-tooltip={isFav ? "Unfavorite" : "Favorite"}
          >
            <Star size={13} fill={isFav ? "currentColor" : "none"} />
          </button>

          {/* Move to collection */}
          <div style={{ position: "relative" }}>
            <button
              className="icon-btn"
              onClick={() => { setShowCollectionMenu(!showCollectionMenu); setShowTagMenu(false); }}
              data-tooltip="Move to collection"
            >
              <FolderOpen size={13} />
            </button>
            {showCollectionMenu && (
              <div style={{
                position: "absolute", right: 0, top: "calc(100% + 4px)",
                background: "var(--bg-elevated)", border: "1px solid var(--border-bright)",
                borderRadius: "var(--radius-sm)", padding: 4, zIndex: 20,
                minWidth: 160, boxShadow: "0 8px 24px rgba(0,0,0,0.4)",
              }}>
                <button
                  className="sidebar-nav-item"
                  style={{ width: "100%", fontSize: 11 }}
                  onClick={() => handleMoveToCollection(null)}
                >
                  No collection
                </button>
                {collections.map((col) => (
                  <button
                    key={col.id}
                    className="sidebar-nav-item"
                    style={{ width: "100%", fontSize: 11, display: "flex", alignItems: "center", gap: 6 }}
                    onClick={() => handleMoveToCollection(col.id)}
                  >
                    <span style={{ width: 8, height: 8, borderRadius: "50%", background: col.color || "#3b82f6", flexShrink: 0 }} />
                    {col.name}
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Add tag */}
          <div style={{ position: "relative" }}>
            <button
              className="icon-btn"
              onClick={() => { setShowTagMenu(!showTagMenu); setShowCollectionMenu(false); }}
              data-tooltip="Add tag"
            >
              <TagIcon size={13} />
            </button>
            {showTagMenu && (
              <div style={{
                position: "absolute", right: 0, top: "calc(100% + 4px)",
                background: "var(--bg-elevated)", border: "1px solid var(--border-bright)",
                borderRadius: "var(--radius-sm)", padding: 4, zIndex: 20,
                minWidth: 160, maxHeight: 180, overflowY: "auto",
                boxShadow: "0 8px 24px rgba(0,0,0,0.4)",
              }}>
                {allTags.length === 0 ? (
                  <div style={{ fontSize: 11, padding: "6px 8px", color: "var(--text-muted)" }}>
                    No tags yet. Create them in Tags page.
                  </div>
                ) : (
                  allTags.map((tag) => {
                    const hasTag = itemTags.some((t) => t.id === tag.id);
                    return (
                      <button
                        key={tag.id}
                        className="sidebar-nav-item"
                        style={{ width: "100%", fontSize: 11, display: "flex", gap: 6 }}
                        onClick={() => {
                          if (hasTag) onTagRemove(item.id, tag.id);
                          else onTagAdd(item.id, tag.id);
                        }}
                      >
                        {hasTag ? <Check size={10} style={{ color: "#34d399" }} /> : <span style={{ width: 10 }} />}
                        {tag.name}
                      </button>
                    );
                  })
                )}
              </div>
            )}
          </div>

          {/* Copy */}
          <button
            className={`icon-btn ${copied ? "success" : ""}`}
            onClick={handleCopy}
            data-tooltip="Copy to clipboard"
          >
            {copied ? <Check size={13} /> : <Copy size={13} />}
          </button>

          {/* Delete */}
          <button className="icon-btn danger" onClick={() => onDelete(item.id)} data-tooltip="Delete">
            <Trash2 size={13} />
          </button>
        </div>
      </div>

      {/* ── Content Body ── */}
      <div>
        {item.content_type === "Image" ? (
          item.image_data ? (
            <ImagePreview
              imageData={item.image_data}
              width={item.image_width}
            />
          ) : (
            <div style={{ color: "var(--text-muted)", fontSize: 12, display: "flex", alignItems: "center", gap: 6 }}>
              📷 {item.content}
            </div>
          )
        ) : item.content_type === "Code" || item.content_type === "JSON" || item.content_type === "HTML" ? (
          <CodeBlock code={item.content} language={item.language} />
        ) : item.content_type === "URL" ? (
          <a
            href={item.content}
            target="_blank"
            rel="noopener noreferrer"
            style={{
              display: "inline-flex", alignItems: "center", gap: 5,
              color: "#60a5fa", textDecoration: "none", fontSize: 13,
              wordBreak: "break-all",
            }}
          >
            {highlightText(item.content, searchQuery)}
            <ExternalLink size={11} style={{ flexShrink: 0 }} />
          </a>
        ) : item.content_type === "Color" ? (
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div style={{
              width: 28, height: 28, borderRadius: 6,
              background: item.content,
              border: "1px solid var(--border-base)",
              flexShrink: 0,
            }} />
            <span style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: 13, color: "var(--text-primary)" }}>
              {highlightText(item.content, searchQuery)}
            </span>
          </div>
        ) : (
          <div style={{
            fontSize: 13, color: "var(--text-primary)", lineHeight: 1.6,
            whiteSpace: "pre-wrap", wordBreak: "break-word",
            maxHeight: 120, overflow: "hidden",
            maskImage: item.content.length > 300 ? "linear-gradient(to bottom, black 70%, transparent 100%)" : undefined,
          }}>
            {highlightText(item.content, searchQuery)}
          </div>
        )}
      </div>

      {/* ── AI Panel ── */}
      {item.ai_processed === 1 && item.ai_summary && (
        <div style={{ marginTop: 8 }}>
          <button
            onClick={() => setShowAiPanel(v => !v)}
            style={{
              display: "flex", alignItems: "center", gap: 5,
              background: "rgba(167,139,250,0.08)",
              border: "1px solid rgba(167,139,250,0.2)",
              borderRadius: 8, padding: "4px 8px",
              cursor: "pointer", color: "#a78bfa", fontSize: 11, fontWeight: 600,
              transition: "all 0.2s",
            }}
          >
            <Brain size={11} />
            {showAiPanel ? "Hide" : "AI Summary"}
          </button>
          {showAiPanel && (
            <div style={{
              marginTop: 6, padding: "8px 10px",
              background: "rgba(167,139,250,0.06)",
              border: "1px solid rgba(167,139,250,0.15)",
              borderRadius: 8, fontSize: 12, lineHeight: 1.6,
              color: "var(--text-secondary)",
            }}>
              <div style={{ display: "flex", alignItems: "flex-start", gap: 6 }}>
                <Sparkles size={11} style={{ color: "#a78bfa", marginTop: 2, flexShrink: 0 }} />
                <span>{item.ai_summary}</span>
              </div>
              {aiKeywords.length > 0 && (
                <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 6 }}>
                  {aiKeywords.map((kw) => (
                    <span key={kw} style={{
                      fontSize: 10, fontWeight: 600,
                      background: "rgba(96,165,250,0.12)",
                      border: "1px solid rgba(96,165,250,0.2)",
                      borderRadius: 10, padding: "1px 7px",
                      color: "#60a5fa",
                    }}>{kw}</span>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      )}
      {/* Spinner while AI is pending */}
      {item.ai_processed !== 1 && item.content.length >= 50 && item.content_type !== "Image" && (
        <div style={{
          display: "flex", alignItems: "center", gap: 5,
          marginTop: 6, fontSize: 10, color: "var(--text-muted)",
        }}>
          <span style={{ width: 10, height: 10, borderRadius: "50%", border: "1.5px solid rgba(167,139,250,0.4)", borderTopColor: "#a78bfa", display: "inline-block", animation: "spin 1s linear infinite" }} />
          AI enriching...
        </div>
      )}

      {/* ── Metadata Strip ── */}
      <div className="meta-strip" style={{ marginTop: 10 }}>
        {item.char_count !== undefined && item.char_count > 0 && (
          <span className="meta-item">
            <Type size={9} />
            {item.char_count.toLocaleString()} chars
          </span>
        )}
        {item.word_count !== undefined && item.word_count > 0 && (
          <span className="meta-item">
            <FileText size={9} />
            {item.word_count} words
          </span>
        )}
        {rtLabel && (
          <span className="meta-item">
            <BookOpen size={9} />
            {rtLabel}
          </span>
        )}
        {item.copy_count !== undefined && item.copy_count > 1 && (
          <span className="meta-item" style={{ color: "#a78bfa", borderColor: "rgba(167,139,250,0.25)" }}>
            ×{item.copy_count} copied
          </span>
        )}
        {item.file_size !== undefined && item.file_size > 0 && (
          <span className="meta-item">{formatBytes(item.file_size)}</span>
        )}
        {item.content_hash && (
          <span className="meta-item" style={{ fontFamily: "monospace" }}>
            <Hash size={9} />
            {item.content_hash.substring(0, 8)}
          </span>
        )}

        {/* AI Category badge */}
        {aiCategoryConfig && aiCategory && (
          <span style={{
            fontSize: 10, fontWeight: 700, display: "inline-flex", alignItems: "center", gap: 3,
            padding: "2px 7px", borderRadius: 10,
            background: aiCategoryConfig.bgColor,
            border: `1px solid ${aiCategoryConfig.borderColor}`,
            color: aiCategoryConfig.color,
          }}>
            {aiCategoryConfig.emoji} {aiCategory}
          </span>
        )}

        {/* AI auto-tags */}
        {aiAutoTags.map((tag) => (
          <span key={tag} style={{
            fontSize: 10, fontWeight: 600,
            background: "rgba(251,191,36,0.1)",
            border: "1px solid rgba(251,191,36,0.2)",
            borderRadius: 10, padding: "1px 6px",
            color: "#fbbf24",
          }}>#{tag}</span>
        ))}

        {/* Manual tag chips */}
        {itemTags.map((tag) => (
          <button
            key={tag.id}
            className="tag-chip"
            onClick={() => onTagRemove(item.id, tag.id)}
            title={`Remove tag: ${tag.name}`}
          >
            {tag.name}
            <X size={8} />
          </button>
        ))}
      </div>

      {/* Semantic Memory: Similar Items Panel */}
      {item.embedding_status === 1 && (
        <div style={{ marginTop: 10 }}>
          <SimilarMemories itemId={item.id} />
        </div>
      )}
    </div>
  );
}
