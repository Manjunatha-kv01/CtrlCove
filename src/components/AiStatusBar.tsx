import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { OllamaStatus, AiConfig } from "../types";

interface AiStatusBarProps {
  pendingCount: number;
}

export function AiStatusBar({ pendingCount }: AiStatusBarProps) {
  const [status, setStatus] = useState<OllamaStatus | null>(null);
  const [config, setConfig] = useState<AiConfig | null>(null);

  useEffect(() => {
    const check = async () => {
      try {
        const [s, c] = await Promise.all([
          invoke<OllamaStatus>("get_ollama_status"),
          invoke<AiConfig>("get_ai_config"),
        ]);
        setStatus(s);
        setConfig(c);
      } catch { /* silently ignore */ }
    };
    check();
    const interval = setInterval(check, 15000); // re-check every 15s
    return () => clearInterval(interval);
  }, []);

  if (!config?.enabled) return null;

  const isRunning = status?.running ?? false;
  const dotColor  = isRunning ? "#34d399" : "#f59e0b";
  const label     = isRunning ? config?.model || "AI" : "AI offline";

  return (
    <div style={{
      display: "flex", alignItems: "center", gap: 6,
      padding: "5px 10px", marginTop: 6,
      borderRadius: 20,
      background: isRunning ? "rgba(52,211,153,0.08)" : "rgba(245,158,11,0.08)",
      border: `1px solid ${isRunning ? "rgba(52,211,153,0.2)" : "rgba(245,158,11,0.2)"}`,
      transition: "all 0.3s ease",
    }}>
      {/* Pulsing dot */}
      <span style={{ position: "relative", display: "inline-flex", width: 8, height: 8, flexShrink: 0 }}>
        {isRunning && (
          <span style={{
            position: "absolute", inset: 0, borderRadius: "50%",
            background: `${dotColor}60`,
            animation: "ping 1.5s ease-in-out infinite",
          }} />
        )}
        <span style={{
          position: "relative", display: "block",
          width: 8, height: 8, borderRadius: "50%",
          background: dotColor,
        }} />
      </span>

      <span style={{ fontSize: 10, color: isRunning ? "#34d399" : "#f59e0b", fontWeight: 600, letterSpacing: "0.02em" }}>
        {label}
      </span>

      {pendingCount > 0 && isRunning && (
        <span style={{
          fontSize: 9, fontWeight: 700,
          background: "rgba(167,139,250,0.2)", color: "#a78bfa",
          border: "1px solid rgba(167,139,250,0.3)",
          borderRadius: 10, padding: "1px 5px",
        }}>
          {pendingCount} queued
        </span>
      )}
    </div>
  );
}
