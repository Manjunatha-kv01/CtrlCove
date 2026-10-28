import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { 
  MessageSquare, Sparkles, Send, Loader2, Bot, 
  User, BookOpen, Calendar, FileText, 
  ChevronRight, Clock, Clipboard
} from "lucide-react";
import type { ChatMessage, MemorySummary, ClipboardItem } from "../types";

export function MemoryAssistant() {
  const [activeTab, setActiveTab] = useState<"chat" | "reports">("chat");
  
  // Chat States
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [selectedSource, setSelectedSource] = useState<ClipboardItem | null>(null);
  
  // Reports States
  const [dailySummary, setDailySummary] = useState<MemorySummary | null>(null);
  const [weeklyReport, setWeeklyReport] = useState<MemorySummary | null>(null);
  const [loadingReports, setLoadingReports] = useState(false);

  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (activeTab === "reports") {
      loadReports();
    }
  }, [activeTab]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const loadReports = async () => {
    setLoadingReports(true);
    try {
      const [daily, weekly] = await Promise.all([
        invoke<MemorySummary>("get_daily_summary"),
        invoke<MemorySummary>("get_weekly_report"),
      ]);
      setDailySummary(daily);
      setWeeklyReport(weekly);
    } catch (err) {
      console.error("Failed to load reports", err);
    } finally {
      setLoadingReports(false);
    }
  };

  const handleSend = async (textToSend: string) => {
    if (!textToSend.trim() || loading) return;

    const userMsg: ChatMessage = {
      id: crypto.randomUUID(),
      role: "user",
      content: textToSend,
    };

    setMessages((prev) => [...prev, userMsg]);
    setInput("");
    setLoading(true);

    try {
      // Map ChatMessage list to simpler history format: role and content
      const historyPayload = messages.map(m => ({
        role: m.role,
        content: m.content
      }));

      const result = await invoke<{ response: string; sources: ClipboardItem[] }>(
        "chat_with_memory",
        { query: textToSend, history: historyPayload }
      );

      const assistantMsg: ChatMessage = {
        id: crypto.randomUUID(),
        role: "assistant",
        content: result.response,
        sources: result.sources,
      };

      setMessages((prev) => [...prev, assistantMsg]);
    } catch (err: any) {
      console.error(err);
      const errorMsg: ChatMessage = {
        id: crypto.randomUUID(),
        role: "assistant",
        content: `Error: ${err.toString() || "Could not connect to Ollama. Make sure Ollama is running and gemma3:1b is installed."}`,
      };
      setMessages((prev) => [...prev, errorMsg]);
    } finally {
      setLoading(false);
    }
  };

  const suggestions = [
    "What Docker commands did I copy recently?",
    "Show me notes related to BSNL",
    "List all my code snippets or helper functions",
    "Find my AI Summaries from yesterday",
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--bg-base)" }}>
      {/* Tabs / Header */}
      <div style={{
        padding: "16px 24px",
        borderBottom: "1px solid var(--border-base)",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        background: "var(--bg-panel)"
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <Sparkles size={18} color="#a78bfa" />
          <span style={{ fontSize: 16, fontWeight: 700, color: "var(--text-main)" }}>Memory Assistant Engine</span>
        </div>
        
        <div style={{ display: "flex", background: "rgba(255,255,255,0.03)", padding: 3, borderRadius: 8, border: "1px solid var(--border-base)" }}>
          <button
            onClick={() => setActiveTab("chat")}
            style={{
              padding: "6px 12px",
              borderRadius: 6,
              border: "none",
              background: activeTab === "chat" ? "var(--bg-card)" : "transparent",
              color: activeTab === "chat" ? "var(--text-main)" : "var(--text-muted)",
              fontSize: 12,
              fontWeight: 600,
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              gap: 6
            }}
          >
            <MessageSquare size={13} /> Chat Workspace
          </button>
          <button
            onClick={() => setActiveTab("reports")}
            style={{
              padding: "6px 12px",
              borderRadius: 6,
              border: "none",
              background: activeTab === "reports" ? "var(--bg-card)" : "transparent",
              color: activeTab === "reports" ? "var(--text-main)" : "var(--text-muted)",
              fontSize: 12,
              fontWeight: 600,
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              gap: 6
            }}
          >
            <FileText size={13} /> Daily & Weekly Insights
          </button>
        </div>
      </div>

      {activeTab === "chat" ? (
        /* CHAT WORKSPACE */
        <div style={{ display: "flex", flex: 1, overflow: "hidden", position: "relative" }}>
          <div style={{ flex: 1, display: "flex", flexDirection: "column", height: "100%" }}>
            
            {/* Messages container */}
            <div style={{ flex: 1, overflowY: "auto", padding: "24px 32px" }}>
              {messages.length === 0 ? (
                <div style={{ maxWidth: 600, margin: "60px auto 0", textAlign: "center" }}>
                  <div style={{ display: "inline-flex", padding: 14, borderRadius: "50%", background: "rgba(167,139,250,0.08)", marginBottom: 16 }}>
                    <Bot size={36} color="#a78bfa" />
                  </div>
                  <h2 style={{ fontSize: 18, fontWeight: 700, color: "var(--text-main)", marginBottom: 8 }}>
                    Chat with your Memory Operating System
                  </h2>
                  <p style={{ fontSize: 13, color: "var(--text-muted)", lineHeight: 1.5, marginBottom: 28 }}>
                    Ground your questions in everything you have ever copied. Ask me to find code, compare notes, or retrieve commands.
                  </p>
                  
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, textAlign: "left" }}>
                    {suggestions.map((s, idx) => (
                      <button
                        key={idx}
                        onClick={() => handleSend(s)}
                        className="glass-card clickable"
                        style={{
                          padding: 14,
                          borderRadius: 8,
                          background: "rgba(255,255,255,0.02)",
                          border: "1px solid var(--border-base)",
                          fontSize: 12,
                          color: "var(--text-main)",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "space-between",
                          textAlign: "left"
                        }}
                      >
                        <span>{s}</span>
                        <ChevronRight size={13} style={{ flexShrink: 0, marginLeft: 8 }} />
                      </button>
                    ))}
                  </div>
                </div>
              ) : (
                <div style={{ maxWidth: 700, margin: "0 auto", display: "flex", flexDirection: "column", gap: 20 }}>
                  {messages.map((m) => (
                    <div
                      key={m.id}
                      style={{
                        display: "flex",
                        gap: 12,
                        alignSelf: m.role === "user" ? "flex-end" : "flex-start",
                        maxWidth: "85%",
                        flexDirection: m.role === "user" ? "row-reverse" : "row"
                      }}
                    >
                      {/* Avatar */}
                      <div style={{
                        width: 28, height: 28, borderRadius: "50%",
                        display: "flex", alignItems: "center", justifyContent: "center",
                        background: m.role === "user" ? "rgba(167,139,250,0.15)" : "rgba(34,197,94,0.15)",
                        border: m.role === "user" ? "1px solid rgba(167,139,250,0.3)" : "1px solid rgba(34,197,94,0.3)",
                        color: m.role === "user" ? "#a78bfa" : "#22c55e",
                        flexShrink: 0
                      }}>
                        {m.role === "user" ? <User size={13} /> : <Bot size={13} />}
                      </div>

                      {/* Bubble */}
                      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                        <div style={{
                          padding: "12px 16px",
                          borderRadius: 12,
                          background: m.role === "user" ? "var(--bg-card)" : "rgba(255,255,255,0.02)",
                          border: "1px solid var(--border-base)",
                          fontSize: 13,
                          color: "var(--text-main)",
                          lineHeight: 1.5,
                          whiteSpace: "pre-wrap"
                        }}>
                          {m.content}
                        </div>

                        {/* Citations / Sources */}
                        {m.sources && m.sources.length > 0 && (
                          <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 4 }}>
                            {m.sources.map((src, i) => (
                              <button
                                key={src.id}
                                onClick={() => setSelectedSource(src)}
                                style={{
                                  fontSize: 10,
                                  background: "rgba(167,139,250,0.06)",
                                  border: "1px solid rgba(167,139,250,0.15)",
                                  borderRadius: 6,
                                  padding: "2px 6px",
                                  color: "#a78bfa",
                                  cursor: "pointer",
                                  display: "flex",
                                  alignItems: "center",
                                  gap: 4
                                }}
                              >
                                <BookOpen size={9} /> Source #{i + 1} ({src.content_type})
                              </button>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>
                  ))}
                  
                  {loading && (
                    <div style={{ display: "flex", gap: 12, alignSelf: "flex-start" }}>
                      <div style={{
                        width: 28, height: 28, borderRadius: "50%",
                        display: "flex", alignItems: "center", justifyContent: "center",
                        background: "rgba(34,197,94,0.15)", border: "1px solid rgba(34,197,94,0.3)",
                        color: "#22c55e", flexShrink: 0
                      }}>
                        <Bot size={13} />
                      </div>
                      <div style={{
                        padding: "12px 16px", borderRadius: 12,
                        background: "rgba(255,255,255,0.02)", border: "1px solid var(--border-base)",
                        display: "flex", alignItems: "center", gap: 6
                      }}>
                        <Loader2 size={13} className="spin" color="#a78bfa" />
                        <span style={{ fontSize: 12, color: "var(--text-muted)" }}>Searching memory & generating answer...</span>
                      </div>
                    </div>
                  )}

                  <div ref={messagesEndRef} />
                </div>
              )}
            </div>

            {/* Input Bar */}
            <div style={{ padding: "16px 32px 24px", borderTop: "1px solid var(--border-base)", background: "var(--bg-panel)" }}>
              <div style={{ maxWidth: 700, margin: "0 auto", position: "relative" }}>
                <input
                  type="text"
                  placeholder="Ask CYMOS anything (e.g., 'What commands did I run yesterday?')"
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleSend(input);
                  }}
                  style={{
                    width: "100%",
                    background: "var(--bg-card)",
                    border: "1px solid var(--border-base)",
                    borderRadius: 10,
                    padding: "12px 48px 12px 16px",
                    color: "var(--text-main)",
                    fontSize: 13,
                    outline: "none"
                  }}
                />
                <button
                  onClick={() => handleSend(input)}
                  disabled={!input.trim() || loading}
                  style={{
                    position: "absolute",
                    right: 8,
                    top: "50%",
                    transform: "translateY(-50%)",
                    background: input.trim() ? "#a78bfa" : "transparent",
                    color: input.trim() ? "#fff" : "var(--text-muted)",
                    border: "none",
                    borderRadius: 6,
                    padding: 6,
                    cursor: input.trim() ? "pointer" : "default",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center"
                  }}
                >
                  <Send size={14} />
                </button>
              </div>
            </div>

          </div>
        </div>
      ) : (
        /* REPORTS / INSIGHTS PANEL */
        <div style={{ flex: 1, overflowY: "auto", padding: "24px 32px" }}>
          <div style={{ maxWidth: 800, margin: "0 auto" }}>
            
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 20 }}>
              <div>
                <h2 style={{ fontSize: 18, fontWeight: 700, color: "var(--text-main)", marginBottom: 4 }}>Daily & Weekly Insights</h2>
                <p style={{ fontSize: 13, color: "var(--text-muted)" }}>Automatically generated summaries of your knowledge accumulation</p>
              </div>
              <button 
                onClick={loadReports}
                disabled={loadingReports}
                className="btn" 
                style={{ fontSize: 12, padding: "6px 12px" }}
              >
                {loadingReports ? "Analyzing..." : "Refresh Reports"}
              </button>
            </div>

            {loadingReports ? (
              <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: 200, flexDirection: "column", gap: 12 }}>
                <Loader2 size={24} className="spin" color="#a78bfa" />
                <span style={{ fontSize: 12, color: "var(--text-muted)" }}>Processing memory history...</span>
              </div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
                {/* Daily Recap */}
                <div className="glass-card" style={{ padding: 20, borderRadius: 10 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
                    <Clock size={16} color="#34d399" />
                    <span style={{ fontSize: 14, fontWeight: 700, color: "var(--text-main)" }}>Daily Memory Summary</span>
                  </div>
                  {dailySummary ? (
                    <div>
                      <p style={{ fontSize: 13, color: "var(--text-main)", lineHeight: 1.6, marginBottom: 16 }}>
                        {dailySummary.summary_text}
                      </p>
                      <div style={{ display: "flex", gap: 20 }}>
                        <div>
                          <div style={{ fontSize: 10, color: "var(--text-muted)", textTransform: "uppercase" }}>Copied Today</div>
                          <div style={{ fontSize: 16, fontWeight: 700, color: "var(--text-main)" }}>{dailySummary.total_copied} items</div>
                        </div>
                        <div>
                          <div style={{ fontSize: 10, color: "var(--text-muted)", textTransform: "uppercase" }}>Primary Focus</div>
                          <div style={{ fontSize: 16, fontWeight: 700, color: "#34d399" }}>{dailySummary.top_category}</div>
                        </div>
                      </div>
                    </div>
                  ) : (
                    <span style={{ fontSize: 13, color: "var(--text-muted)" }}>No daily data generated yet.</span>
                  )}
                </div>

                {/* Weekly Learning Report */}
                <div className="glass-card" style={{ padding: 20, borderRadius: 10 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
                    <Calendar size={16} color="#60a5fa" />
                    <span style={{ fontSize: 14, fontWeight: 700, color: "var(--text-main)" }}>Weekly Learning & Progress Report</span>
                  </div>
                  {weeklyReport ? (
                    <div>
                      <p style={{ fontSize: 13, color: "var(--text-main)", lineHeight: 1.6, marginBottom: 16 }}>
                        {weeklyReport.summary_text}
                      </p>
                      <div style={{ display: "flex", gap: 20 }}>
                        <div>
                          <div style={{ fontSize: 10, color: "var(--text-muted)", textTransform: "uppercase" }}>Records Scanned</div>
                          <div style={{ fontSize: 16, fontWeight: 700, color: "var(--text-main)" }}>{weeklyReport.total_copied} items</div>
                        </div>
                        <div>
                          <div style={{ fontSize: 10, color: "var(--text-muted)", textTransform: "uppercase" }}>Dominant Domain</div>
                          <div style={{ fontSize: 16, fontWeight: 700, color: "#60a5fa" }}>{weeklyReport.top_category}</div>
                        </div>
                      </div>
                    </div>
                  ) : (
                    <span style={{ fontSize: 13, color: "var(--text-muted)" }}>No weekly data generated yet.</span>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Source Viewer Modal */}
      {selectedSource && (
        <div style={{
          position: "fixed",
          inset: 0,
          background: "rgba(0,0,0,0.5)",
          backdropFilter: "blur(4px)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          zIndex: 100
        }}>
          <div className="glass-panel" style={{ width: "90%", maxWidth: 600, maxHeight: "80vh", overflow: "hidden", display: "flex", flexDirection: "column", borderRadius: 12 }}>
            <div style={{ padding: "16px 20px", borderBottom: "1px solid var(--border-base)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <Clipboard size={14} color="#a78bfa" />
                <span style={{ fontSize: 14, fontWeight: 700, color: "var(--text-main)" }}>Source Clipboard Record</span>
              </div>
              <button 
                onClick={() => setSelectedSource(null)}
                style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer", fontSize: 16 }}
              >
                ✕
              </button>
            </div>
            
            <div style={{ padding: 20, overflowY: "auto", flex: 1 }}>
              <div style={{ fontSize: 11, background: "rgba(255,255,255,0.03)", padding: "4px 8px", borderRadius: 4, display: "inline-block", color: "var(--text-muted)", marginBottom: 12 }}>
                {selectedSource.content_type} · Copied on {selectedSource.created_at}
              </div>
              <pre style={{
                background: "rgba(0,0,0,0.2)",
                padding: 16,
                borderRadius: 8,
                fontSize: 12,
                fontFamily: "monospace",
                color: "var(--text-main)",
                whiteSpace: "pre-wrap",
                border: "1px solid var(--border-base)",
                maxHeight: 250,
                overflowY: "auto"
              }}>
                {selectedSource.content}
              </pre>
              {selectedSource.ai_summary && (
                <div style={{ marginTop: 16 }}>
                  <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-muted)", marginBottom: 4 }}>AI Summary</h4>
                  <p style={{ fontSize: 13, color: "var(--text-main)", lineHeight: 1.4 }}>{selectedSource.ai_summary}</p>
                </div>
              )}
            </div>

            <div style={{ padding: "12px 20px", borderTop: "1px solid var(--border-base)", display: "flex", justifyContent: "flex-end" }}>
              <button onClick={() => setSelectedSource(null)} className="btn btn-primary" style={{ fontSize: 12, padding: "6px 16px" }}>Close Source</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
