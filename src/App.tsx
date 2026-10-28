import "./App.css";
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Collection, Tag, ClipboardItem, ViewId, FilterOptions } from "./types";
import { Sidebar } from "./components/Sidebar";
import { Dashboard } from "./pages/Dashboard";
import { History } from "./pages/History";
import { Collections } from "./pages/Collections";
import { Favorites } from "./pages/Favorites";
import { Tags } from "./pages/Tags";
import { Timeline } from "./pages/Timeline";
import { StatisticsPage } from "./pages/Statistics";
import { SemanticSearch } from "./pages/SemanticSearch";
import { KnowledgeGraph } from "./pages/KnowledgeGraph";
import { MemoryAssistant } from "./pages/MemoryAssistant";
import { AgentConsole } from "./pages/AgentConsole";
import { AutonomousDashboard } from "./pages/AutonomousDashboard";
import { Settings } from "./pages/Settings";

function App() {
  const [activeView, setActiveView] = useState<ViewId>("dashboard");
  const [collections, setCollections] = useState<Collection[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [itemCount, setItemCount] = useState(0);
  const [favCount, setFavCount] = useState(0);
  const [aiPendingCount, setAiPendingCount] = useState(0);

  // Override filter for navigating from Collections page
  const [historyFilter, setHistoryFilter] = useState<FilterOptions>({});


  // Load global data
  const loadGlobal = useCallback(async () => {
    try {
      const [cols, tgs, items] = await Promise.all([
        invoke<Collection[]>("get_collections"),
        invoke<Tag[]>("get_tags"),
        invoke<ClipboardItem[]>("get_clipboard_items", { limit: 1000 }),
      ]);
      setCollections(cols);
      setTags(tgs);
      setItemCount(items.length);
      setFavCount(items.filter((i) => i.is_favorite === 1).length);
      // Track pending AI count
      const pending = items.filter(
        (i) => i.ai_processed !== 1 && (i.char_count ?? 0) >= 50 && i.content_type !== "Image"
      ).length;
      setAiPendingCount(pending);
    } catch (err) {
      console.error("Global load error:", err);
    }
  }, []);

  useEffect(() => {
    loadGlobal();
  }, [loadGlobal]);

  // Real-time: new clipboard item
  useEffect(() => {
    let unsub: (() => void) | null = null;
    listen<ClipboardItem>("clipboard-updated", (event) => {
      setItemCount((prev) => prev + 1);
      const item = event.payload;
      if (item && item.content.length >= 50 && item.content_type !== "Image") {
        setAiPendingCount((prev) => prev + 1);
      }
    }).then((fn) => { unsub = fn; });
    return () => { if (unsub) unsub(); };
  }, []);

  // Real-time: AI enriched
  useEffect(() => {
    let unsub: (() => void) | null = null;
    listen<ClipboardItem>("ai-enriched", () => {
      setAiPendingCount((prev) => Math.max(0, prev - 1));
    }).then((fn) => { unsub = fn; });
    return () => { if (unsub) unsub(); };
  }, []);

  // Real-time: Embedding ready
  useEffect(() => {
    let unsub: (() => void) | null = null;
    listen("embedding-ready", () => {
      // You could update a global semantic pending count here if needed
    }).then((fn) => { unsub = fn; });
    return () => { if (unsub) unsub(); };
  }, []);

  const navigateToHistory = (collectionId?: number) => {
    if (collectionId !== undefined) {
      setHistoryFilter({ collection_id: collectionId });
    } else {
      setHistoryFilter({});
    }
    setActiveView("history");
  };

  const handleItemCountChange = (count: number) => {
    if (count >= 0) setItemCount(count);
    else loadGlobal();
  };

  const handleFavCountChange = (count: number) => {
    if (count >= 0) setFavCount(count);
    else loadGlobal();
  };

  const renderPage = () => {
    switch (activeView) {
      case "dashboard":
        return <Dashboard onNavigate={(v) => setActiveView(v as ViewId)} />;
      case "history":
        return (
          <History
            key={JSON.stringify(historyFilter)}
            collections={collections}
            tags={tags}
            onItemCountChange={handleItemCountChange}
            onFavCountChange={handleFavCountChange}
            onNavigate={(v) => setActiveView(v as ViewId)}
          />
        );
      case "collections":
        return (
          <Collections
            collections={collections}
            onCollectionsChange={setCollections}
            onNavigateToHistory={navigateToHistory}
          />
        );
      case "favorites":
        return (
          <Favorites
            collections={collections}
            tags={tags}
            onFavCountChange={handleFavCountChange}
          />
        );
      case "tags":
        return <Tags tags={tags} onTagsChange={setTags} />;
      case "timeline":
        return <Timeline collections={collections} tags={tags} />;
      case "statistics":
        return <StatisticsPage />;
      case "semantic":
        return <SemanticSearch onNavigate={(v) => setActiveView(v as ViewId)} />;
      case "graph":
        return <KnowledgeGraph />;
      case "assistant":
        return <MemoryAssistant />;
      case "agent":
        return <AgentConsole />;
      case "autonomous":
        return <AutonomousDashboard />;
      case "settings":
        return <Settings />;
      default:
        return <Dashboard onNavigate={(v) => setActiveView(v as ViewId)} />;
    }
  };

  return (
    <div className="app-shell">
      <Sidebar
        activeView={activeView}
        onNavigate={setActiveView}
        itemCount={itemCount}
        favCount={favCount}
        aiPendingCount={aiPendingCount}
      />
      <main className="app-content">
        {renderPage()}
      </main>
    </div>
  );
}

export default App;
