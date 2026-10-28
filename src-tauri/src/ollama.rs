use serde::{Deserialize, Serialize};
use std::time::Duration;

// ─── Configuration ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub min_chars: usize,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: "http://localhost:11434".to_string(),
            model: "gemma3:1b".to_string(),
            timeout_secs: 30,
            min_chars: 50,
        }
    }
}

// ─── AI Result ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResult {
    pub summary: String,
    pub category: String,
    pub keywords: Vec<String>,
    pub reading_time_seconds: i64,
    pub auto_tags: Vec<String>,
}

impl Default for AiResult {
    fn default() -> Self {
        Self {
            summary: String::new(),
            category: "Other".to_string(),
            keywords: vec![],
            reading_time_seconds: 0,
            auto_tags: vec![],
        }
    }
}

// ─── Ollama API Types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    format: String,
    options: GenerateOptions,
}

#[derive(Serialize)]
struct GenerateOptions {
    temperature: f32,
    num_predict: i32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ModelInfo {
    pub name: String,
}

// ─── Status ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaStatus {
    pub running: bool,
    pub models: Vec<String>,
}

// ─── Prompt Building ──────────────────────────────────────────────────────────

fn build_prompt(content: &str, content_type: &str, language: Option<&str>) -> String {
    let type_hint = match content_type {
        "Code"  => format!("a {} code snippet", language.unwrap_or("programming")),
        "URL"   => "a URL or web link".to_string(),
        "JSON"  => "JSON data".to_string(),
        "HTML"  => "HTML markup".to_string(),
        "Email" => "an email address".to_string(),
        "Color" => "a color code".to_string(),
        "Image" => "an image".to_string(),
        _       => "text content".to_string(),
    };

    // Truncate to 2000 chars to keep prompt fast
    let truncated = if content.len() > 2000 {
        format!("{}... [truncated]", &content[..2000])
    } else {
        content.to_string()
    };

    format!(
        r#"You are a memory enrichment AI. Analyze this clipboard item and respond ONLY with a single valid JSON object. No explanation, no markdown, no code blocks — just raw JSON.

Content type: {type_hint}

Content:
{truncated}

Respond with exactly this JSON structure:
{{
  "summary": "1-2 sentence summary of what this content is or does",
  "category": "one of: Programming, Research, Documentation, AI, Networking, Personal, Work, Other",
  "keywords": ["keyword1", "keyword2", "keyword3"],
  "reading_time_seconds": 30,
  "auto_tags": ["tag1", "tag2", "tag3"]
}}"#
    )
}

// ─── Core Functions ───────────────────────────────────────────────────────────

/// Check if Ollama is running and list available models
pub async fn get_status(config: &OllamaConfig) -> OllamaStatus {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return OllamaStatus { running: false, models: vec![] },
    };

    let url = format!("{}/api/tags", config.base_url);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let models = resp
                .json::<TagsResponse>()
                .await
                .map(|r| r.models.iter().map(|m| m.name.clone()).collect())
                .unwrap_or_default();
            OllamaStatus { running: true, models }
        }
        _ => OllamaStatus { running: false, models: vec![] },
    }
}

/// Enrich a clipboard item with AI analysis
pub async fn enrich_item(
    content: &str,
    content_type: &str,
    language: Option<&str>,
    config: &OllamaConfig,
) -> Result<AiResult, String> {
    // Skip if content too short
    if content.len() < config.min_chars {
        return Ok(AiResult {
            summary: content.to_string(),
            ..AiResult::default()
        });
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .build()
        .map_err(|e| e.to_string())?;

    let prompt = build_prompt(content, content_type, language);

    let request = GenerateRequest {
        model: config.model.clone(),
        prompt,
        stream: false,
        format: "json".to_string(),
        options: GenerateOptions {
            temperature: 0.1, // Low temp for consistent structured output
            num_predict: 512,
        },
    };

    let url = format!("{}/api/generate", config.base_url);
    let resp = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Ollama returned status: {}", resp.status()));
    }

    let gen_resp: GenerateResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    // Parse the JSON response
    parse_ai_response(&gen_resp.response)
}

fn parse_ai_response(raw: &str) -> Result<AiResult, String> {
    // Try to extract JSON from the response (model might wrap it)
    let json_str = extract_json(raw);

    let val: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("JSON parse error: {} | raw: {}", e, &json_str[..json_str.len().min(200)]))?;

    let summary = val["summary"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    let category = val["category"]
        .as_str()
        .map(|s| normalize_category(s))
        .unwrap_or_else(|| "Other".to_string());

    let keywords: Vec<String> = val["keywords"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .take(5)
                .collect()
        })
        .unwrap_or_default();

    let reading_time_seconds = val["reading_time_seconds"]
        .as_i64()
        .unwrap_or(0);

    let auto_tags: Vec<String> = val["auto_tags"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .take(4)
                .collect()
        })
        .unwrap_or_default();

    Ok(AiResult {
        summary,
        category,
        keywords,
        reading_time_seconds,
        auto_tags,
    })
}

fn extract_json(s: &str) -> String {
    let s = s.trim();
    // Try to find first { and last }
    if let (Some(start), Some(end)) = (s.find('{'), s.rfind('}')) {
        if end > start {
            return s[start..=end].to_string();
        }
    }
    s.to_string()
}

fn normalize_category(s: &str) -> String {
    let valid = [
        "Programming", "Research", "Documentation",
        "AI", "Networking", "Personal", "Work", "Other",
    ];
    let title = {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().to_string() + c.as_str(),
        }
    };
    if valid.contains(&title.as_str()) {
        title
    } else {
        // Best-effort fuzzy match
        let lower = s.to_lowercase();
        if lower.contains("program") || lower.contains("code") || lower.contains("dev") {
            "Programming".to_string()
        } else if lower.contains("research") || lower.contains("study") || lower.contains("paper") {
            "Research".to_string()
        } else if lower.contains("doc") || lower.contains("manual") || lower.contains("guide") {
            "Documentation".to_string()
        } else if lower.contains("ai") || lower.contains("ml") || lower.contains("llm") {
            "AI".to_string()
        } else if lower.contains("net") || lower.contains("api") || lower.contains("http") {
            "Networking".to_string()
        } else if lower.contains("work") || lower.contains("office") || lower.contains("meeting") {
            "Work".to_string()
        } else if lower.contains("personal") || lower.contains("private") {
            "Personal".to_string()
        } else {
            "Other".to_string()
        }
    }
}

// ─── Graph Extraction ─────────────────────────────────────────────────────────

use crate::graph::ExtractedGraphData;

pub async fn extract_graph(
    content: &str,
    config: &OllamaConfig,
) -> Result<ExtractedGraphData, String> {
    if content.len() < config.min_chars {
        return Ok(ExtractedGraphData {
            entities: vec![],
            relationships: vec![],
        });
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .build()
        .map_err(|e| e.to_string())?;

    let truncated = if content.len() > 3000 {
        format!("{}... [truncated]", &content[..3000])
    } else {
        content.to_string()
    };

    let prompt = format!(
        r#"You are a Knowledge Graph extractor. Analyze this content and extract key entities and relationships.
Respond ONLY with a valid JSON object matching exactly this structure:
{{
  "entities": [
    {{"name": "Docker", "entity_type": "Technology"}},
    {{"name": "FastAPI", "entity_type": "Technology"}}
  ],
  "relationships": [
    {{"source": "FastAPI", "target": "Docker", "relationship_type": "deployed_on"}}
  ]
}}

Entity types should be one of: Person, Organization, Project, Technology, Language, Concept.
Content:
{truncated}"#
    );

    let request = GenerateRequest {
        model: config.model.clone(),
        prompt,
        stream: false,
        format: "json".to_string(),
        options: GenerateOptions {
            temperature: 0.1,
            num_predict: 1024,
        },
    };

    let url = format!("{}/api/generate", config.base_url);
    let resp = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Ollama returned status: {}", resp.status()));
    }

    let gen_resp: GenerateResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    let json_str = extract_json(&gen_resp.response);
    serde_json::from_str(&json_str)
        .map_err(|e| format!("Graph JSON parse error: {}", e))
}

pub async fn generate_text(
    prompt: &str,
    config: &OllamaConfig,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .build()
        .map_err(|e| e.to_string())?;

    let request = GenerateRequest {
        model: config.model.clone(),
        prompt: prompt.to_string(),
        stream: false,
        format: "".to_string(),
        options: GenerateOptions {
            temperature: 0.7,
            num_predict: 1024,
        },
    };

    let url = format!("{}/api/generate", config.base_url);
    let resp = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Ollama returned status: {}", resp.status()));
    }

    let gen_resp: GenerateResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    Ok(gen_resp.response)
}
