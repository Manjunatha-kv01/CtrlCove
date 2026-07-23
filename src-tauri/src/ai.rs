use serde::{Deserialize, Serialize};
use std::time::Duration;

const ALLOWED_CATEGORIES: &[&str] = &[
    "Programming",
    "Research",
    "Documentation",
    "Networking",
    "Operations",
    "AI",
    "Personal",
    "Work",
    "General",
];

#[derive(Debug, Clone)]
pub struct MemoryInsight {
    pub summary: String,
    pub category: String,
    pub keywords: Vec<String>,
    pub reading_time_minutes: i64,
}

#[derive(Debug, Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

pub async fn enrich(content: &str, content_type: &str, language: Option<&str>) -> MemoryInsight {
    let fallback = rule_based_insight(content, content_type, language);

    if content.trim().len() < 80 {
        return fallback;
    }

    match ollama_insight(content, content_type, language).await {
        Some(insight) => insight,
        None => fallback,
    }
}

async fn ollama_insight(
    content: &str,
    content_type: &str,
    language: Option<&str>,
) -> Option<MemoryInsight> {
    let model = std::env::var("CYMOS_OLLAMA_MODEL").unwrap_or_else(|_| "gemma3:1b".to_string());
    let clipped: String = content.chars().take(4000).collect();
    let prompt = format!(
        "Return compact JSON only with keys summary, category, keywords. Category must be one of Programming, Research, Documentation, Networking, Operations, AI, Personal, Work, General. Content type: {content_type}. Language: {}. Content:\n{clipped}",
        language.unwrap_or("Unknown")
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .ok()?;
    let response = client
        .post("http://127.0.0.1:11434/api/generate")
        .json(&OllamaRequest {
            model: &model,
            prompt,
            stream: false,
        })
        .send()
        .await
        .ok()?
        .json::<OllamaResponse>()
        .await
        .ok()?;

    parse_ai_json(&response.response, content)
}

fn parse_ai_json(raw: &str, content: &str) -> Option<MemoryInsight> {
    let value = serde_json::from_str::<serde_json::Value>(raw).ok()?;
    let summary = compact_model_text(value.get("summary")?.as_str()?, 1_000);
    let requested_category = value
        .get("category")
        .and_then(|value| value.as_str())
        .unwrap_or("General")
        .trim()
        .to_string();
    let category = if ALLOWED_CATEGORIES.contains(&requested_category.as_str()) {
        requested_category
    } else {
        "General".to_string()
    };
    let keywords = value
        .get("keywords")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(|item| compact_model_text(item, 64))
                .filter(|item| !item.is_empty())
                .take(8)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if summary.is_empty() {
        return None;
    }

    Some(MemoryInsight {
        summary,
        category,
        keywords,
        reading_time_minutes: reading_time_minutes(content),
    })
}

fn compact_model_text(value: &str, max_chars: usize) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn rule_based_insight(
    content: &str,
    content_type: &str,
    language: Option<&str>,
) -> MemoryInsight {
    let keywords = extract_keywords(content, language);
    let category = categorize(content, content_type, language);
    let summary = summarize(content, content_type, language);

    MemoryInsight {
        summary,
        category,
        keywords,
        reading_time_minutes: reading_time_minutes(content),
    }
}

fn summarize(content: &str, content_type: &str, language: Option<&str>) -> String {
    if content_type == "Image" {
        return "Copied image memory with stored preview and dimensions.".to_string();
    }
    if content_type == "URL" {
        return format!("Saved link: {}", content.trim());
    }
    if content_type == "Code" {
        return format!(
            "{} code snippet with {} words.",
            language.unwrap_or("Detected"),
            content.split_whitespace().count()
        );
    }

    let clean = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let clipped: String = clean.chars().take(180).collect();
    if clean.chars().count() > 180 {
        format!("{clipped}...")
    } else if clipped.is_empty() {
        "Clipboard memory item.".to_string()
    } else {
        clipped
    }
}

fn categorize(content: &str, content_type: &str, language: Option<&str>) -> String {
    let lower = content.to_lowercase();

    if content_type == "Code" || language.is_some() {
        return "Programming".to_string();
    }
    if lower.contains("ollama")
        || lower.contains("llm")
        || lower.contains("ai")
        || lower.contains("model")
    {
        return "AI".to_string();
    }
    if lower.contains("research")
        || lower.contains("paper")
        || lower.contains("study")
        || lower.contains("abstract")
    {
        return "Research".to_string();
    }
    if lower.contains("http") || lower.contains("docs") || lower.contains("documentation") {
        return "Documentation".to_string();
    }
    if lower.contains("tcp")
        || lower.contains("ip")
        || lower.contains("dns")
        || lower.contains("network")
    {
        return "Networking".to_string();
    }
    if lower.contains("office") || lower.contains("meeting") || lower.contains("project") {
        return "Work".to_string();
    }
    if lower.contains("personal") || lower.contains("note") {
        return "Personal".to_string();
    }

    "General".to_string()
}

fn extract_keywords(content: &str, language: Option<&str>) -> Vec<String> {
    let mut keywords = Vec::new();

    if let Some(language) = language {
        keywords.push(language.to_string());
    }

    let lower = content.to_lowercase();
    for (needle, label) in [
        ("docker", "Docker"),
        ("github", "GitHub"),
        ("rust", "Rust"),
        ("python", "Python"),
        ("typescript", "TypeScript"),
        ("javascript", "JavaScript"),
        ("ollama", "Ollama"),
        ("ai", "AI"),
        ("network", "Networking"),
        ("sql", "SQL"),
        ("tauri", "Tauri"),
        ("react", "React"),
        ("sqlite", "SQLite"),
    ] {
        if lower.contains(needle) {
            keywords.push(label.to_string());
        }
    }

    for word in content
        .split(|ch: char| !ch.is_alphanumeric() && ch != '#')
        .filter(|word| word.len() >= 5 && word.len() <= 24)
        .take(80)
    {
        let normalized = capitalize(word);
        if !STOP_WORDS.contains(&word.to_lowercase().as_str()) {
            keywords.push(normalized);
        }
        if keywords.len() >= 8 {
            break;
        }
    }

    keywords.sort();
    keywords.dedup();
    keywords.truncate(8);
    keywords
}

fn reading_time_minutes(content: &str) -> i64 {
    let words = content.split_whitespace().count() as i64;
    std::cmp::max(1, (words + 199) / 200)
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase()),
        None => String::new(),
    }
}

const STOP_WORDS: &[&str] = &[
    "about", "after", "again", "being", "could", "every", "first", "from", "have", "their",
    "there", "these", "those", "through", "where", "which", "while", "would",
];

#[cfg(test)]
mod tests {
    use super::rule_based_insight;

    #[test]
    fn categorizes_code_as_programming() {
        let insight = rule_based_insight("fn main() {}", "Code", Some("Rust"));
        assert_eq!(insight.category, "Programming");
        assert!(insight.keywords.contains(&"Rust".to_string()));
    }

    #[test]
    fn summarizes_text() {
        let insight = rule_based_insight("CYMOS remembers copied research notes.", "Text", None);
        assert_eq!(insight.category, "Research");
        assert!(insight.summary.contains("CYMOS"));
    }
}
