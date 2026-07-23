use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;

const EMBEDDING_DIMS: usize = 128;

#[derive(Debug, Clone)]
pub struct SemanticVector {
    pub values: Vec<f32>,
    pub source: String,
}

#[derive(Debug, Serialize)]
struct OllamaEmbeddingRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

pub async fn embed(text: &str) -> SemanticVector {
    if let Some(values) = ollama_embedding(text).await {
        // Mixing embedding dimensions makes cosine scores silently meaningless.
        if values.len() == EMBEDDING_DIMS {
            return SemanticVector {
                values: normalize(values),
                source: "Ollama".to_string(),
            };
        }
    }

    SemanticVector {
        values: local_embedding(text),
        source: "Local".to_string(),
    }
}

pub fn local_embedding(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0_f32; EMBEDDING_DIMS];
    let normalized = text.to_lowercase();

    for token in normalized
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| token.len() >= 2)
    {
        let hash = Sha256::digest(token.as_bytes());
        let index = ((hash[0] as usize) << 8 | hash[1] as usize) % EMBEDDING_DIMS;
        let sign = if hash[2] % 2 == 0 { 1.0 } else { -1.0 };
        let weight = if token.len() > 6 { 1.4 } else { 1.0 };
        vector[index] += sign * weight;
    }

    normalize(vector)
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;

    for (left_value, right_value) in left.iter().zip(right.iter()) {
        dot += left_value * right_value;
        left_norm += left_value * left_value;
        right_norm += right_value * right_value;
    }

    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}

pub fn serialize_embedding(values: &[f32]) -> String {
    values
        .iter()
        .map(|value| format!("{value:.6}"))
        .collect::<Vec<_>>()
        .join(",")
}

pub fn deserialize_embedding(value: &str) -> Vec<f32> {
    value
        .split(',')
        .filter_map(|part| part.parse::<f32>().ok())
        .collect()
}

pub fn semantic_text(
    content: &str,
    content_type: &str,
    language: Option<&str>,
    summary: &str,
    category: &str,
    keywords: &[String],
    tags: &[String],
) -> String {
    format!(
        "{content}\nType: {content_type}\nLanguage: {}\nSummary: {summary}\nCategory: {category}\nKeywords: {}\nTags: {}",
        language.unwrap_or("Unknown"),
        keywords.join(" "),
        tags.join(" ")
    )
}

fn normalize(mut values: Vec<f32>) -> Vec<f32> {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();

    if norm > 0.0 {
        for value in &mut values {
            *value /= norm;
        }
    }

    values
}

async fn ollama_embedding(text: &str) -> Option<Vec<f32>> {
    let model =
        std::env::var("CYMOS_EMBED_MODEL").unwrap_or_else(|_| "nomic-embed-text".to_string());
    let clipped: String = text.chars().take(4000).collect();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .ok()?;

    client
        .post("http://127.0.0.1:11434/api/embeddings")
        .json(&OllamaEmbeddingRequest {
            model: &model,
            prompt: &clipped,
        })
        .send()
        .await
        .ok()?
        .json::<OllamaEmbeddingResponse>()
        .await
        .ok()
        .map(|response| response.embedding)
        .filter(|embedding| !embedding.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{cosine_similarity, local_embedding, EMBEDDING_DIMS};

    #[test]
    fn similar_texts_score_higher() {
        let rust = local_embedding("Rust ownership borrow checker lifetime");
        let rust_query = local_embedding("borrow checker ownership notes");
        let cooking = local_embedding("cooking recipe spices dinner");

        assert!(cosine_similarity(&rust, &rust_query) > cosine_similarity(&rust, &cooking));
    }

    #[test]
    fn local_embeddings_have_a_stable_dimension() {
        assert_eq!(local_embedding("CYMOS").len(), EMBEDDING_DIMS);
    }
}
