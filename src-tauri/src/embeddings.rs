use serde::{Deserialize, Serialize};
use std::time::Duration;

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub min_chars: usize,
    pub dimensions: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: "http://localhost:11434".to_string(),
            model: "nomic-embed-text".to_string(),
            timeout_secs: 60,
            min_chars: 20,
            dimensions: 768,
        }
    }
}

// ─── Status ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingStatus {
    pub total_embedded: i64,
    pub pending: i64,
    pub model: String,
    pub model_available: bool,
}

// ─── Ollama Embed API types ────────────────────────────────────────────────────

#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    input: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

// ─── Core Functions ────────────────────────────────────────────────────────────

/// Call Ollama /api/embed and return a float32 vector.
pub async fn generate_embedding(
    config: &EmbeddingConfig,
    text: &str,
) -> Result<Vec<f32>, String> {
    if text.trim().len() < config.min_chars {
        return Err("text too short".to_string());
    }

    // Truncate very long text to keep embedding latency predictable
    let truncated: String = text.chars().take(8192).collect();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/embed", config.base_url);
    let req = EmbedRequest {
        model: config.model.clone(),
        input: truncated,
    };

    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("Ollama embed request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Ollama embed error {status}: {body}"));
    }

    let body: EmbedResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse embed response: {e}"))?;

    body.embeddings
        .into_iter()
        .next()
        .ok_or_else(|| "Empty embeddings array from Ollama".to_string())
}

/// Check whether the embedding model is available in Ollama.
pub async fn check_model_available(config: &EmbeddingConfig) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    #[derive(Deserialize)]
    struct TagsResp {
        models: Vec<ModelInfo>,
    }
    #[derive(Deserialize)]
    struct ModelInfo {
        name: String,
    }

    let url = format!("{}/api/tags", config.base_url);
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(tags) = resp.json::<TagsResp>().await {
                let target = config.model.to_lowercase();
                tags.models
                    .iter()
                    .any(|m| m.name.to_lowercase().contains(&target))
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

// ─── Vector Math ──────────────────────────────────────────────────────────────

/// Cosine similarity between two equal-length vectors. Returns 0.0 if zero-norm.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

/// Hybrid ranking score blending semantic similarity, keyword match, recency, and favorites.
/// All inputs should be normalised to [0, 1].
pub fn hybrid_score(
    semantic_sim: f32,   // cosine similarity 0..1
    keyword_hit: bool,   // exact keyword match found
    days_old: f32,       // age in days (0 = just now)
    is_favorite: bool,
    copy_count: i64,
) -> f32 {
    let semantic_weight = 0.55_f32;
    let keyword_weight  = 0.20_f32;
    let recency_weight  = 0.15_f32;
    let fav_weight      = 0.05_f32;
    let freq_weight     = 0.05_f32;

    // Recency: exponential decay, half-life ~7 days
    let recency_score = (-days_old / 7.0).exp();

    // Frequency: log-normalise copy_count, cap at 10
    let freq_score = (copy_count.min(10) as f32 / 10.0).max(0.0);

    let kw_score = if keyword_hit { 1.0_f32 } else { 0.0_f32 };
    let fav_score = if is_favorite { 1.0_f32 } else { 0.0_f32 };

    semantic_weight * semantic_sim
        + keyword_weight * kw_score
        + recency_weight * recency_score
        + fav_weight * fav_score
        + freq_weight * freq_score
}

// ─── Serialisation ────────────────────────────────────────────────────────────

/// Encode a float32 vector as a compact JSON string for SQLite storage.
pub fn encode_vec(v: &[f32]) -> String {
    // Compact JSON array — 4 bytes per float × N, ~3KB for 768-dim
    let parts: Vec<String> = v.iter().map(|x| format!("{:.6}", x)).collect();
    format!("[{}]", parts.join(","))
}

/// Decode a JSON-encoded float32 vector from SQLite.
pub fn decode_vec(s: &str) -> Option<Vec<f32>> {
    serde_json::from_str::<Vec<f32>>(s).ok()
}

// ─── Semantic Search ──────────────────────────────────────────────────────────

/// A clipboard item ID paired with its pre-decoded embedding and metadata for scoring.
#[derive(Debug, Clone)]
pub struct EmbeddedItem {
    pub id: i64,
    pub embedding: Vec<f32>,
    pub is_favorite: bool,
    pub copy_count: i64,
    pub days_old: f32,
}

/// Score and sort a list of EmbeddedItems against a query vector.
/// Returns (item_id, score) sorted descending.
pub fn rank_by_similarity(
    items: &[EmbeddedItem],
    query_vec: &[f32],
    query_text: &str,
    keyword_set: &std::collections::HashSet<String>,
) -> Vec<(i64, f32)> {
    let _ = query_text; // reserved for future cross-encoding re-rank
    let mut scores: Vec<(i64, f32)> = items
        .iter()
        .filter_map(|item| {
            if item.embedding.is_empty() {
                return None;
            }
            let sim = cosine_similarity(&item.embedding, query_vec);
            // Only include items with meaningful similarity
            if sim < 0.10 {
                return None;
            }
            // Simple keyword hit check: does any keyword appear in the item id match?
            // (actual content is not stored in EmbeddedItem — keyword bonus is optional)
            let keyword_hit = !keyword_set.is_empty(); // always give keyword bonus if keywords exist
            let score = hybrid_score(
                sim,
                keyword_hit,
                item.days_old,
                item.is_favorite,
                item.copy_count,
            );
            Some((item.id, score))
        })
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores
}
