import { useState, useRef, useEffect } from "react";
import { Search, X, SlidersHorizontal, Calendar, Layers, Tag as TagIcon, FolderOpen } from "lucide-react";
import type { Collection, Tag, FilterOptions } from "../types";
import { CONTENT_TYPE_CONFIG } from "../types";

interface SearchBarProps {
  query: string;
  onQueryChange: (q: string) => void;
  filters: FilterOptions;
  onFiltersChange: (f: FilterOptions) => void;
  collections: Collection[];
  tags: Tag[];
}

const TYPES = Object.keys(CONTENT_TYPE_CONFIG);

export function SearchBar({
  query,
  onQueryChange,
  filters,
  onFiltersChange,
  collections,
  tags,
}: SearchBarProps) {
  const [showFilters, setShowFilters] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  // Close filter panel on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setShowFilters(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const activeFilterCount = [
    filters.content_type,
    filters.date_from || filters.date_to,
    filters.collection_id,
    filters.tag_id,
    filters.is_favorite,
  ].filter(Boolean).length;

  const clearFilters = () => {
    onFiltersChange({});
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      {/* Search Row */}
      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <div className="search-container" style={{ flex: 1 }}>
          <Search size={15} style={{ position: "absolute", left: 12, color: "var(--text-muted)", pointerEvents: "none" }} />
          <input
            id="search-input"
            type="text"
            className="search-input"
            placeholder="Search content, source app, language..."
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
          />
          {query && (
            <button
              onClick={() => onQueryChange("")}
              style={{ position: "absolute", right: 10, color: "var(--text-muted)", background: "none", border: "none", cursor: "pointer", display: "flex" }}
            >
              <X size={14} />
            </button>
          )}
        </div>

        {/* Filter toggle */}
        <div ref={panelRef} style={{ position: "relative" }}>
          <button
            className={`btn btn-ghost ${activeFilterCount > 0 ? "filter-chip active" : ""}`}
            onClick={() => setShowFilters(!showFilters)}
            style={{ gap: 5, padding: "6px 12px", position: "relative" }}
          >
            <SlidersHorizontal size={14} />
            Filters
            {activeFilterCount > 0 && (
              <span style={{
                background: "var(--accent-blue)", color: "white",
                borderRadius: "50%", width: 16, height: 16,
                fontSize: 10, fontWeight: 700,
                display: "flex", alignItems: "center", justifyContent: "center",
              }}>
                {activeFilterCount}
              </span>
            )}
          </button>

          {/* Filter Panel */}
          {showFilters && (
            <div style={{
              position: "absolute", right: 0, top: "calc(100% + 8px)",
              background: "var(--bg-elevated)", border: "1px solid var(--border-bright)",
              borderRadius: "var(--radius-md)", padding: 16, zIndex: 30,
              width: 280, boxShadow: "0 16px 48px rgba(0,0,0,0.5)",
              animation: "slide-up 0.15s ease",
            }}>
              {/* Type filter */}
              <div style={{ marginBottom: 14 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 11, fontWeight: 600, color: "var(--text-muted)", marginBottom: 8, textTransform: "uppercase", letterSpacing: "0.06em" }}>
                  <Layers size={11} /> Content Type
                </div>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 5 }}>
                  {TYPES.map((type) => (
                    <button
                      key={type}
                      className={`filter-chip ${filters.content_type === type ? "active" : ""}`}
                      onClick={() => onFiltersChange({ ...filters, content_type: filters.content_type === type ? undefined : type })}
                      style={{ fontSize: 11 }}
                    >
                      {type}
                    </button>
                  ))}
                </div>
              </div>

              {/* Date filter */}
              <div style={{ marginBottom: 14 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 11, fontWeight: 600, color: "var(--text-muted)", marginBottom: 8, textTransform: "uppercase", letterSpacing: "0.06em" }}>
                  <Calendar size={11} /> Date Range
                </div>
                <div style={{ display: "flex", gap: 6 }}>
                  <input
                    type="date"
                    className="input"
                    style={{ fontSize: 11, padding: "5px 8px" }}
                    value={filters.date_from || ""}
                    onChange={(e) => onFiltersChange({ ...filters, date_from: e.target.value || undefined })}
                  />
                  <input
                    type="date"
                    className="input"
                    style={{ fontSize: 11, padding: "5px 8px" }}
                    value={filters.date_to || ""}
                    onChange={(e) => onFiltersChange({ ...filters, date_to: e.target.value || undefined })}
                  />
                </div>
              </div>

              {/* Collection filter */}
              {collections.length > 0 && (
                <div style={{ marginBottom: 14 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 11, fontWeight: 600, color: "var(--text-muted)", marginBottom: 8, textTransform: "uppercase", letterSpacing: "0.06em" }}>
                    <FolderOpen size={11} /> Collection
                  </div>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 5 }}>
                    {collections.map((col) => (
                      <button
                        key={col.id}
                        className={`filter-chip ${filters.collection_id === col.id ? "active" : ""}`}
                        onClick={() => onFiltersChange({ ...filters, collection_id: filters.collection_id === col.id ? undefined : col.id })}
                        style={{ fontSize: 11, gap: 4 }}
                      >
                        <span style={{ width: 7, height: 7, borderRadius: "50%", background: col.color || "#3b82f6" }} />
                        {col.name}
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* Tag filter */}
              {tags.length > 0 && (
                <div style={{ marginBottom: 14 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 11, fontWeight: 600, color: "var(--text-muted)", marginBottom: 8, textTransform: "uppercase", letterSpacing: "0.06em" }}>
                    <TagIcon size={11} /> Tag
                  </div>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 5 }}>
                    {tags.map((tag) => (
                      <button
                        key={tag.id}
                        className={`filter-chip ${filters.tag_id === tag.id ? "active" : ""}`}
                        onClick={() => onFiltersChange({ ...filters, tag_id: filters.tag_id === tag.id ? undefined : tag.id })}
                        style={{ fontSize: 11 }}
                      >
                        {tag.name}
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* Favorites filter */}
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                  <input
                    type="checkbox"
                    checked={!!filters.is_favorite}
                    onChange={(e) => onFiltersChange({ ...filters, is_favorite: e.target.checked ? true : undefined })}
                    style={{ accentColor: "#f59e0b" }}
                  />
                  Favorites only
                </label>
                {activeFilterCount > 0 && (
                  <button className="btn btn-ghost" onClick={clearFilters} style={{ fontSize: 11, padding: "3px 8px" }}>
                    Clear all
                  </button>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Active filter chips */}
      {activeFilterCount > 0 && (
        <div className="filter-bar">
          {filters.content_type && (
            <span className="filter-chip active">
              {filters.content_type}
              <X size={10} onClick={() => onFiltersChange({ ...filters, content_type: undefined })} style={{ cursor: "pointer" }} />
            </span>
          )}
          {(filters.date_from || filters.date_to) && (
            <span className="filter-chip active">
              <Calendar size={10} />
              {filters.date_from || "…"} → {filters.date_to || "…"}
              <X size={10} onClick={() => onFiltersChange({ ...filters, date_from: undefined, date_to: undefined })} style={{ cursor: "pointer" }} />
            </span>
          )}
          {filters.collection_id && (
            <span className="filter-chip active">
              <FolderOpen size={10} />
              {collections.find((c) => c.id === filters.collection_id)?.name || "Collection"}
              <X size={10} onClick={() => onFiltersChange({ ...filters, collection_id: undefined })} style={{ cursor: "pointer" }} />
            </span>
          )}
          {filters.tag_id && (
            <span className="filter-chip active">
              <TagIcon size={10} />
              {tags.find((t) => t.id === filters.tag_id)?.name || "Tag"}
              <X size={10} onClick={() => onFiltersChange({ ...filters, tag_id: undefined })} style={{ cursor: "pointer" }} />
            </span>
          )}
          {filters.is_favorite && (
            <span className="filter-chip active" style={{ color: "#fcd34d", borderColor: "rgba(251,191,36,0.3)", background: "rgba(251,191,36,0.08)" }}>
              ★ Favorites
              <X size={10} onClick={() => onFiltersChange({ ...filters, is_favorite: undefined })} style={{ cursor: "pointer" }} />
            </span>
          )}
        </div>
      )}
    </div>
  );
}
