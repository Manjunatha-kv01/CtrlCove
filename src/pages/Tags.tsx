import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Tag } from "../types";
import { Plus, Trash2, Tag as TagIcon, X, Check } from "lucide-react";

interface TagsProps {
  tags: Tag[];
  onTagsChange: (tags: Tag[]) => void;
}

export function Tags({ tags, onTagsChange }: TagsProps) {
  const [newTagName, setNewTagName] = useState("");
  const [creating, setCreating] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<number | null>(null);

  const handleCreate = async () => {
    const name = newTagName.trim();
    if (!name) return;
    try {
      const tag = await invoke<Tag>("create_tag", { name });
      if (!tags.some((t) => t.id === tag.id)) {
        onTagsChange([...tags, tag].sort((a, b) => a.name.localeCompare(b.name)));
      }
      setNewTagName("");
      setCreating(false);
    } catch (err) {
      console.error("Create tag failed:", err);
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await invoke("delete_tag", { id });
      onTagsChange(tags.filter((t) => t.id !== id));
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Delete tag failed:", err);
    }
  };

  // Predefined quick-add tags
  const quickTags = ["Important", "Work", "Personal", "Research", "AI", "Temp", "Review", "Archive"];
  const existingNames = new Set(tags.map((t) => t.name.toLowerCase()));

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div className="page-header">
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div>
            <div className="page-title">Tags</div>
            <div className="page-subtitle">{tags.length} tag{tags.length !== 1 ? "s" : ""} — use them to organize clipboard items</div>
          </div>
          <button className="btn btn-primary" onClick={() => setCreating(true)}>
            <Plus size={14} /> New Tag
          </button>
        </div>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: 16 }}>
        {/* Create form */}
        {creating && (
          <div style={{
            background: "var(--bg-elevated)", border: "1px solid var(--border-bright)",
            borderRadius: "var(--radius-md)", padding: 16, marginBottom: 16,
            animation: "slide-up 0.15s ease",
          }}>
            <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 10, color: "var(--text-primary)" }}>New Tag</div>
            <div style={{ display: "flex", gap: 8 }}>
              <input
                className="input"
                placeholder="Tag name..."
                value={newTagName}
                onChange={(e) => setNewTagName(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") handleCreate(); if (e.key === "Escape") setCreating(false); }}
                autoFocus
                style={{ flex: 1 }}
              />
              <button className="icon-btn success" onClick={handleCreate}><Check size={14} /></button>
              <button className="icon-btn" onClick={() => { setCreating(false); setNewTagName(""); }}><X size={14} /></button>
            </div>
          </div>
        )}

        {/* Quick-add suggestions */}
        {quickTags.some((q) => !existingNames.has(q.toLowerCase())) && (
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", padding: 14, marginBottom: 16,
          }}>
            <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-muted)", marginBottom: 8, textTransform: "uppercase", letterSpacing: "0.06em" }}>
              Quick Add
            </div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
              {quickTags
                .filter((q) => !existingNames.has(q.toLowerCase()))
                .map((q) => (
                  <button
                    key={q}
                    className="filter-chip"
                    onClick={async () => {
                      const tag = await invoke<Tag>("create_tag", { name: q });
                      if (!tags.some((t) => t.id === tag.id)) {
                        onTagsChange([...tags, tag].sort((a, b) => a.name.localeCompare(b.name)));
                      }
                    }}
                    style={{ fontSize: 12 }}
                  >
                    <Plus size={10} /> {q}
                  </button>
                ))}
            </div>
          </div>
        )}

        {/* Tag grid */}
        {tags.length === 0 ? (
          <div className="empty-state">
            <div className="empty-icon"><TagIcon size={24} strokeWidth={1.5} /></div>
            <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text-secondary)" }}>No tags yet</div>
            <div style={{ fontSize: 12, color: "var(--text-muted)" }}>Create tags to categorize and filter your clipboard items.</div>
          </div>
        ) : (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
            {tags.map((tag) => (
              <div
                key={tag.id}
                style={{
                  display: "inline-flex", alignItems: "center", gap: 8,
                  background: "var(--bg-surface)", border: "1px solid var(--border-base)",
                  borderRadius: 20, padding: "6px 12px",
                  transition: "all 0.15s ease",
                }}
              >
                <TagIcon size={12} style={{ color: "var(--text-muted)" }} />
                <span style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)" }}>{tag.name}</span>
                {deleteConfirm === tag.id ? (
                  <div style={{ display: "flex", gap: 4 }}>
                    <button className="icon-btn danger" onClick={() => handleDelete(tag.id)} style={{ padding: 2 }}>
                      <Check size={11} />
                    </button>
                    <button className="icon-btn" onClick={() => setDeleteConfirm(null)} style={{ padding: 2 }}>
                      <X size={11} />
                    </button>
                  </div>
                ) : (
                  <button
                    className="icon-btn"
                    onClick={() => setDeleteConfirm(tag.id)}
                    style={{ padding: 2, opacity: 0.5 }}
                  >
                    <Trash2 size={11} />
                  </button>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default Tags;
