import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Cpu, BookOpen, FlaskConical, Map, GraduationCap, Code2,
  FileText, Search, Sparkles, Play, Loader2, CheckCircle2,
  History, AlertCircle, Clipboard, Clock,
} from "lucide-react";
import type { AgentResult, AgentHistoryTask, AgentStep, ClipboardItem } from "../types";

// ─── Agent Definitions ────────────────────────────────────────────────────────

interface AgentDef {
  key: string;
  label: string;
  icon: React.ReactNode;
  description: string;
  color: string;
  example: string;
}

const AGENTS: AgentDef[] = [
  {
    key: "orchestrator",
    label: "Auto-Orchestrate",
    icon: <Sparkles size={18} />,
    description: "Multi-agent collaboration — automatically decomposes your task and runs the right agents.",
    color: "#a78bfa",
    example: "Prepare me for the BSNL Telecom exam based on my notes",
  },
  {
    key: "memory",
    label: "Memory Agent",
    icon: <BookOpen size={18} />,
    description: "Finds and surfaces relevant knowledge from your clipboard history.",
    color: "#60a5fa",
    example: "Find everything I collected about Docker",
  },
  {
    key: "research",
    label: "Research Agent",
    icon: <FlaskConical size={18} />,
    description: "Analyzes your clipboard history on a topic and produces structured research summaries.",
    color: "#34d399",
    example: "Summarize my Kubernetes research",
  },
  {
    key: "planning",
    label: "Planning Agent",
    icon: <Map size={18} />,
    description: "Creates actionable plans, roadmaps, and task breakdowns grounded in your knowledge.",
    color: "#f59e0b",
    example: "Create a FastAPI project plan based on my notes",
  },
  {
    key: "learning",
    label: "Learning Agent",
    icon: <GraduationCap size={18} />,
    description: "Generates study guides, flashcard prompts, and quizzes from your collected material.",
    color: "#f472b6",
    example: "Create a quiz from my networking notes",
  },
  {
    key: "coding",
    label: "Coding Agent",
    icon: <Code2 size={18} />,
    description: "Assists with code analysis, generation, and project architecture using your code snippets.",
    color: "#fb923c",
    example: "Review my Python code snippets and suggest improvements",
  },
  {
    key: "document",
    label: "Document Agent",
    icon: <FileText size={18} />,
    description: "Generates well-structured documents and reports from your collected notes.",
    color: "#38bdf8",
    example: "Write a technical report on BSNL NMS based on my notes",
  },
  {
    key: "search",
    label: "Search Agent",
    icon: <Search size={18} />,
    description: "Finds and explains the most relevant clipboard items for your query.",
    color: "#94a3b8",
    example: "Find all entries about AI agents I've collected",
  },
];

// ─── Component ────────────────────────────────────────────────────────────────

export function AgentConsole() {
  const [selectedAgent, setSelectedAgent] = useState<AgentDef>(AGENTS[0]);
  const [task, setTask] = useState("");
  const [running, setRunning] = useState(false);
  const [activeResult, setActiveResult] = useState<AgentResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<AgentHistoryTask[]>([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [selectedSource, setSelectedSource] = useState<ClipboardItem | null>(null);
  const [activeStepIdx, setActiveStepIdx] = useState(0);
  const stepTimer = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    loadHistory();
  }, []);

  useEffect(() => {
    return () => {
      if (stepTimer.current) clearInterval(stepTimer.current);
    };
  }, []);

  const loadHistory = async () => {
    setLoadingHistory(true);
    try {
      const tasks = await invoke<AgentHistoryTask[]>("get_agent_history", { limit: 30 });
      setHistory(tasks);
    } catch (e) {
      console.error(e);
    } finally {
      setLoadingHistory(false);
    }
  };

  const animateSteps = (steps: AgentStep[]) => {
    setActiveStepIdx(0);
    let i = 0;
    stepTimer.current = setInterval(() => {
      i++;
      if (i >= steps.length) {
        clearInterval(stepTimer.current!);
      } else {
        setActiveStepIdx(i);
      }
    }, 600);
  };

  const runTask = async () => {
    if (!task.trim() || running) return;
    setRunning(true);
    setError(null);
    setActiveResult(null);

    try {
      let result: AgentResult;
      if (selectedAgent.key === "orchestrator") {
        result = await invoke<AgentResult>("orchestrate_task", { task: task.trim() });
      } else {
        result = await invoke<AgentResult>("run_agent_task", {
          agentType: selectedAgent.key,
          task: task.trim(),
        });
      }
      setActiveResult(result);
      animateSteps(result.steps);
      await loadHistory(); // refresh history panel
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setRunning(false);
    }
  };

  const restoreFromHistory = (item: AgentHistoryTask) => {
    if (!item.result) return;
    const steps: AgentStep[] = item.steps ? JSON.parse(item.steps) : [];
    setActiveResult({
      task_id: item.id,
      agent_type: item.agent_type,
      task: item.task,
      status: item.status as any,
      result: item.result,
      steps,
      sources: [],
      created_at: item.created_at,
    });
    setActiveStepIdx(steps.length - 1);

    const agentDef = AGENTS.find(a => a.key === item.agent_type) ?? AGENTS[0];
    setSelectedAgent(agentDef);
    setTask(item.task);
  };

  const getAgentColor = (key: string) =>
    AGENTS.find(a => a.key === key)?.color ?? "#94a3b8";

  const getStatusIcon = (status: string) => {
    if (status === "done") return <CheckCircle2 size={11} color="#34d399" />;
    if (status === "error") return <AlertCircle size={11} color="#f87171" />;
    return <Loader2 size={11} className="spin" color="#a78bfa" />;
  };

  return (
    <div style={{ display: "flex", height: "100%", background: "var(--bg-base)", overflow: "hidden" }}>

      {/* ── LEFT: Agent Picker ── */}
      <div style={{
        width: 240,
        borderRight: "1px solid var(--border-base)",
        background: "var(--bg-panel)",
        display: "flex",
        flexDirection: "column",
        overflowY: "auto",
      }}>
        <div style={{ padding: "14px 16px 10px", borderBottom: "1px solid var(--border-base)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 7 }}>
            <Cpu size={14} color="#a78bfa" />
            <span style={{ fontSize: 12, fontWeight: 700, color: "var(--text-main)", textTransform: "uppercase", letterSpacing: 0.5 }}>Agents</span>
          </div>
        </div>
        <div style={{ padding: "8px 8px", display: "flex", flexDirection: "column", gap: 3 }}>
          {AGENTS.map(agent => (
            <button
              key={agent.key}
              onClick={() => { setSelectedAgent(agent); setTask(agent.example); setActiveResult(null); setError(null); }}
              style={{
                padding: "10px 12px",
                borderRadius: 8,
                border: selectedAgent.key === agent.key
                  ? `1px solid ${agent.color}44`
                  : "1px solid transparent",
                background: selectedAgent.key === agent.key
                  ? `${agent.color}11`
                  : "transparent",
                display: "flex",
                alignItems: "flex-start",
                gap: 10,
                cursor: "pointer",
                textAlign: "left",
                color: selectedAgent.key === agent.key ? agent.color : "var(--text-muted)",
                transition: "all 0.15s ease",
              }}
            >
              <span style={{ marginTop: 1, flexShrink: 0 }}>{agent.icon}</span>
              <div>
                <div style={{ fontSize: 12, fontWeight: 600, color: selectedAgent.key === agent.key ? "var(--text-main)" : "var(--text-muted)" }}>
                  {agent.label}
                </div>
                <div style={{ fontSize: 10, color: "var(--text-subtle, #6b7280)", marginTop: 2, lineHeight: 1.3 }}>
                  {agent.description.slice(0, 55)}…
                </div>
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* ── CENTER: Task Workspace ── */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        {/* Header */}
        <div style={{
          padding: "14px 24px",
          borderBottom: "1px solid var(--border-base)",
          background: "var(--bg-panel)",
          display: "flex",
          alignItems: "center",
          gap: 10,
        }}>
          <span style={{ color: selectedAgent.color }}>{selectedAgent.icon}</span>
          <div>
            <div style={{ fontSize: 15, fontWeight: 700, color: "var(--text-main)" }}>{selectedAgent.label}</div>
            <div style={{ fontSize: 11, color: "var(--text-muted)" }}>{selectedAgent.description}</div>
          </div>
        </div>

        {/* Content Area */}
        <div style={{ flex: 1, overflowY: "auto", padding: "20px 28px" }}>
          {/* Task Input */}
          <div style={{ marginBottom: 20 }}>
            <label style={{ fontSize: 11, fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: 0.5, display: "block", marginBottom: 8 }}>
              Task Description
            </label>
            <textarea
              value={task}
              onChange={e => setTask(e.target.value)}
              placeholder={selectedAgent.example}
              rows={3}
              style={{
                width: "100%",
                background: "var(--bg-card)",
                border: "1px solid var(--border-base)",
                borderRadius: 8,
                padding: "12px 14px",
                color: "var(--text-main)",
                fontSize: 13,
                resize: "vertical",
                outline: "none",
                lineHeight: 1.5,
                fontFamily: "inherit",
                boxSizing: "border-box",
              }}
              onKeyDown={e => {
                if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) runTask();
              }}
            />
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: 10 }}>
              <span style={{ fontSize: 11, color: "var(--text-muted)" }}>⌘ + Enter to run</span>
              <button
                onClick={runTask}
                disabled={!task.trim() || running}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 7,
                  padding: "8px 18px",
                  borderRadius: 8,
                  border: "none",
                  background: task.trim() && !running ? selectedAgent.color : "rgba(255,255,255,0.05)",
                  color: task.trim() && !running ? "#fff" : "var(--text-muted)",
                  fontSize: 13,
                  fontWeight: 600,
                  cursor: task.trim() && !running ? "pointer" : "default",
                  transition: "all 0.15s",
                }}
              >
                {running
                  ? <><Loader2 size={13} className="spin" /> Running Agents…</>
                  : <><Play size={13} /> Run {selectedAgent.label}</>
                }
              </button>
            </div>
          </div>

          {/* Error */}
          {error && (
            <div style={{ background: "rgba(248,113,113,0.08)", border: "1px solid rgba(248,113,113,0.2)", borderRadius: 8, padding: "12px 16px", marginBottom: 16, display: "flex", gap: 8 }}>
              <AlertCircle size={14} color="#f87171" style={{ flexShrink: 0, marginTop: 1 }} />
              <span style={{ fontSize: 13, color: "#fca5a5" }}>{error}</span>
            </div>
          )}

          {/* Running Steps (live) */}
          {(running || activeResult) && activeResult?.steps && activeResult.steps.length > 0 && (
            <div style={{ marginBottom: 20 }}>
              <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 10 }}>
                Reasoning Trace
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {activeResult.steps.slice(0, activeStepIdx + 1).map((step, i) => (
                  <div
                    key={i}
                    style={{
                      display: "flex",
                      gap: 10,
                      padding: "10px 14px",
                      borderRadius: 8,
                      background: "rgba(255,255,255,0.02)",
                      border: "1px solid var(--border-base)",
                      animation: "fadeIn 0.3s ease",
                    }}
                  >
                    <div style={{
                      width: 20, height: 20, borderRadius: "50%",
                      background: `${selectedAgent.color}22`,
                      border: `1px solid ${selectedAgent.color}44`,
                      display: "flex", alignItems: "center", justifyContent: "center",
                      flexShrink: 0,
                    }}>
                      <CheckCircle2 size={11} color={selectedAgent.color} />
                    </div>
                    <div>
                      <div style={{ fontSize: 11, fontWeight: 700, color: selectedAgent.color, marginBottom: 2 }}>{step.agent}</div>
                      <div style={{ fontSize: 12, color: "var(--text-muted)", lineHeight: 1.4 }}>{step.output}</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Result */}
          {activeResult && activeResult.result && (
            <div>
              <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 10 }}>
                Agent Output
              </div>
              <div style={{
                background: "rgba(255,255,255,0.02)",
                border: "1px solid var(--border-base)",
                borderRadius: 10,
                padding: "20px 22px",
                fontSize: 13,
                color: "var(--text-main)",
                lineHeight: 1.7,
                whiteSpace: "pre-wrap",
              }}>
                {activeResult.result}
              </div>

              {/* Sources */}
              {activeResult.sources.length > 0 && (
                <div style={{ marginTop: 14 }}>
                  <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 8 }}>
                    Memory Sources Used
                  </div>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                    {activeResult.sources.map((src, i) => (
                      <button
                        key={src.id}
                        onClick={() => setSelectedSource(src)}
                        style={{
                          fontSize: 11,
                          background: `${selectedAgent.color}0d`,
                          border: `1px solid ${selectedAgent.color}22`,
                          borderRadius: 6,
                          padding: "4px 9px",
                          color: selectedAgent.color,
                          cursor: "pointer",
                          display: "flex",
                          alignItems: "center",
                          gap: 5,
                        }}
                      >
                        <Clipboard size={9} /> Source #{i + 1} · {src.content_type}
                      </button>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Empty state */}
          {!running && !activeResult && !error && (
            <div style={{ textAlign: "center", padding: "60px 20px", color: "var(--text-muted)" }}>
              <div style={{ display: "inline-flex", padding: 16, borderRadius: "50%", background: `${selectedAgent.color}11`, marginBottom: 14 }}>
                <span style={{ color: selectedAgent.color }}>{selectedAgent.icon}</span>
              </div>
              <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text-main)", marginBottom: 6 }}>
                Ready to run the {selectedAgent.label}
              </div>
              <div style={{ fontSize: 12, maxWidth: 340, margin: "0 auto" }}>
                Edit the task above or use the example, then press <strong>Run {selectedAgent.label}</strong>.
              </div>
            </div>
          )}
        </div>
      </div>

      {/* ── RIGHT: History Panel ── */}
      <div style={{
        width: 280,
        borderLeft: "1px solid var(--border-base)",
        background: "var(--bg-panel)",
        display: "flex",
        flexDirection: "column",
      }}>
        <div style={{ padding: "14px 16px 10px", borderBottom: "1px solid var(--border-base)", display: "flex", alignItems: "center", gap: 7 }}>
          <History size={13} color="var(--text-muted)" />
          <span style={{ fontSize: 12, fontWeight: 700, color: "var(--text-main)", textTransform: "uppercase", letterSpacing: 0.5 }}>Agent History</span>
        </div>

        <div style={{ flex: 1, overflowY: "auto", padding: "8px" }}>
          {loadingHistory ? (
            <div style={{ display: "flex", justifyContent: "center", padding: 20 }}>
              <Loader2 size={16} className="spin" color="#a78bfa" />
            </div>
          ) : history.length === 0 ? (
            <div style={{ textAlign: "center", padding: "32px 16px", fontSize: 12, color: "var(--text-muted)" }}>
              No agent tasks yet. Run your first task!
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              {history.map(item => {
                const agentColor = getAgentColor(item.agent_type);
                return (
                  <button
                    key={item.id}
                    onClick={() => restoreFromHistory(item)}
                    style={{
                      padding: "10px 12px",
                      borderRadius: 7,
                      border: "1px solid var(--border-base)",
                      background: "rgba(255,255,255,0.01)",
                      cursor: "pointer",
                      textAlign: "left",
                      display: "flex",
                      flexDirection: "column",
                      gap: 4,
                      width: "100%",
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <span style={{
                        fontSize: 9,
                        fontWeight: 700,
                        background: `${agentColor}22`,
                        color: agentColor,
                        padding: "2px 6px",
                        borderRadius: 4,
                        textTransform: "uppercase",
                        letterSpacing: 0.5,
                      }}>
                        {item.agent_type}
                      </span>
                      <span style={{ display: "flex", alignItems: "center", gap: 3 }}>
                        {getStatusIcon(item.status)}
                      </span>
                    </div>
                    <div style={{ fontSize: 12, color: "var(--text-main)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {item.task}
                    </div>
                    <div style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 10, color: "var(--text-muted)" }}>
                      <Clock size={9} />
                      {item.created_at}
                    </div>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* Source Viewer Modal */}
      {selectedSource && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)", backdropFilter: "blur(4px)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 200 }}>
          <div className="glass-panel" style={{ width: "90%", maxWidth: 560, maxHeight: "75vh", display: "flex", flexDirection: "column", borderRadius: 12, overflow: "hidden" }}>
            <div style={{ padding: "14px 18px", borderBottom: "1px solid var(--border-base)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span style={{ fontSize: 13, fontWeight: 700, color: "var(--text-main)" }}>Memory Source</span>
              <button onClick={() => setSelectedSource(null)} style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer", fontSize: 16 }}>✕</button>
            </div>
            <div style={{ padding: 18, overflowY: "auto", flex: 1 }}>
              <div style={{ fontSize: 10, color: "var(--text-muted)", marginBottom: 10 }}>
                {selectedSource.content_type} · {selectedSource.created_at}
              </div>
              <pre style={{ background: "rgba(0,0,0,0.2)", padding: 14, borderRadius: 8, fontSize: 12, fontFamily: "monospace", color: "var(--text-main)", whiteSpace: "pre-wrap", border: "1px solid var(--border-base)", maxHeight: 220, overflowY: "auto" }}>
                {selectedSource.content}
              </pre>
              {selectedSource.ai_summary && (
                <div style={{ marginTop: 14 }}>
                  <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-muted)", marginBottom: 4 }}>AI Summary</div>
                  <p style={{ fontSize: 13, color: "var(--text-main)", lineHeight: 1.5 }}>{selectedSource.ai_summary}</p>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
