import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { isToday, isYesterday, isThisWeek, isThisMonth, parseISO } from "date-fns";
import type { ClipboardItem, Collection, Tag } from "../types";
import { ClipboardCard } from "../components/ClipboardCard";
import { Clock } from "lucide-react";

interface TimelineProps {
  collections: Collection[];
  tags: Tag[];
}

interface TimelineGroup {
  label: string;
  items: ClipboardItem[];
}

function parseTimestamp(ts: string): Date {
  try {
    return parseISO(ts.replace(" ", "T") + (ts.includes("Z") ? "" : "Z"));
  } catch {
    return new Date(ts);
  }
}

function groupByTimeline(items: ClipboardItem[]): TimelineGroup[] {
  const groups: Record<string, ClipboardItem[]> = {
    "Today": [],
    "Yesterday": [],
    "This Week": [],
    "This Month": [],
    "Older": [],
  };

  for (const item of items) {
    const d = parseTimestamp(item.created_at);
    if (isToday(d)) groups["Today"].push(item);
    else if (isYesterday(d)) groups["Yesterday"].push(item);
    else if (isThisWeek(d, { weekStartsOn: 1 })) groups["This Week"].push(item);
    else if (isThisMonth(d)) groups["This Month"].push(item);
    else groups["Older"].push(item);
  }

  return Object.entries(groups)
    .filter(([, items]) => items.length > 0)
    .map(([label, items]) => ({ label, items }));
}

export function Timeline({ collections, tags }: TimelineProps) {
  const [items, setItems] = useState<ClipboardItem[]>([]);
  const [itemTags, setItemTags] = useState<Record<number, Tag[]>>({});
  const [loading, setLoading] = useState(true);

  const fetchItems = useCallback(async () => {
    setLoading(true);
    try {
      const fetched = await invoke<ClipboardItem[]>("get_clipboard_items", { limit: 500 });
      setItems(fetched);

      const tagsMap: Record<number, Tag[]> = {};
      await Promise.all(
        fetched.slice(0, 100).map(async (item) => {
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

  useEffect(() => { fetchItems(); }, [fetchItems]);

  const handleDelete = async (id: number) => {
    await invoke("delete_clipboard_item", { id });
    setItems((prev) => prev.filter((i) => i.id !== id));
  };

  const handleFavoriteToggle = (id: number, newState: boolean) => {
    setItems((prev) => prev.map((i) => i.id === id ? { ...i, is_favorite: newState ? 1 : 0 } : i));
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

  const groups = groupByTimeline(items);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div className="page-header">
        <div className="page-title">Timeline</div>
        <div className="page-subtitle">Your clipboard history across time</div>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: "12px 16px" }}>
        {loading ? (
          <div className="empty-state">
            <div style={{ width: 24, height: 24, borderRadius: "50%", border: "2px solid var(--border-base)", borderTopColor: "#3b82f6", animation: "ping 0.8s linear infinite" }} />
          </div>
        ) : groups.length === 0 ? (
          <div className="empty-state">
            <div className="empty-icon"><Clock size={24} strokeWidth={1.5} /></div>
            <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text-secondary)" }}>No history yet</div>
            <div style={{ fontSize: 12, color: "var(--text-muted)" }}>Start copying things to build your timeline.</div>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
            {groups.map((group) => (
              <div key={group.label} className="timeline-group">
                <div className="timeline-label">
                  <Clock size={11} />
                  {group.label}
                  <span style={{ fontSize: 10, fontWeight: 400, color: "var(--text-muted)", marginLeft: 4 }}>
                    {group.items.length} item{group.items.length !== 1 ? "s" : ""}
                  </span>
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  {group.items.map((item) => (
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
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default Timeline;
