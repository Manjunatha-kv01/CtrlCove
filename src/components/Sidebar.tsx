import {
  LayoutDashboard,
  History,
  FolderOpen,
  Star,
  Tag,
  BarChart2,
  Settings,
  Clipboard as ClipboardIcon,
  Brain,
  Clock,
  Network,
  MessageSquare,
  Cpu,
  Zap,
} from "lucide-react";
import type { ViewId } from "../types";
import { AiStatusBar } from "./AiStatusBar";

interface SidebarProps {
  activeView: ViewId;
  onNavigate: (view: ViewId) => void;
  itemCount: number;
  favCount: number;
  aiPendingCount: number;
}

const NAV_ITEMS: {
  id: ViewId;
  label: string;
  icon: React.ReactNode;
  section?: string;
}[] = [
  { id: "dashboard",   label: "Dashboard",   icon: <LayoutDashboard size={15} />, section: "OVERVIEW" },
  { id: "history",     label: "History",     icon: <History size={15} /> },
  { id: "favorites",   label: "Favorites",   icon: <Star size={15} /> },
  { id: "collections", label: "Collections", icon: <FolderOpen size={15} />, section: "ORGANIZE" },
  { id: "tags",        label: "Tags",        icon: <Tag size={15} /> },
  { id: "timeline",    label: "Timeline",    icon: <Clock size={15} /> },
  { id: "semantic",    label: "Semantic",    icon: <Brain size={15} />, section: "INSIGHTS" },
  { id: "graph",       label: "Knowledge Graph", icon: <Network size={15} /> },
  { id: "assistant",   label: "AI Assistant",    icon: <MessageSquare size={15} /> },
  { id: "agent",       label: "AI Agents",       icon: <Cpu size={15} /> },
  { id: "autonomous",  label: "Autonomous",      icon: <Zap size={15} /> },
  { id: "statistics",  label: "Statistics",  icon: <BarChart2 size={15} /> },
  { id: "settings",    label: "Settings",    icon: <Settings size={15} />, section: "SYSTEM" },
];

export function Sidebar({ activeView, onNavigate, itemCount, favCount, aiPendingCount }: SidebarProps) {
  return (
    <aside className="app-sidebar">
      {/* Logo */}
      <div className="sidebar-logo">
        <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
          <div className="sidebar-logo-mark">CY</div>
          <div>
            <div style={{ fontSize: "13px", fontWeight: 700, color: "var(--text-primary)", letterSpacing: "-0.01em" }}>
              CYMOS
            </div>
            <div style={{ fontSize: "10px", color: "var(--text-muted)", marginTop: "1px" }}>
              v1.0 · Personal Cognitive OS
            </div>
          </div>
        </div>
      </div>

      {/* Navigation */}
      <nav className="sidebar-nav">
        {NAV_ITEMS.map((item) => (
          <div key={item.id}>
            {item.section && (
              <div className="sidebar-section-label">{item.section}</div>
            )}
            <button
              className={`sidebar-nav-item ${activeView === item.id ? "active" : ""}`}
              onClick={() => onNavigate(item.id)}
              style={{ width: "100%", textAlign: "left" }}
            >
              <span className="nav-icon">{item.icon}</span>
              <span className="nav-label">{item.label}</span>
              {item.id === "history" && itemCount > 0 && (
                <span className="sidebar-badge">{itemCount > 999 ? "999+" : itemCount}</span>
              )}
              {item.id === "favorites" && favCount > 0 && (
                <span className="sidebar-badge" style={{ color: "#fcd34d" }}>{favCount}</span>
              )}
            </button>
          </div>
        ))}
      </nav>

      {/* Footer */}
      <div className="sidebar-footer">
        <div className="listening-pill">
          <span style={{ position: "relative", display: "inline-flex", width: 8, height: 8 }}>
            <span
              className="animate-ping"
              style={{
                position: "absolute", inset: 0,
                borderRadius: "50%",
                background: "rgba(16,185,129,0.5)",
              }}
            />
            <span
              style={{
                position: "relative", display: "block",
                width: 8, height: 8, borderRadius: "50%",
                background: "#10b981",
              }}
            />
          </span>
          Listening
        </div>

        {/* AI Status */}
        <AiStatusBar pendingCount={aiPendingCount} />

        <div style={{ marginTop: 6, fontSize: 10, color: "var(--text-muted)", display: "flex", alignItems: "center", gap: 4 }}>
          <ClipboardIcon size={9} />
          Offline · SQLite · Ollama
        </div>
      </div>
    </aside>
  );
}
