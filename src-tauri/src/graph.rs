use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub id: i64,
    pub name: String,
    pub entity_type: String,
    pub weight: i64,
    pub cluster: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub source: i64,
    pub target: i64,
    pub relationship: String,
    pub weight: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopicCluster {
    pub name: String,
    pub count: i64,
    pub entities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub clusters: Vec<TopicCluster>,
    pub recommendations: Vec<String>,
}

pub fn extract_entities(
    content: &str,
    content_type: &str,
    language: Option<&str>,
    category: &str,
    keywords: &[String],
    tags: &[String],
) -> Vec<ExtractedEntity> {
    let mut entities = BTreeMap::<String, String>::new();

    for value in keywords.iter().chain(tags.iter()) {
        insert_entity(
            &mut entities,
            value,
            classify_entity(value, content_type, category),
        );
    }

    if let Some(language) = language {
        insert_entity(&mut entities, language, "Programming Language");
    }

    for (needle, entity_type) in KNOWN_ENTITIES {
        if contains_word(content, needle) {
            insert_entity(&mut entities, needle, entity_type);
        }
    }

    for token in content.split_whitespace() {
        let clean = token.trim_matches(|ch: char| {
            matches!(
                ch,
                ',' | '.' | ';' | ':' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}'
            )
        });

        if clean.starts_with("http://")
            || clean.starts_with("https://")
            || clean.starts_with("www.")
        {
            insert_entity(&mut entities, clean, "URL");
        } else if clean.contains('.') && clean.rsplit('.').next().is_some_and(|ext| ext.len() <= 5)
        {
            insert_entity(&mut entities, clean, "Document");
        } else if clean.ends_with("()") || clean.contains("::") || clean.contains("/api/") {
            insert_entity(&mut entities, clean, "API");
        } else if clean.starts_with("git") || clean.starts_with("npm") || clean.starts_with("cargo")
        {
            insert_entity(&mut entities, clean, "Command");
        } else if is_title_case(clean) && clean.len() > 2 {
            insert_entity(&mut entities, clean, "Concept");
        }
    }

    entities
        .into_iter()
        .map(|(name, entity_type)| ExtractedEntity { name, entity_type })
        .collect()
}

pub fn cluster_for_entities(category: &str, entities: &[ExtractedEntity]) -> String {
    let names = entities
        .iter()
        .map(|entity| entity.name.to_lowercase())
        .collect::<Vec<_>>();

    if names.iter().any(|name| {
        [
            "docker",
            "kubernetes",
            "fastapi",
            "rust",
            "python",
            "react",
            "tauri",
        ]
        .contains(&name.as_str())
    }) {
        return "Programming".to_string();
    }
    if names.iter().any(|name| {
        ["bsnl", "5g", "fiber", "dns", "tcp", "networking", "network"].contains(&name.as_str())
    }) {
        return "Networking".to_string();
    }
    if names
        .iter()
        .any(|name| ["ai", "ollama", "llm", "qdrant", "neo4j"].contains(&name.as_str()))
    {
        return "AI Systems".to_string();
    }
    if !category.trim().is_empty() && category != "Uncategorized" {
        return category.to_string();
    }

    "General Knowledge".to_string()
}

pub fn relationship_between(left: &ExtractedEntity, right: &ExtractedEntity) -> String {
    if left.entity_type == "Programming Language" || right.entity_type == "Programming Language" {
        return "USES_LANGUAGE".to_string();
    }
    if left.entity_type == "Technology" && right.entity_type == "Technology" {
        return "RELATED_TECH".to_string();
    }
    if left.entity_type == "URL" || right.entity_type == "URL" {
        return "REFERENCES".to_string();
    }
    if left.entity_type == "Command" || right.entity_type == "Command" {
        return "USES_COMMAND".to_string();
    }
    if left.entity_type == "Document" || right.entity_type == "Document" {
        return "MENTIONS_DOCUMENT".to_string();
    }
    "RELATED_TO".to_string()
}

pub fn recommendations(nodes: &[GraphNode], clusters: &[TopicCluster]) -> Vec<String> {
    let mut output = Vec::new();

    if let Some(top) = nodes.iter().max_by_key(|node| node.weight) {
        output.push(format!("Explore more around {}", top.name));
    }

    if let Some(cluster) = clusters.iter().max_by_key(|cluster| cluster.count) {
        output.push(format!("Your strongest topic cluster is {}", cluster.name));
    }

    let has_docker = nodes
        .iter()
        .any(|node| node.name.eq_ignore_ascii_case("docker"));
    let has_kubernetes = nodes
        .iter()
        .any(|node| node.name.eq_ignore_ascii_case("kubernetes"));
    if has_docker && !has_kubernetes {
        output.push("Missing connection: Docker often links to Kubernetes".to_string());
    }

    let has_fastapi = nodes
        .iter()
        .any(|node| node.name.eq_ignore_ascii_case("fastapi"));
    let has_python = nodes
        .iter()
        .any(|node| node.name.eq_ignore_ascii_case("python"));
    if has_fastapi && !has_python {
        output.push("Missing connection: FastAPI usually connects to Python".to_string());
    }

    output.truncate(5);
    output
}

pub fn clusters_from_nodes(nodes: &[GraphNode]) -> Vec<TopicCluster> {
    let mut clusters = BTreeMap::<String, (i64, BTreeSet<String>)>::new();

    for node in nodes {
        let entry = clusters
            .entry(node.cluster.clone())
            .or_insert_with(|| (0, BTreeSet::new()));
        entry.0 += node.weight;
        entry.1.insert(node.name.clone());
    }

    clusters
        .into_iter()
        .map(|(name, (count, entities))| TopicCluster {
            name,
            count,
            entities: entities.into_iter().take(8).collect(),
        })
        .collect()
}

fn insert_entity(entities: &mut BTreeMap<String, String>, value: &str, entity_type: &str) {
    let clean = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '#' | ',' | '.' | '"' | '\'' | '`'))
        .to_string();

    if clean.len() < 2 || STOP_WORDS.contains(&clean.to_lowercase().as_str()) {
        return;
    }

    entities
        .entry(clean)
        .or_insert_with(|| entity_type.to_string());
}

fn classify_entity(value: &str, content_type: &str, category: &str) -> &'static str {
    let lower = value.to_lowercase();

    if TECHNOLOGIES.contains(&lower.as_str()) {
        "Technology"
    } else if PROGRAMMING_LANGUAGES.contains(&lower.as_str()) {
        "Programming Language"
    } else if ORGANIZATIONS.contains(&lower.as_str()) {
        "Organization"
    } else if content_type == "URL" || lower.starts_with("http") {
        "URL"
    } else if category == "Programming" {
        "Technology"
    } else {
        "Concept"
    }
}

fn contains_word(content: &str, needle: &str) -> bool {
    content.to_lowercase().contains(&needle.to_lowercase())
}

fn is_title_case(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|first| first.is_uppercase())
        && value.chars().any(|ch| ch.is_lowercase())
}

const KNOWN_ENTITIES: &[(&str, &str)] = &[
    ("BSNL", "Organization"),
    ("CYMOS", "Project"),
    ("Docker", "Technology"),
    ("Kubernetes", "Technology"),
    ("FastAPI", "Technology"),
    ("Rust", "Programming Language"),
    ("Python", "Programming Language"),
    ("TypeScript", "Programming Language"),
    ("JavaScript", "Programming Language"),
    ("React", "Technology"),
    ("Tauri", "Technology"),
    ("SQLite", "Technology"),
    ("Qdrant", "Technology"),
    ("Neo4j", "Technology"),
    ("Ollama", "Technology"),
    ("GitHub", "Technology"),
    ("5G", "Technology"),
    ("Fiber", "Technology"),
    ("API", "API"),
];

const TECHNOLOGIES: &[&str] = &[
    "docker",
    "kubernetes",
    "fastapi",
    "react",
    "tauri",
    "sqlite",
    "qdrant",
    "neo4j",
    "ollama",
    "github",
    "api",
    "5g",
    "fiber",
];

const PROGRAMMING_LANGUAGES: &[&str] = &[
    "rust",
    "python",
    "typescript",
    "javascript",
    "java",
    "sql",
    "html",
    "css",
];

const ORGANIZATIONS: &[&str] = &["bsnl", "github", "openai"];

const STOP_WORDS: &[&str] = &[
    "the", "and", "for", "with", "from", "this", "that", "your", "into", "using", "copied",
    "memory", "notes",
];

#[cfg(test)]
mod tests {
    use super::{cluster_for_entities, extract_entities};

    #[test]
    fn extracts_technology_entities() {
        let entities = extract_entities(
            "Deploy FastAPI using Docker and Python",
            "Text",
            None,
            "Programming",
            &[],
            &[],
        );
        assert!(entities.iter().any(|entity| entity.name == "FastAPI"));
        assert!(entities.iter().any(|entity| entity.name == "Docker"));
    }

    #[test]
    fn clusters_networking() {
        let entities = extract_entities(
            "BSNL 5G Fiber monitoring",
            "Text",
            None,
            "General",
            &[],
            &[],
        );
        assert_eq!(cluster_for_entities("General", &entities), "Networking");
    }
}
