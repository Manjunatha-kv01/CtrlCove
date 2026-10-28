import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Collection, ClipboardItem } from "../types";
import { COLLECTION_COLORS } from "../types";
import { Plus, Trash2, Edit3, FolderOpen, Check, X } from "lucide-react";

interface CollectionsProps {
  collections: Collection[];
  onCollectionsChange: (collections: Collection[]) => void;
  onNavigateToHistory: (collectionId: number) => void;
}

export function Collections({ collections, onCollectionsChange, onNavigateToHistory }: CollectionsProps) {
  const [itemCounts, setItemCounts] = useState<Record<number, number>>({});
  const [showCreate, setShowCreate] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [newName, setNewName] = useState("");
  const [newColor, setNewColor] = useState(COLLECTION_COLORS[0]);
  const [editName, setEditName] = useState("");
  const [editColor, setEditColor] = useState("");

  useEffect(() => {
    // Fetch item counts per collection
    const loadCounts = async () => {
      const counts: Record<number, number> = {};
      await Promise.all(
        collections.map(async (col) => {
          try {
            const items = await invoke<ClipboardItem[]>("get_clipboard_items_filtered", {
              opts: { collection_id: col.id, limit: 10000 },
            });
            counts[col.id] = items.length;
          } catch { counts[col.id] = 0; }
        })
      );
      setItemCounts(counts);
    };
    if (collections.length > 0) loadCounts();
  }, [collections]);

  const handleCreate = async () => {
    if (!newName.trim()) return;
    try {
      const col = await invoke<Collection>("create_collection", { name: newName.trim(), color: newColor });
      onCollectionsChange([...collections, col]);
      setNewName("");
      setNewColor(COLLECTION_COLORS[0]);
      setShowCreate(false);
    } catch (err) {
      console.error("Create collection failed:", err);
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await invoke("delete_collection", { id });
      onCollectionsChange(collections.filter((c) => c.id !== id));
    } catch (err) {
      console.error("Delete collection failed:", err);
    }
  };

  const handleEdit = async (id: number) => {
    if (!editName.trim()) return;
    try {
      await invoke("update_collection", { id, name: editName.trim(), color: editColor });
      onCollectionsChange(
        collections.map((c) => (c.id === id ? { ...c, name: editName.trim(), color: editColor } : c))
      );
      setEditingId(null);
    } catch (err) {
      console.error("Update collection failed:", err);
    }
  };

  const startEdit = (col: Collection) => {
    setEditingId(col.id);
    setEditName(col.name);
    setEditColor(col.color || COLLECTION_COLORS[0]);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div className="page-header">
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div>
            <div className="page-title">Collections</div>
            <div className="page-subtitle">{collections.length} collection{collections.length !== 1 ? "s" : ""}</div>
          </div>
          <button className="btn btn-primary" onClick={() => setShowCreate(true)}>
            <Plus size={14} /> New Collection
          </button>
        </div>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: 16 }}>
        {/* Create form */}
        {showCreate && (
          <div style={{
            background: "var(--bg-elevated)", border: "1px solid var(--border-bright)",
            borderRadius: "var(--radius-md)", padding: 16, marginBottom: 16,
            animation: "slide-up 0.15s ease",
          }}>
            <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" }}>
              New Collection
            </div>
            <input
              className="input"
              placeholder="Collection name..."
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handleCreate(); if (e.key === "Escape") setShowCreate(false); }}
              autoFocus
              style={{ marginBottom: 10 }}
            />
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 12 }}>
              {COLLECTION_COLORS.map((color) => (
                <button
                  key={color}
                  onClick={() => setNewColor(color)}
                  style={{
                    width: 22, height: 22, borderRadius: "50%",
                    background: color, border: newColor === color ? "2px solid white" : "2px solid transparent",
                    cursor: "pointer", outline: "none",
                    boxShadow: newColor === color ? `0 0 0 2px ${color}50` : "none",
                  }}
                />
              ))}
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button className="btn btn-primary" onClick={handleCreate} style={{ flex: 1 }}>Create</button>
              <button className="btn btn-ghost" onClick={() => setShowCreate(false)}>Cancel</button>
            </div>
          </div>
        )}

        {collections.length === 0 && !showCreate ? (
          <div className="empty-state">
            <div className="empty-icon"><FolderOpen size={24} strokeWidth={1.5} /></div>
            <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text-secondary)" }}>No collections yet</div>
            <div style={{ fontSize: 12, color: "var(--text-muted)" }}>Create collections to organize your clipboard items.</div>
            <button className="btn btn-primary" onClick={() => setShowCreate(true)} style={{ marginTop: 8 }}>
              <Plus size={13} /> Create First Collection
            </button>
          </div>
        ) : (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 12 }}>
            {collections.map((col) => (
              <div key={col.id} className="collection-card">
                {/* Color strip */}
                <div className="collection-color-strip" style={{ background: col.color || "#3b82f6" }} />

                {editingId === col.id ? (
                  <div style={{ paddingTop: 8 }}>
                    <input
                      className="input"
                      value={editName}
                      onChange={(e) => setEditName(e.target.value)}
                      onKeyDown={(e) => { if (e.key === "Enter") handleEdit(col.id); if (e.key === "Escape") setEditingId(null); }}
                      autoFocus
                      style={{ marginBottom: 8, fontSize: 12 }}
                    />
                    <div style={{ display: "flex", gap: 4, flexWrap: "wrap", marginBottom: 8 }}>
                      {COLLECTION_COLORS.map((c) => (
                        <button
                          key={c}
                          onClick={() => setEditColor(c)}
                          style={{
                            width: 18, height: 18, borderRadius: "50%",
                            background: c, border: editColor === c ? "2px solid white" : "2px solid transparent",
                            cursor: "pointer", outline: "none",
                          }}
                        />
                      ))}
                    </div>
                    <div style={{ display: "flex", gap: 6 }}>
                      <button className="icon-btn success" onClick={() => handleEdit(col.id)}><Check size={12} /></button>
                      <button className="icon-btn" onClick={() => setEditingId(null)}><X size={12} /></button>
                    </div>
                  </div>
                ) : (
                  <>
                    <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", marginBottom: 8, paddingTop: 8 }}>
                      <div
                        style={{ width: 36, height: 36, borderRadius: 10, background: `${col.color || "#3b82f6"}20`, border: `1px solid ${col.color || "#3b82f6"}30`, display: "flex", alignItems: "center", justifyContent: "center", color: col.color || "#3b82f6" }}
                      >
                        <FolderOpen size={16} />
                      </div>
                      <div style={{ display: "flex", gap: 2 }}>
                        <button className="icon-btn" onClick={() => startEdit(col)} style={{ opacity: 0.6 }}><Edit3 size={12} /></button>
                        <button className="icon-btn danger" onClick={() => handleDelete(col.id)} style={{ opacity: 0.6 }}><Trash2 size={12} /></button>
                      </div>
                    </div>
                    <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text-primary)", marginBottom: 4 }}>{col.name}</div>
                    <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 12 }}>
                      {itemCounts[col.id] ?? 0} items
                    </div>
                    <button
                      className="btn btn-ghost"
                      onClick={() => onNavigateToHistory(col.id)}
                      style={{ fontSize: 11, padding: "4px 10px", width: "100%", justifyContent: "center" }}
                    >
                      View Items
                    </button>
                  </>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default Collections;
