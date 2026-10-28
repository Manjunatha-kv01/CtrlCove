// ─── Core Types ───────────────────────────────────────────────────────────────

export interface ClipboardItem {
  id: number;
  content: string;
  content_type: ContentType;
  created_at: string;
  updated_at?: string;
  content_hash?: string;
  source_app?: string;
  char_count?: number;
  word_count?: number;
  file_size?: number;
  image_width?: number;
  image_height?: number;
  language?: string;
  is_favorite?: number; // 0 or 1 from SQLite
  collection_id?: number;
  image_data?: string; // "rgba:W:H:base64data" format
  // v0.3 AI fields
  ai_summary?: string;
  ai_category?: AiCategory;
  ai_keywords?: string;   // JSON string e.g. '["rust","async","tokio"]'
  ai_tags?: string;       // JSON string e.g. '["Backend","Systems"]'
  reading_time?: number;  // seconds
  copy_count?: number;
  ai_processed?: number;  // 0 or 1
  ai_model?: string;
  // v0.4 Semantic fields
  embedding_status?: number; // 0 = pending, 1 = embedded
}

export type ContentType =
  | "Text"
  | "URL"
  | "Code"
  | "Image"
  | "HTML"
  | "JSON"
  | "Email"
  | "Phone"
  | "Color"
  | "File";

export type AiCategory =
  | "Programming"
  | "Research"
  | "Documentation"
  | "AI"
  | "Networking"
  | "Personal"
  | "Work"
  | "Other";

// ─── AI Types ─────────────────────────────────────────────────────────────────

export interface OllamaStatus {
  running: boolean;
  models: string[];
}

export interface AiConfig {
  enabled: boolean;
  base_url: string;
  model: string;
  timeout_secs: number;
  min_chars: number;
}

export interface CategoryCount {
  category: string;
  count: number;
}

export interface MemoryInsights {
  top_categories: CategoryCount[];
  top_keywords: string[];
  most_copied: ClipboardItem[];
  ai_processed_count: number;
  ai_pending_count: number;
}

export interface EmbeddingStatus {
  total_embedded: number;
  pending: number;
  model: string;
  model_available: boolean;
}

// ─── Graph Types ──────────────────────────────────────────────────────────────

export interface GraphNode {
  id: number;
  name: string;
  entity_type: string;
  first_seen_at: string;
}

export interface GraphEdge {
  source: number;
  target: number;
  relationship: string;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

// ─── Collection / Tag Types ───────────────────────────────────────────────────

export interface Collection {
  id: number;
  name: string;
  color?: string;
  created_at: string;
}

export interface Tag {
  id: number;
  name: string;
}

export interface Statistics {
  total_items: number;
  text_items: number;
  url_items: number;
  code_items: number;
  image_items: number;
  file_items: number;
  html_items: number;
  json_items: number;
  email_items: number;
  color_items: number;
  favorites_count: number;
  collections_count: number;
  tags_count: number;
  storage_bytes: number;
  ai_processed_count: number;
}

export interface FilterOptions {
  content_type?: string;
  date_from?: string;
  date_to?: string;
  collection_id?: number;
  tag_id?: number;
  is_favorite?: boolean;
  language?: string;
  source_app?: string;
  limit?: number;
}

export type ViewId =
  | "dashboard"
  | "history"
  | "collections"
  | "favorites"
  | "tags"
  | "timeline"
  | "statistics"
  | "semantic"
  | "graph"
  | "assistant"
  | "agent"
  | "autonomous"
  | "settings";

// ─── UI Helpers ──────────────────────────────────────────────────────────────

export const CONTENT_TYPE_CONFIG: Record<
  string,
  { label: string; color: string; bgColor: string; borderColor: string }
> = {
  Text:  { label: "Text",  color: "#60a5fa", bgColor: "rgba(59,130,246,0.1)",  borderColor: "rgba(59,130,246,0.2)" },
  URL:   { label: "Link",  color: "#34d399", bgColor: "rgba(52,211,153,0.1)",  borderColor: "rgba(52,211,153,0.2)" },
  Code:  { label: "Code",  color: "#a78bfa", bgColor: "rgba(167,139,250,0.1)", borderColor: "rgba(167,139,250,0.2)" },
  Image: { label: "Image", color: "#f59e0b", bgColor: "rgba(245,158,11,0.1)",  borderColor: "rgba(245,158,11,0.2)" },
  HTML:  { label: "HTML",  color: "#fb923c", bgColor: "rgba(251,146,60,0.1)",  borderColor: "rgba(251,146,60,0.2)" },
  JSON:  { label: "JSON",  color: "#4ade80", bgColor: "rgba(74,222,128,0.1)",  borderColor: "rgba(74,222,128,0.2)" },
  Email: { label: "Email", color: "#38bdf8", bgColor: "rgba(56,189,248,0.1)",  borderColor: "rgba(56,189,248,0.2)" },
  Phone: { label: "Phone", color: "#94a3b8", bgColor: "rgba(148,163,184,0.1)", borderColor: "rgba(148,163,184,0.2)" },
  Color: { label: "Color", color: "#f472b6", bgColor: "rgba(244,114,182,0.1)", borderColor: "rgba(244,114,182,0.2)" },
  File:  { label: "File",  color: "#fbbf24", bgColor: "rgba(251,191,36,0.1)",  borderColor: "rgba(251,191,36,0.2)" },
};

export const AI_CATEGORY_CONFIG: Record<string, { color: string; bgColor: string; borderColor: string; emoji: string }> = {
  Programming:   { color: "#a78bfa", bgColor: "rgba(167,139,250,0.12)", borderColor: "rgba(167,139,250,0.3)", emoji: "⚡" },
  Research:      { color: "#60a5fa", bgColor: "rgba(96,165,250,0.12)",  borderColor: "rgba(96,165,250,0.3)",  emoji: "🔬" },
  Documentation: { color: "#34d399", bgColor: "rgba(52,211,153,0.12)", borderColor: "rgba(52,211,153,0.3)",  emoji: "📄" },
  AI:            { color: "#f472b6", bgColor: "rgba(244,114,182,0.12)", borderColor: "rgba(244,114,182,0.3)", emoji: "🤖" },
  Networking:    { color: "#38bdf8", bgColor: "rgba(56,189,248,0.12)",  borderColor: "rgba(56,189,248,0.3)",  emoji: "🌐" },
  Personal:      { color: "#fbbf24", bgColor: "rgba(251,191,36,0.12)",  borderColor: "rgba(251,191,36,0.3)",  emoji: "👤" },
  Work:          { color: "#fb923c", bgColor: "rgba(251,146,60,0.12)",  borderColor: "rgba(251,146,60,0.3)",  emoji: "💼" },
  Other:         { color: "#94a3b8", bgColor: "rgba(148,163,184,0.12)", borderColor: "rgba(148,163,184,0.3)", emoji: "📌" },
};

export const LANGUAGE_COLORS: Record<string, string> = {
  Python:     "#3b82f6",
  JavaScript: "#f59e0b",
  TypeScript: "#60a5fa",
  Rust:       "#f97316",
  Java:       "#ef4444",
  "C++":      "#8b5cf6",
  C:          "#6b7280",
  HTML:       "#fb923c",
  CSS:        "#a78bfa",
  SQL:        "#34d399",
  JSON:       "#4ade80",
  Go:         "#38bdf8",
  Shell:      "#94a3b8",
  Markdown:   "#e2e8f0",
  PHP:        "#818cf8",
};

export const COLLECTION_COLORS = [
  "#3b82f6", "#8b5cf6", "#ec4899", "#f59e0b",
  "#10b981", "#06b6d4", "#f97316", "#ef4444",
  "#6366f1", "#84cc16", "#14b8a6", "#a855f7",
];

// ─── Helpers ─────────────────────────────────────────────────────────────────

export function parseJsonArray(s?: string): string[] {
  if (!s) return [];
  try { return JSON.parse(s) as string[]; } catch { return []; }
}

export function readingTimeLabel(seconds?: number): string {
  if (!seconds || seconds <= 0) return "";
  if (seconds < 60) return `${seconds}s read`;
  return `${Math.ceil(seconds / 60)} min read`;
}

// ─── Assistant Types (v0.0.6) ─────────────────────────────────────────────────

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  sources?: ClipboardItem[];
}

export interface MemorySummary {
  summary_text: string;
  total_copied: number;
  top_category: string;
}

// ─── Agent Types (v0.0.7) ──────────────────────────────────────────────

export type AgentTypeKey =
  | "memory" | "research" | "planning"
  | "learning" | "coding" | "document" | "search" | "orchestrator";

export interface AgentStep {
  agent: string;
  output: string;
}

export interface AgentResult {
  task_id: number;
  agent_type: AgentTypeKey;
  task: string;
  status: "pending" | "running" | "done" | "error";
  result: string;
  steps: AgentStep[];
  sources: ClipboardItem[];
  created_at: string;
}

export interface AgentHistoryTask {
  id: number;
  agent_type: AgentTypeKey;
  task: string;
  status: string;
  result: string | null;
  steps: string | null;     // JSON string
  source_ids: string | null; // JSON string
  created_at: string;
}

// ─── Autonomous Intelligence Types (v0.0.8) ──────────────────────────────────

export interface Notification {
  id: number;
  notification_type: 'duplicate' | 'stale' | 'insight' | 'milestone' | string;
  title: string;
  message: string;
  is_read: number;
  related_ids?: string;
  created_at: string;
}

export interface SchedulerRun {
  id: number;
  task_type: string;
  status: string;
  items_processed: number;
  duration_ms: number;
  created_at: string;
}

export interface KnowledgeHealth {
  id: number;
  total_memories: number;
  embedded_count: number;
  ai_processed_count: number;
  graph_entities: number;
  graph_relationships: number;
  duplicate_groups: number;
  avg_connections: number;
  created_at: string;
}
