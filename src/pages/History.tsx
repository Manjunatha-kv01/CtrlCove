import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ClipboardItem, Collection, Tag, FilterOptions } from "../types";
import { ClipboardCard } from "../components/ClipboardCard";
import { SearchBar } from "../components/SearchBar";
import { FileText, Trash2 } from "lucide-react";

interface HistoryProps {
  collections: Collection[];
  tags: Tag[];
  onItemCountChange: (count: number) => void;
  onFavCountChange: (count: number) => void;
  onNavigate?: (view: string) => void;
}

export function History({ collections, tags, onItemCountChange, onFavCountChange, onNavigate }: HistoryProps) {
  const [items, setItems] = useState<ClipboardItem[]>([]);
  const [itemTags, setItemTags] = useState<Record<number, Tag[]>>({});
  const [query, setQuery] = useState("");
  const [filters, setFilters] = useState<FilterOptions>({});
  const [loading, setLoading] = useState(true);
  const [showClearConfirm, setShowClearConfirm] = useState(false);

  const fetchItems = useCallback(async () => {
    setLoading(true);
    try {
      let fetched: ClipboardItem[] = [];
      if (query.trim()) {
        fetched = await invoke<ClipboardItem[]>("search_clipboard_items", { query });
      } else {
        fetched = await invoke<ClipboardItem[]>("get_clipboard_items_filtered", {
          opts: { ...filters, limit: 200 },
        });
      }
      setItems(fetched);
      onItemCountChange(fetched.length);
      onFavCountChange(fetched.filter((i) => i.is_favorite === 1).length);

      // Fetch tags for all items
      const tagsMap: Record<number, Tag[]> = {};
      await Promise.all(
        fetched.map(async (item) => {
          try {
            const t = await invoke<Tag[]>("get_tags_for_item", { clipboardId: item.id });
            tagsMap[item.id] = t;
          } catch { tagsMap[item.id] = []; }
        })
      );
      setItemTags(tagsMap);
    } catch (err) {
      console.error("Fetch error:", err);
    } finally {
      setLoading(false);
    }
  }, [query, filters]);

  useEffect(() => {
    const t = setTimeout(fetchItems, 180);
    return () => clearTimeout(t);
  }, [fetchItems]);

  // Real-time listener
  useEffect(() => {
    let unsub: (() => void) | null = null;
    listen<ClipboardItem>("clipboard-updated", (ev) => {
      const newItem = ev.payload;
      setItems((prev) => {
        if (prev.some((i) => i.id === newItem.id)) {
          return prev.map((i) => (i.id === newItem.id ? newItem : i));
        }
        if (query && !newItem.content.toLowerCase().includes(query.toLowerCase())) return prev;
        return [newItem, ...prev];
      });
      setItemTags((prev) => ({ ...prev, [newItem.id]: prev[newItem.id] || [] }));
      onItemCountChange(-1); // signal refresh needed
    }).then((fn) => { unsub = fn; });
    return () => { if (unsub) unsub(); };
  }, [query]);

  const handleDelete = async (id: number) => {
    await invoke("delete_clipboard_item", { id });
    setItems((prev) => prev.filter((i) => i.id !== id));
  };

  const handleFavoriteToggle = (id: number, newState: boolean) => {
    setItems((prev) =>
      prev.map((i) => (i.id === id ? { ...i, is_favorite: newState ? 1 : 0 } : i))
    );
    onFavCountChange(-1);
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

  const handleMoveToCollection = (clipboardId: number, collectionId: number | null) => {
    setItems((prev) =>
      prev.map((i) => (i.id === clipboardId ? { ...i, collection_id: collectionId ?? undefined } : i))
    );
  };

  const handleClearAll = async () => {
    await invoke("clear_clipboard_history");
    setItems([]);
    setItemTags({});
    setShowClearConfirm(false);
    onItemCountChange(0);
    onFavCountChange(0);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Header */}
      <div className="page-header">
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 12 }}>
          <div>
            <div className="page-title">Clipboard History</div>
            <div className="page-subtitle">{items.length} items captured</div>
          </div>
          {items.length > 0 && (
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <button
                className="btn"
                onClick={() => onNavigate?.("semantic")}
                style={{
                  fontSize: 12, gap: 5, padding: "5px 10px",
                  background: "linear-gradient(135deg, rgba(167,139,250,0.15), rgba(236,72,153,0.15))",
                  border: "1px solid rgba(167,139,250,0.3)", color: "#a78bfa"
                }}
              >
                Semantic Search ✨
              </button>
              <button className="btn btn-ghost" onClick={() => setShowClearConfirm(true)} style={{ fontSize: 12, gap: 5, color: "var(--text-muted)" }}>
                <Trash2 size={13} /> Clear All
              </button>
            </div>
          )}
        </div>
        <SearchBar
          query={query}
          onQueryChange={setQuery}
          filters={filters}
          onFiltersChange={setFilters}
          collections={collections}
          tags={tags}
        />
      </div>

      {/* List */}
      <div style={{ flex: 1, overflowY: "auto", padding: "12px 16px", display: "flex", flexDirection: "column", gap: 8 }}>
        {loading && items.length === 0 ? (
          <div className="empty-state">
            <div style={{ width: 24, height: 24, borderRadius: "50%", border: "2px solid var(--border-base)", borderTopColor: "#3b82f6", animation: "ping 0.8s linear infinite" }} />
            <span style={{ fontSize: 12, color: "var(--text-muted)" }}>Loading database...</span>
          </div>
        ) : items.length === 0 ? (
          <div className="empty-state">
            <div className="empty-icon"><FileText size={24} strokeWidth={1.5} /></div>
            <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text-secondary)" }}>No items found</div>
            <div style={{ fontSize: 12, color: "var(--text-muted)", maxWidth: 240 }}>
              {query || Object.keys(filters).some((k) => (filters as any)[k] !== undefined)
                ? "No results match your search or filters."
                : "Copied items will automatically appear here."}
            </div>
          </div>
        ) : (
          items.map((item) => (
            <ClipboardCard
              key={item.id}
              item={item}
              searchQuery={query}
              collections={collections}
              allTags={tags}
              itemTags={itemTags[item.id] || []}
              onDelete={handleDelete}
              onFavoriteToggle={handleFavoriteToggle}
              onTagAdd={handleTagAdd}
              onTagRemove={handleTagRemove}
              onMoveToCollection={handleMoveToCollection}
            />
          ))
        )}
      </div>

      {/* Clear confirmation modal */}
      {showClearConfirm && (
        <div className="modal-backdrop">
          <div className="modal-panel">
            <div style={{ fontSize: 16, fontWeight: 700, marginBottom: 8 }}>Clear all history?</div>
            <div style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 20 }}>
              This permanently deletes all {items.length} clipboard items from your local database. This cannot be undone.
            </div>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button className="btn btn-ghost" onClick={() => setShowClearConfirm(false)}>Cancel</button>
              <button className="btn btn-danger" onClick={handleClearAll}>Delete All</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default History;
