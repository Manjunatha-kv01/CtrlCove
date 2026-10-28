import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Download, Database, Shield, Info, Check, AlertCircle, Brain, Zap, RefreshCw } from "lucide-react";
import type { OllamaStatus, AiConfig, EmbeddingStatus } from "../types";

export function Settings() {
  const [exportStatus, setExportStatus] = useState<Record<string, "idle" | "loading" | "done" | "error">>({
    json: "idle", csv: "idle", md: "idle",
  });

  // AI Engine state
  const [ollamaStatus, setOllamaStatus] = useState<OllamaStatus | null>(null);
  const [, setAiConfig]         = useState<AiConfig | null>(null);
  const [aiEnabled, setAiEnabled]       = useState(true);
  const [selectedModel, setSelectedModel] = useState("gemma3:1b");
  const [baseUrl, setBaseUrl]           = useState("http://localhost:11434");
  const [aiSaveStatus, setAiSaveStatus] = useState<"idle" | "saving" | "saved">("idle");
  const [processStatus, setProcessStatus] = useState<"idle" | "running" | "done">("idle");
  const [processedCount, setProcessedCount] = useState<number | null>(null);
  const [checkingOllama, setCheckingOllama] = useState(false);

  // Semantic Engine state
  const [embedStatus, setEmbedStatus] = useState<EmbeddingStatus | null>(null);
  const [embedModel, setEmbedModel] = useState("nomic-embed-text");
  const [embedSaveStatus, setEmbedSaveStatus] = useState<"idle" | "saving" | "saved">("idle");
  const [embedProcessStatus, setEmbedProcessStatus] = useState<"idle" | "running" | "done">("idle");

  const loadAiConfig = async () => {
    try {
      const [status, cfg, eStatus] = await Promise.all([
        invoke<OllamaStatus>("get_ollama_status"),
        invoke<AiConfig>("get_ai_config"),
        invoke<EmbeddingStatus>("get_embedding_status"),
      ]);
      setOllamaStatus(status);
      setAiConfig(cfg);
      setAiEnabled(cfg.enabled);
      setSelectedModel(cfg.model);
      setBaseUrl(cfg.base_url);

      setEmbedStatus(eStatus);
      setEmbedModel(eStatus.model);
    } catch (err) {
      console.error("Config load:", err);
    }
  };

  useEffect(() => { loadAiConfig(); }, []);

  const checkOllama = async () => {
    setCheckingOllama(true);
    try {
      const s = await invoke<OllamaStatus>("get_ollama_status");
      setOllamaStatus(s);
    } catch { /* ignore */ }
    setCheckingOllama(false);
  };

  const saveAiConfig = async () => {
    setAiSaveStatus("saving");
    try {
      await invoke("set_ai_config", { model: selectedModel, enabled: aiEnabled, baseUrl });
      setAiSaveStatus("saved");
      setTimeout(() => setAiSaveStatus("idle"), 2500);
    } catch { setAiSaveStatus("idle"); }
  };

  const processAll = async () => {
    setProcessStatus("running");
    try {
      const count = await invoke<number>("process_all_with_ai");
      setProcessedCount(count);
      setProcessStatus("done");
      setTimeout(() => setProcessStatus("idle"), 5000);
    } catch { setProcessStatus("idle"); }
  };

  // ── Export ──
  const saveEmbedConfig = async () => {
    setEmbedSaveStatus("saving");
    try {
      await invoke("set_embedding_model", { model: embedModel, enabled: true });
      await loadAiConfig(); // refresh status
      setEmbedSaveStatus("saved");
      setTimeout(() => setEmbedSaveStatus("idle"), 2500);
    } catch { setEmbedSaveStatus("idle"); }
  };

  const processEmbedAll = async () => {
    setEmbedProcessStatus("running");
    try {
      const started = await invoke<number>("embed_all_items");
      setEmbedStatus(prev => prev ? { ...prev, pending: started } : null);
      setEmbedProcessStatus("done");
      setTimeout(() => setEmbedProcessStatus("idle"), 3000);
    } catch {
      setEmbedProcessStatus("idle");
    }
  };

  const triggerExport = async (format: "json" | "csv" | "md") => {
    setExportStatus((prev) => ({ ...prev, [format]: "loading" }));
    try {
      let content = "";
      let filename = "";
      let mime = "";
      const ts = new Date().toISOString().split("T")[0];
      if (format === "json") {
        content = await invoke<string>("export_items_json");
        filename = `cymos-export-${ts}.json`;
        mime = "application/json";
      } else if (format === "csv") {
        content = await invoke<string>("export_items_csv");
        filename = `cymos-export-${ts}.csv`;
        mime = "text/csv";
      } else {
        content = await invoke<string>("export_items_markdown");
        filename = `cymos-export-${ts}.md`;
        mime = "text/markdown";
      }
      const blob = new Blob([content], { type: mime });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      a.click();
      URL.revokeObjectURL(url);
      setExportStatus((prev) => ({ ...prev, [format]: "done" }));
      setTimeout(() => setExportStatus((prev) => ({ ...prev, [format]: "idle" })), 3000);
    } catch (err) {
      console.error("Export failed:", err);
      setExportStatus((prev) => ({ ...prev, [format]: "error" }));
      setTimeout(() => setExportStatus((prev) => ({ ...prev, [format]: "idle" })), 3000);
    }
  };

  const exportFormats = [
    { key: "json" as const, label: "JSON",     desc: "Full export with all metadata and AI fields", color: "#4ade80", ext: ".json" },
    { key: "csv"  as const, label: "CSV",      desc: "Spreadsheet-compatible format",               color: "#34d399", ext: ".csv" },
    { key: "md"   as const, label: "Markdown", desc: "Human-readable document format",              color: "#60a5fa", ext: ".md" },
  ];

  const StatusIcon = ({ s }: { s: string }) => {
    if (s === "loading") return <div style={{ width: 14, height: 14, borderRadius: "50%", border: "2px solid rgba(255,255,255,0.2)", borderTopColor: "white", animation: "spin 0.6s linear infinite" }} />;
    if (s === "done") return <Check size={14} style={{ color: "#34d399" }} />;
    if (s === "error") return <AlertCircle size={14} style={{ color: "#f87171" }} />;
    return <Download size={14} />;
  };

  const isRunning = ollamaStatus?.running ?? false;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div className="page-header">
        <div className="page-title">Settings</div>
        <div className="page-subtitle">Configure CYMOS and manage your data</div>
      </div>

      <div style={{ flex: 1, overflowY: "auto" }}>
        <div style={{ padding: 16, maxWidth: 620, margin: "0 auto", display: "flex", flexDirection: "column", gap: 16 }}>

          {/* ── AI Engine Section ── */}
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", overflow: "hidden",
          }}>
            <div style={{ padding: "12px 16px", borderBottom: "1px solid var(--border-dim)", display: "flex", alignItems: "center", gap: 8, background: "rgba(244,114,182,0.04)" }}>
              <Brain size={15} style={{ color: "#f472b6" }} />
              <div>
                <div style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>AI Memory Engine</div>
                <div style={{ fontSize: 11, color: "var(--text-muted)" }}>Local AI via Ollama — fully offline</div>
              </div>
            </div>
            <div style={{ padding: 14, display: "flex", flexDirection: "column", gap: 12 }}>

              {/* Ollama Status */}
              <div style={{
                display: "flex", alignItems: "center", justifyContent: "space-between",
                padding: "10px 12px", borderRadius: 8,
                background: isRunning ? "rgba(52,211,153,0.06)" : "rgba(245,158,11,0.06)",
                border: `1px solid ${isRunning ? "rgba(52,211,153,0.2)" : "rgba(245,158,11,0.2)"}`,
              }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: isRunning ? "#34d399" : "#f59e0b", display: "inline-block" }} />
                  <div>
                    <div style={{ fontSize: 12, fontWeight: 600, color: isRunning ? "#34d399" : "#f59e0b" }}>
                      {isRunning ? "Ollama Connected" : "Ollama Offline"}
                    </div>
                    <div style={{ fontSize: 10, color: "var(--text-muted)" }}>
                      {isRunning
                        ? `${ollamaStatus?.models?.length ?? 0} model(s) available`
                        : "Run: ollama serve"}
                    </div>
                  </div>
                </div>
                <button
                  className="btn btn-ghost"
                  style={{ fontSize: 11, gap: 4 }}
                  onClick={checkOllama}
                  disabled={checkingOllama}
                >
                  <RefreshCw size={11} style={{ animation: checkingOllama ? "spin 1s linear infinite" : "none" }} />
                  {checkingOllama ? "Checking..." : "Refresh"}
                </button>
              </div>

              {/* Enable toggle */}
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <div>
                  <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>Enable AI Enrichment</div>
                  <div style={{ fontSize: 11, color: "var(--text-muted)" }}>Automatically enrich new clipboard items</div>
                </div>
                <button
                  onClick={() => setAiEnabled(v => !v)}
                  style={{
                    width: 36, height: 20, borderRadius: 10, border: "none", cursor: "pointer",
                    background: aiEnabled ? "#3b82f6" : "var(--bg-elevated)",
                    position: "relative", transition: "background 0.2s",
                    boxShadow: "inset 0 0 0 1px rgba(255,255,255,0.1)",
                  }}
                >
                  <span style={{
                    position: "absolute", top: 3, left: aiEnabled ? 18 : 3,
                    width: 14, height: 14, borderRadius: "50%",
                    background: "white", transition: "left 0.2s",
                  }} />
                </button>
              </div>

              {/* Model selector */}
              <div>
                <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)", marginBottom: 6 }}>Model</div>
                {isRunning && ollamaStatus!.models.length > 0 ? (
                  <select
                    value={selectedModel}
                    onChange={(e) => setSelectedModel(e.target.value)}
                    style={{
                      width: "100%", background: "var(--bg-elevated)",
                      border: "1px solid var(--border-base)", borderRadius: 6,
                      color: "var(--text-primary)", fontSize: 12, padding: "6px 8px",
                    }}
                  >
                    {ollamaStatus!.models.map((m) => (
                      <option key={m} value={m}>{m}</option>
                    ))}
                  </select>
                ) : (
                  <input
                    value={selectedModel}
                    onChange={(e) => setSelectedModel(e.target.value)}
                    placeholder="e.g. gemma3:1b"
                    style={{
                      width: "100%", background: "var(--bg-elevated)",
                      border: "1px solid var(--border-base)", borderRadius: 6,
                      color: "var(--text-primary)", fontSize: 12, padding: "6px 8px",
                      outline: "none", boxSizing: "border-box",
                    }}
                  />
                )}
                <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 4 }}>
                  Recommended: gemma3:1b (fast, ~815MB) · qwen2.5:1.5b · llama3.2:1b
                </div>
              </div>

              {/* Base URL */}
              <div>
                <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)", marginBottom: 6 }}>Ollama URL</div>
                <input
                  value={baseUrl}
                  onChange={(e) => setBaseUrl(e.target.value)}
                  style={{
                    width: "100%", background: "var(--bg-elevated)",
                    border: "1px solid var(--border-base)", borderRadius: 6,
                    color: "var(--text-primary)", fontSize: 12, padding: "6px 8px",
                    outline: "none", boxSizing: "border-box",
                  }}
                />
              </div>

              {/* Save button */}
              <div style={{ display: "flex", gap: 8 }}>
                <button
                  className="btn btn-primary"
                  style={{ flex: 1, gap: 5, fontSize: 12 }}
                  onClick={saveAiConfig}
                  disabled={aiSaveStatus === "saving"}
                >
                  {aiSaveStatus === "saving" ? (
                    <div style={{ width: 12, height: 12, borderRadius: "50%", border: "2px solid rgba(255,255,255,0.3)", borderTopColor: "white", animation: "spin 0.6s linear infinite" }} />
                  ) : aiSaveStatus === "saved" ? (
                    <Check size={13} />
                  ) : (
                    <Zap size={13} />
                  )}
                  {aiSaveStatus === "saved" ? "Saved!" : aiSaveStatus === "saving" ? "Saving..." : "Save AI Config"}
                </button>

                {/* Process all */}
                <button
                  className="btn btn-ghost"
                  style={{ flex: 1, gap: 5, fontSize: 12 }}
                  onClick={processAll}
                  disabled={processStatus === "running" || !isRunning}
                  title={!isRunning ? "Start Ollama first" : "Enrich all unprocessed items"}
                >
                  {processStatus === "running" ? (
                    <div style={{ width: 12, height: 12, borderRadius: "50%", border: "2px solid rgba(255,255,255,0.2)", borderTopColor: "white", animation: "spin 0.6s linear infinite" }} />
                  ) : (
                    <Brain size={13} style={{ color: "#f472b6" }} />
                  )}
                  {processStatus === "running" ? "Processing..." :
                   processStatus === "done" ? `Queued ${processedCount ?? 0}` : "Process All"}
                </button>
              </div>

              {/* Process all hint */}
              <div style={{ fontSize: 10, color: "var(--text-muted)", padding: "6px 10px", background: "var(--bg-elevated)", borderRadius: 6 }}>
                💡 "Process All" queues all unprocessed items through Ollama in the background. Items appear enriched in History as they complete.
              </div>
            </div>
          </div>

          {/* ── Semantic Memory Engine Configuration ── */}
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", overflow: "hidden",
            boxShadow: "0 2px 8px rgba(0,0,0,0.05)",
          }}>
            <div style={{ padding: "16px 20px", borderBottom: "1px solid var(--border-dim)", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <div style={{ width: 28, height: 28, borderRadius: 8, background: "#ec489918", border: "1px solid #ec489930", display: "flex", alignItems: "center", justifyContent: "center", color: "#ec4899" }}>
                  <Brain size={14} />
                </div>
                <div>
                  <div style={{ fontSize: 14, fontWeight: 700, color: "var(--text-primary)", display: "flex", alignItems: "center", gap: 6 }}>
                    Semantic Memory Engine
                  </div>
                  <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 2 }}>
                    Local embeddings for natural language search
                  </div>
                </div>
              </div>
            </div>

            <div style={{ padding: "20px" }}>
              {/* Status */}
              <div style={{
                background: "var(--bg-elevated)", border: "1px solid var(--border-dim)", borderRadius: "var(--radius-sm)",
                padding: "12px 16px", marginBottom: 16, display: "flex", justifyContent: "space-between", alignItems: "center"
              }}>
                <div>
                  <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 4 }}>Embedding Model Status</div>
                  <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                    <div style={{ width: 8, height: 8, borderRadius: "50%", background: embedStatus?.model_available ? "#34d399" : "#f87171" }} />
                    <span style={{ fontSize: 13, fontWeight: 500, color: embedStatus?.model_available ? "#34d399" : "#f87171" }}>
                      {embedStatus?.model_available ? "Model Ready" : "Model Missing (Run: ollama run nomic-embed-text)"}
                    </span>
                  </div>
                </div>
                <div style={{ textAlign: "right" }}>
                  <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 4 }}>Embedded Memories</div>
                  <div style={{ fontSize: 14, fontWeight: 700, color: "var(--text-primary)" }}>
                    {embedStatus?.total_embedded || 0} <span style={{ fontSize: 11, fontWeight: 500, color: "var(--text-muted)" }}>({embedStatus?.pending || 0} pending)</span>
                  </div>
                </div>
              </div>

              {/* Settings Form */}
              <div style={{ display: "flex", flexDirection: "column", gap: 16, marginBottom: 20 }}>
                <div>
                  <label style={{ display: "block", fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 6 }}>
                    EMBEDDING MODEL
                  </label>
                  <input
                    type="text"
                    value={embedModel}
                    onChange={(e) => setEmbedModel(e.target.value)}
                    style={{
                      width: "100%", padding: "8px 12px", borderRadius: "var(--radius-sm)",
                      background: "var(--bg-elevated)", border: "1px solid var(--border-base)",
                      color: "var(--text-primary)", fontSize: 13,
                    }}
                    placeholder="e.g. nomic-embed-text"
                  />
                  <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 6 }}>
                    Recommended: <code style={{ fontSize: 10, background: "rgba(0,0,0,0.1)", padding: "2px 4px", borderRadius: 4 }}>nomic-embed-text</code> (fast and local)
                  </div>
                </div>
              </div>

              {/* Actions */}
              <div style={{ display: "flex", gap: 10 }}>
                <button
                  className="btn"
                  onClick={saveEmbedConfig}
                  disabled={embedSaveStatus === "saving"}
                  style={{ background: "#ec4899", color: "white", border: "none" }}
                >
                  {embedSaveStatus === "saving" ? "Saving..." : embedSaveStatus === "saved" ? "Saved!" : "Save Engine Config"}
                </button>
                <button
                  className="btn btn-ghost"
                  onClick={processEmbedAll}
                  disabled={embedProcessStatus === "running" || (embedStatus?.pending === 0 && (embedStatus?.total_embedded || 0) > 0)}
                  style={{ gap: 6, color: "var(--text-primary)" }}
                >
                  <Zap size={14} />
                  {embedProcessStatus === "running" ? "Background Embedding Started..." :
                   embedProcessStatus === "done" ? "Added to Background Queue" :
                   "Embed All Past Memories"}
                </button>
              </div>
            </div>
          </div>

          {/* ── Export Section ── */}
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", overflow: "hidden",
          }}>
            <div style={{ padding: "12px 16px", borderBottom: "1px solid var(--border-dim)", display: "flex", alignItems: "center", gap: 8 }}>
              <Download size={15} style={{ color: "var(--text-muted)" }} />
              <div>
                <div style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>Export Data</div>
                <div style={{ fontSize: 11, color: "var(--text-muted)" }}>Download all clipboard history in your preferred format</div>
              </div>
            </div>
            <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 8 }}>
              {exportFormats.map((fmt) => (
                <div
                  key={fmt.key}
                  style={{
                    display: "flex", alignItems: "center", justifyContent: "space-between",
                    padding: "10px 12px", borderRadius: "var(--radius-sm)",
                    background: "var(--bg-elevated)", border: "1px solid var(--border-dim)",
                  }}
                >
                  <div>
                    <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
                      <span style={{
                        fontSize: 11, fontWeight: 700, fontFamily: "monospace",
                        color: fmt.color, background: `${fmt.color}18`, padding: "1px 6px",
                        borderRadius: 4, border: `1px solid ${fmt.color}30`,
                      }}>
                        {fmt.ext}
                      </span>
                      <span style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>{fmt.label}</span>
                    </div>
                    <div style={{ fontSize: 11, color: "var(--text-muted)" }}>{fmt.desc}</div>
                  </div>
                  <button
                    className="btn btn-ghost"
                    onClick={() => triggerExport(fmt.key)}
                    disabled={exportStatus[fmt.key] === "loading"}
                    style={{ gap: 5, minWidth: 90 }}
                  >
                    <StatusIcon s={exportStatus[fmt.key]} />
                    {exportStatus[fmt.key] === "loading" ? "Exporting..." :
                     exportStatus[fmt.key] === "done" ? "Exported!" :
                     exportStatus[fmt.key] === "error" ? "Failed" : "Export"}
                  </button>
                </div>
              ))}
            </div>
          </div>

          {/* ── Database Info ── */}
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", overflow: "hidden",
          }}>
            <div style={{ padding: "12px 16px", borderBottom: "1px solid var(--border-dim)", display: "flex", alignItems: "center", gap: 8 }}>
              <Database size={15} style={{ color: "var(--text-muted)" }} />
              <div style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>Database</div>
            </div>
            <div style={{ padding: "12px 16px", display: "flex", flexDirection: "column", gap: 8 }}>
              {[
                ["Engine",   "SQLite (offline-first)"],
                ["Location", "~/{project}/database/cymos.db"],
                ["ORM",      "SQLx (async Rust)"],
                ["WAL Mode", "Enabled"],
              ].map(([k, v]) => (
                <div key={k} style={{ display: "flex", justifyContent: "space-between" }}>
                  <span style={{ fontSize: 12, color: "var(--text-muted)" }}>{k}</span>
                  <span style={{ fontSize: 12, color: "var(--text-secondary)", fontFamily: "monospace" }}>{v}</span>
                </div>
              ))}
            </div>
          </div>

          {/* ── Privacy ── */}
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", overflow: "hidden",
          }}>
            <div style={{ padding: "12px 16px", borderBottom: "1px solid var(--border-dim)", display: "flex", alignItems: "center", gap: 8 }}>
              <Shield size={15} style={{ color: "var(--text-muted)" }} />
              <div style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>Privacy</div>
            </div>
            <div style={{ padding: "12px 16px" }}>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {[
                  "All data is stored exclusively on your local machine.",
                  "No telemetry, no analytics, no network requests to external servers.",
                  "AI processing uses local Ollama — your data never leaves your device.",
                  "Clipboard data never leaves your device.",
                  "SHA-256 content hashing is computed locally.",
                ].map((line, i) => (
                  <div key={i} style={{ display: "flex", gap: 8, alignItems: "flex-start" }}>
                    <Check size={12} style={{ color: "#34d399", flexShrink: 0, marginTop: 2 }} />
                    <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>{line}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* ── About ── */}
          <div style={{
            background: "var(--bg-surface)", border: "1px solid var(--border-dim)",
            borderRadius: "var(--radius-md)", overflow: "hidden",
          }}>
            <div style={{ padding: "12px 16px", borderBottom: "1px solid var(--border-dim)", display: "flex", alignItems: "center", gap: 8 }}>
              <Info size={15} style={{ color: "var(--text-muted)" }} />
              <div style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>About CYMOS</div>
            </div>
            <div style={{ padding: "12px 16px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
                <div style={{
                  width: 44, height: 44, borderRadius: 12,
                  background: "linear-gradient(135deg, #3b82f6, #8b5cf6)",
                  display: "flex", alignItems: "center", justifyContent: "center",
                  fontSize: 14, fontWeight: 800, color: "white",
                }}>
                  CY
                </div>
                <div>
                  <div style={{ fontSize: 16, fontWeight: 700, color: "var(--text-primary)", letterSpacing: "-0.01em" }}>CYMOS</div>
                  <div style={{ fontSize: 11, color: "var(--text-muted)" }}>v0.4 · Semantic Memory Engine</div>
                </div>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {[
                  ["Version",     "0.4.0"],
                  ["Built with",  "Tauri 2 + Rust + React + TypeScript"],
                  ["Database",    "SQLite via SQLx"],
                  ["AI Engine",   "Ollama (local LLM)"],
                  ["Clipboard",   "arboard 3.4"],
                ].map(([k, v]) => (
                  <div key={k} style={{ display: "flex", justifyContent: "space-between" }}>
                    <span style={{ fontSize: 12, color: "var(--text-muted)" }}>{k}</span>
                    <span style={{ fontSize: 12, color: "var(--text-secondary)", fontFamily: "monospace" }}>{v}</span>
                  </div>
                ))}
              </div>
              <div style={{ marginTop: 14, padding: "10px 12px", background: "var(--bg-elevated)", borderRadius: 8, border: "1px solid var(--border-dim)" }}>
                <div style={{ fontSize: 11, color: "var(--text-muted)", fontStyle: "italic", lineHeight: 1.6 }}>
                  "Today, CYMOS remembers what you copy. Tomorrow, it will remember what you know."
                </div>
              </div>
            </div>
          </div>

        </div>
      </div>
    </div>
  );
}

export default Settings;
