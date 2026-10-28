import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ClipboardItem, Collection, Tag } from "../types";
import { ClipboardCard } from "../components/ClipboardCard";
import { Star } from "lucide-react";

interface FavoritesProps {
  collections: Collection[];
  tags: Tag[];
  onFavCountChange: (count: number) => void;
}

export function Favorites({ collections, tags, onFavCountChange }: FavoritesProps) {
  const [items, setItems] = useState<ClipboardItem[]>([]);
  const [itemTags, setItemTags] = useState<Record<number, Tag[]>>({});
  const [loading, setLoading] = useState(true);

  const fetchFavorites = useCallback(async () => {
    setLoading(true);
    try {
      const fetched = await invoke<ClipboardItem[]>("get_clipboard_items_filtered", {
        opts: { is_favorite: true, limit: 500 },
      });
      setItems(fetched);
      onFavCountChange(fetched.length);

      const tagsMap: Record<number, Tag[]> = {};
      await Promise.all(
        fetched.map(async (item) => {
          try {
            tagsMap[item.id] = await invoke<Tag[]>("get_tags_for_item", { clipboardId: item.id });
          } catch { tagsMap[item.id] = []; }
        })
      );
      setItemTags(tagsMap);
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchFavorites(); }, [fetchFavorites]);

  const handleDelete = async (id: number) => {
    await invoke("delete_clipboard_item", { id });
    setItems((prev) => prev.filter((i) => i.id !== id));
    onFavCountChange(-1);
  };

  const handleFavoriteToggle = async (id: number, newState: boolean) => {
    if (!newState) {
      // Removed from favorites
      setItems((prev) => prev.filter((i) => i.id !== id));
      onFavCountChange(items.length - 1);
    }
  };

  const handleTagAdd = async (clipboardId: number, tagId: number) => {
    await invoke("add_tag_to_item", { clipboardId, tagId });
    const updated = await invoke<Tag[]>("get_tags_for_item", { clipboardId });
    setItemTags((prev) => ({ ...prev, [clipboardId]: updated }));
  };

  const handleTagRemove = async (clipboardId: number, tagId: number) => {
    await invoke("remove_tag_from_item", { clipboardId, tagId });
    setItemTags((prev) => ({
      ...prev,
      [clipboardId]: (prev[clipboardId] || []).filter((t) => t.id !== tagId),
    }));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div className="page-header">
        <div className="page-title">Favorites</div>
        <div className="page-subtitle">{items.length} starred item{items.length !== 1 ? "s" : ""}</div>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: "12px 16px", display: "flex", flexDirection: "column", gap: 8 }}>
        {loading ? (
          <div className="empty-state">
            <div style={{ width: 24, height: 24, borderRadius: "50%", border: "2px solid var(--border-base)", borderTopColor: "#f59e0b", animation: "ping 0.8s linear infinite" }} />
          </div>
        ) : items.length === 0 ? (
          <div className="empty-state">
            <div className="empty-icon" style={{ color: "#f59e0b" }}><Star size={24} strokeWidth={1.5} /></div>
            <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text-secondary)" }}>No favorites yet</div>
            <div style={{ fontSize: 12, color: "var(--text-muted)", maxWidth: 240 }}>
              Click the ★ star on any clipboard item to save it here.
            </div>
          </div>
        ) : (
          items.map((item) => (
            <ClipboardCard
              key={item.id}
              item={item}
              collections={collections}
              allTags={tags}
              itemTags={itemTags[item.id] || []}
              onDelete={handleDelete}
              onFavoriteToggle={handleFavoriteToggle}
              onTagAdd={handleTagAdd}
              onTagRemove={handleTagRemove}
              onMoveToCollection={(clipId, colId) => {
                setItems((prev) => prev.map((i) => i.id === clipId ? { ...i, collection_id: colId ?? undefined } : i));
              }}
            />
          ))
        )}
      </div>
    </div>
  );
}

export default Favorites;
