import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  Bell,
  Cpu,
  Database,
  CheckCircle2,
  Info,
  Clock,
  Play,
  Zap,
  Network,
  Layers,
  Brain,
  Trash2
} from "lucide-react";
import type { Notification, SchedulerRun, KnowledgeHealth } from "../types";

export function AutonomousDashboard() {
  const [health, setHealth] = useState<KnowledgeHealth | null>(null);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [runs, setRuns] = useState<SchedulerRun[]>([]);
  const [isMaintenanceRunning, setIsMaintenanceRunning] = useState(false);

  const fetchData = async () => {
    try {
      const [hData, nData, rData] = await Promise.all([
        invoke<KnowledgeHealth | null>("get_knowledge_health"),
        invoke<Notification[]>("get_notifications", { unreadOnly: false }),
        invoke<SchedulerRun[]>("get_scheduler_runs", { limit: 20 }),
      ]);
      setHealth(hData);
      setNotifications(nData);
      setRuns(rData);
    } catch (e) {
      console.error("Failed to fetch autonomous data:", e);
    }
  };

  useEffect(() => {
    fetchData();
    const unlisten = listen("scheduler-cycle-complete", () => {
      fetchData();
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const handleRunMaintenance = async () => {
    setIsMaintenanceRunning(true);
    try {
      await invoke("run_maintenance_now");
      await fetchData();
    } catch (e) {
      console.error(e);
    } finally {
      setIsMaintenanceRunning(false);
    }
  };

  const handleMarkRead = async (id: number) => {
    await invoke("mark_notification_read", { id });
    fetchData();
  };

  const handleDismiss = async (id: number) => {
    await invoke("dismiss_notification", { id });
    fetchData();
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  };

  const getNotificationIcon = (type: string) => {
    switch (type) {
      case "duplicate": return <Layers size={14} className="text-warning" />;
      case "stale": return <Clock size={14} className="text-muted" />;
      case "insight": return <Zap size={14} className="text-primary" />;
      case "milestone": return <CheckCircle2 size={14} className="text-success" />;
      default: return <Info size={14} className="text-muted" />;
    }
  };

  return (
    <div className="page-container" style={{ padding: 24, maxWidth: 1200, margin: "0 auto", height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 32 }}>
        <div>
          <h1 style={{ fontSize: 24, fontWeight: 600, display: "flex", alignItems: "center", gap: 12, margin: 0 }}>
            <Activity className="text-primary" size={28} />
            Autonomous Intelligence Engine
          </h1>
          <p style={{ color: "var(--text-muted)", marginTop: 8, fontSize: 14 }}>
            CYMOS is continuously analyzing, organizing, and improving your knowledge in the background.
          </p>
        </div>
        <div style={{ display: "flex", gap: 12 }}>
          <button 
            className="btn btn-secondary"
            onClick={handleRunMaintenance}
            disabled={isMaintenanceRunning}
            style={{ display: "flex", alignItems: "center", gap: 8 }}
          >
            <Play size={16} />
            {isMaintenanceRunning ? "Running..." : "Run Maintenance Now"}
          </button>
        </div>
      </div>

      {/* Health Metrics */}
      <h2 style={{ fontSize: 16, fontWeight: 600, marginBottom: 16, color: "var(--text-secondary)" }}>Knowledge Health</h2>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 16, marginBottom: 32 }}>
        {[
          { label: "Total Memories", value: health?.total_memories || 0, icon: <Database size={18} /> },
          { label: "Embedded", value: health?.embedded_count || 0, icon: <Brain size={18} /> },
          { label: "AI Processed", value: health?.ai_processed_count || 0, icon: <Cpu size={18} /> },
          { label: "Graph Entities", value: health?.graph_entities || 0, icon: <Network size={18} /> },
          { label: "Relationships", value: health?.graph_relationships || 0, icon: <Activity size={18} /> },
          { label: "Duplicates", value: health?.duplicate_groups || 0, icon: <Layers size={18} /> },
        ].map((stat, i) => (
          <div key={i} className="card" style={{ padding: 16, display: "flex", flexDirection: "column", gap: 8 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, color: "var(--text-muted)", fontSize: 12 }}>
              {stat.icon} {stat.label}
            </div>
            <div style={{ fontSize: 24, fontWeight: 700 }}>
              {stat.value.toLocaleString()}
            </div>
          </div>
        ))}
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 24 }}>
        {/* Smart Notifications */}
        <div className="card" style={{ padding: 20 }}>
          <h2 style={{ fontSize: 16, fontWeight: 600, marginBottom: 16, display: "flex", alignItems: "center", gap: 8 }}>
            <Bell size={18} className="text-primary" />
            Smart Notifications
          </h2>
          {notifications.length === 0 ? (
            <div style={{ padding: 32, textAlign: "center", color: "var(--text-muted)" }}>
              <CheckCircle2 size={32} style={{ margin: "0 auto 12px", opacity: 0.5 }} />
              <p>No new notifications. Your knowledge base is healthy.</p>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {notifications.map(n => (
                <div key={n.id} style={{ 
                  padding: 16, 
                  background: n.is_read ? "var(--bg-tertiary)" : "rgba(167, 139, 250, 0.1)",
                  borderRadius: 8,
                  border: n.is_read ? "1px solid var(--border-color)" : "1px solid var(--primary)",
                  display: "flex",
                  gap: 12,
                  alignItems: "flex-start"
                }}>
                  <div style={{ marginTop: 2 }}>{getNotificationIcon(n.notification_type)}</div>
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 4, color: n.is_read ? "var(--text-secondary)" : "var(--text-primary)" }}>{n.title}</div>
                    <div style={{ fontSize: 12, color: "var(--text-muted)", lineHeight: 1.5 }}>{n.message}</div>
                    <div style={{ fontSize: 10, color: "var(--text-muted)", opacity: 0.7, marginTop: 8 }}>
                      {new Date(n.created_at + "Z").toLocaleString()}
                    </div>
                  </div>
                  <div style={{ display: "flex", gap: 8 }}>
                    {n.is_read === 0 && (
                      <button className="btn-icon" onClick={() => handleMarkRead(n.id)} title="Mark as read">
                        <CheckCircle2 size={14} />
                      </button>
                    )}
                    <button className="btn-icon text-danger" onClick={() => handleDismiss(n.id)} title="Dismiss">
                      <Trash2 size={14} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Background Activity Log */}
        <div className="card" style={{ padding: 20 }}>
          <h2 style={{ fontSize: 16, fontWeight: 600, marginBottom: 16, display: "flex", alignItems: "center", gap: 8 }}>
            <Clock size={18} className="text-primary" />
            Background Activity Log
          </h2>
          <div style={{ overflowX: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border-color)", textAlign: "left", color: "var(--text-muted)" }}>
                  <th style={{ padding: "8px 12px", fontWeight: 500 }}>Task</th>
                  <th style={{ padding: "8px 12px", fontWeight: 500 }}>Status</th>
                  <th style={{ padding: "8px 12px", fontWeight: 500 }}>Items</th>
                  <th style={{ padding: "8px 12px", fontWeight: 500 }}>Time</th>
                </tr>
              </thead>
              <tbody>
                {runs.length === 0 ? (
                  <tr>
                    <td colSpan={4} style={{ padding: 32, textAlign: "center", color: "var(--text-muted)" }}>
                      No scheduler runs yet.
                    </td>
                  </tr>
                ) : (
                  runs.map(run => (
                    <tr key={run.id} style={{ borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)" }}>
                      <td style={{ padding: "12px", display: "flex", alignItems: "center", gap: 8 }}>
                        {run.task_type === "auto-embed" && <Brain size={14} className="text-primary" />}
                        {run.task_type === "auto-ai" && <Cpu size={14} className="text-secondary" />}
                        {run.task_type === "auto-graph" && <Network size={14} className="text-accent" />}
                        {run.task_type}
                      </td>
                      <td style={{ padding: "12px" }}>
                        <span style={{ 
                          padding: "2px 6px", 
                          borderRadius: 4, 
                          fontSize: 11,
                          background: run.status === "done" ? "rgba(52, 211, 153, 0.1)" : "rgba(239, 68, 68, 0.1)",
                          color: run.status === "done" ? "var(--success)" : "var(--danger)",
                        }}>
                          {run.status}
                        </span>
                      </td>
                      <td style={{ padding: "12px" }}>{run.items_processed}</td>
                      <td style={{ padding: "12px", fontSize: 11, color: "var(--text-muted)" }}>
                        {new Date(run.created_at + "Z").toLocaleTimeString()} <span style={{ opacity: 0.5 }}>({formatDuration(run.duration_ms)})</span>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}
