use crate::ai;
use crate::database::NewClipboardItem;
use crate::operations;
use crate::privacy::{self, TextCaptureDecision};
use crate::semantic;
use crate::validation;
use crate::AppState;
use arboard::Clipboard;
use image::{DynamicImage, ImageFormat, RgbaImage};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::net::IpAddr;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

#[derive(Debug, Deserialize)]
pub struct BrowserBookmarkRequest {
    pub url: String,
    pub title: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct IdeSnippetRequest {
    pub content: String,
    pub title: String,
    pub language: String,
    pub project: String,
    pub file_path: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TerminalCommandRequest {
    pub command: String,
    pub shell: String,
    pub host: String,
    pub project: String,
    pub tags: Vec<String>,
}

pub fn start_monitor(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_text_hash = String::new();
        let mut last_image_hash = String::new();

        loop {
            if let Some(content) = read_clipboard_text() {
                let content = content.trim().to_string();
                let hash = hash_bytes(content.as_bytes());

                if !content.is_empty() && hash != last_text_hash {
                    if let Err(error) = validation::clipboard_text(&content) {
                        eprintln!("clipboard text skipped: {error}");
                        last_text_hash = hash;
                        sleep(Duration::from_millis(750)).await;
                        continue;
                    }

                    let state = app.state::<AppState>();
                    let Ok(permit) = state.ingestion_limiter.clone().try_acquire_owned() else {
                        // Keep the hash unchanged so the active clipboard value is retried when capacity returns.
                        sleep(Duration::from_millis(750)).await;
                        continue;
                    };
                    last_text_hash = hash.clone();
                    let database = state.database.clone();
                    let graph_lock = state.graph_lock.clone();
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _permit = permit;
                        let settings = match database.privacy_settings().await {
                            Ok(settings) => settings,
                            Err(error) => {
                                eprintln!("clipboard privacy settings unavailable: {error}");
                                return;
                            }
                        };
                        match privacy::text_capture_decision(&settings, &content) {
                            TextCaptureDecision::Allow => {}
                            TextCaptureDecision::Skip => return,
                            TextCaptureDecision::Block(reason) => {
                                if let Err(error) =
                                    database.record_privacy_block("Clipboard", &reason).await
                                {
                                    eprintln!(
                                        "failed to record blocked clipboard capture: {error}"
                                    );
                                }
                                let _ = app_handle.emit("privacy-capture-blocked", ());
                                return;
                            }
                        }
                        let metadata = analyze_text(&content, &hash).await;
                        let _graph_guard = graph_lock.lock().await;
                        match database.insert_item(metadata).await {
                            Ok(Some(_)) => {
                                let _ = app_handle.emit("clipboard-item-created", ());
                            }
                            Ok(None) => {}
                            Err(error) => eprintln!("failed to store clipboard text: {error}"),
                        }
                    });
                }
            }

            if let Some(image) = read_clipboard_image() {
                let hash = hash_bytes(&image.bytes);

                if hash != last_image_hash {
                    if let Err(error) =
                        validation::clipboard_image(image.width, image.height, image.bytes.len())
                    {
                        eprintln!("clipboard image skipped: {error}");
                        last_image_hash = hash;
                        sleep(Duration::from_millis(750)).await;
                        continue;
                    }

                    let state = app.state::<AppState>();
                    let Ok(permit) = state.ingestion_limiter.clone().try_acquire_owned() else {
                        sleep(Duration::from_millis(750)).await;
                        continue;
                    };
                    last_image_hash = hash.clone();
                    let database = state.database.clone();
                    let graph_lock = state.graph_lock.clone();
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _permit = permit;
                        match database.privacy_settings().await {
                            Ok(settings) if settings.capture_images => {}
                            Ok(_) => return,
                            Err(error) => {
                                eprintln!("clipboard privacy settings unavailable: {error}");
                                return;
                            }
                        }
                        let assets_dir = database.assets_dir();
                        let asset_hash = hash.clone();
                        let image_result = tauri::async_runtime::spawn_blocking(move || {
                            store_image(
                                &assets_dir,
                                &image.bytes,
                                image.width,
                                image.height,
                                &asset_hash,
                            )
                        })
                        .await;

                        match image_result {
                            Ok(Ok((path, size))) => {
                                let item = NewClipboardItem {
                                    content: path.clone(),
                                    content_type: "Image".to_string(),
                                    source_application: "Unknown".to_string(),
                                    content_hash: hash.clone(),
                                    character_count: 0,
                                    word_count: 0,
                                    file_size: Some(size),
                                    image_width: Some(image.width as i64),
                                    image_height: Some(image.height as i64),
                                    language: None,
                                    ai_summary:
                                        "Copied image memory with stored preview and dimensions."
                                            .to_string(),
                                    category: "General".to_string(),
                                    keywords: vec!["Image".to_string(), "Screenshot".to_string()],
                                    reading_time_minutes: 1,
                                    semantic_text: format!(
                                        "Image memory screenshot file {path} dimensions {}x{}",
                                        image.width, image.height
                                    ),
                                    embedding: semantic::local_embedding(&format!(
                                        "Image screenshot {}x{}",
                                        image.width, image.height
                                    )),
                                    embedding_source: "Local".to_string(),
                                    operational_context: operations::OperationalContext::default(),
                                    tags: vec!["Image".to_string()],
                                };

                                let _graph_guard = graph_lock.lock().await;
                                match database.insert_item(item).await {
                                    Ok(Some(_)) => {
                                        let _ = app_handle.emit("clipboard-item-created", ());
                                    }
                                    Ok(None) => {}
                                    Err(error) => {
                                        eprintln!("failed to store clipboard image: {error}")
                                    }
                                }
                            }
                            Ok(Err(error)) => {
                                eprintln!("failed to persist clipboard image: {error}")
                            }
                            Err(error) => eprintln!("image persistence task failed: {error}"),
                        }
                    });
                }
            }

            sleep(Duration::from_millis(750)).await;
        }
    });
}

fn read_clipboard_text() -> Option<String> {
    let mut clipboard = Clipboard::new().ok()?;
    clipboard.get_text().ok()
}

fn read_clipboard_image() -> Option<ClipboardImage> {
    let mut clipboard = Clipboard::new().ok()?;
    let image = clipboard.get_image().ok()?;
    Some(ClipboardImage {
        width: image.width,
        height: image.height,
        bytes: image.bytes.into_owned(),
    })
}

struct ClipboardImage {
    width: usize,
    height: usize,
    bytes: Vec<u8>,
}

fn store_image(
    assets_dir: &Path,
    rgba_bytes: &[u8],
    width: usize,
    height: usize,
    hash: &str,
) -> Result<(String, i64), Box<dyn std::error::Error + Send + Sync>> {
    fs::create_dir_all(assets_dir)?;
    let image_path = assets_dir.join(format!("{hash}.png"));

    if !image_path.exists() {
        let buffer = RgbaImage::from_raw(width as u32, height as u32, rgba_bytes.to_vec())
            .ok_or("invalid clipboard image buffer")?;
        let dynamic = DynamicImage::ImageRgba8(buffer);
        let mut png = Vec::new();
        dynamic.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
        fs::write(&image_path, &png)?;
    }

    let file_size = fs::metadata(&image_path)?.len() as i64;
    Ok((image_path.to_string_lossy().to_string(), file_size))
}

pub async fn analyze_text(content: &str, hash: &str) -> NewClipboardItem {
    let analysis_content = validation::analysis_excerpt(content);
    let operational_context = operations::analyze(&analysis_content, None);
    let content_type = if operational_context.kind == "Terminal Command" {
        "Code"
    } else {
        classify_content(&analysis_content)
    };
    let language = if operational_context.kind == "Terminal Command" {
        Some("Bash")
    } else {
        detect_language(&analysis_content)
    };
    let mut tags = auto_tags(&analysis_content, content_type, language);
    let mut insight = if operational_context.is_operational() {
        ai::rule_based_insight(&analysis_content, content_type, language)
    } else {
        ai::enrich(&analysis_content, content_type, language).await
    };
    if operational_context.is_operational() {
        insight.category = "Operations".to_string();
        insight.summary = operational_context.summary();
        insight
            .keywords
            .extend(operational_context.services.clone());
        insight
            .keywords
            .extend(operational_context.technologies.clone());
        insight.keywords.sort();
        insight.keywords.dedup();
        insight.keywords.truncate(8);
        tags.extend(operational_context.tags());
        tags.sort();
        tags.dedup();
    }
    let semantic_text = semantic::semantic_text(
        &analysis_content,
        content_type,
        language,
        &insight.summary,
        &insight.category,
        &insight.keywords,
        &tags,
    );
    let embedding = semantic::embed(&semantic_text).await;

    if content_type == "Code" {
        tags.push("Code".to_string());
    }

    NewClipboardItem {
        content: content.to_string(),
        content_type: content_type.to_string(),
        source_application: "Unknown".to_string(),
        content_hash: hash.to_string(),
        character_count: content.chars().count() as i64,
        word_count: content.split_whitespace().count() as i64,
        file_size: file_size_for_text(content),
        image_width: None,
        image_height: None,
        language: language.map(ToString::to_string),
        ai_summary: insight.summary,
        category: insight.category,
        keywords: insight.keywords,
        reading_time_minutes: insight.reading_time_minutes,
        semantic_text,
        embedding: embedding.values,
        embedding_source: embedding.source,
        operational_context,
        tags,
    }
}

pub async fn browser_bookmark_item(request: &BrowserBookmarkRequest) -> NewClipboardItem {
    let url = request.url.trim();
    let mut item = analyze_text(url, &hash_bytes(url.as_bytes())).await;
    let title = request.title.trim();
    item.source_application = "Browser bookmark".to_string();
    item.category = "Research".to_string();
    item.tags
        .extend(["Browser".to_string(), "Bookmark".to_string()]);
    item.tags.extend(
        request
            .tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
            .map(ToString::to_string),
    );
    if !title.is_empty() {
        item.keywords.push(title.to_string());
    }
    item.keywords.sort();
    item.keywords.dedup();
    item.keywords.truncate(8);
    item.tags.sort();
    item.tags.dedup();
    item.tags.truncate(12);
    item.ai_summary = if title.is_empty() {
        "Saved browser bookmark for local retrieval.".to_string()
    } else {
        format!("Saved browser bookmark: {title}")
    };
    item.semantic_text = semantic::semantic_text(
        &format!("{title}\n{url}"),
        "URL",
        None,
        &item.ai_summary,
        &item.category,
        &item.keywords,
        &item.tags,
    );
    let embedding = semantic::embed(&item.semantic_text).await;
    item.embedding = embedding.values;
    item.embedding_source = embedding.source;
    item
}

pub async fn ide_snippet_item(request: &IdeSnippetRequest) -> NewClipboardItem {
    let content = request.content.trim();
    let project = request.project.trim();
    let file_path = request.file_path.trim();
    // Project and path are intentionally part of the identity: the same snippet can mean
    // different things in different codebases while repeated captures in one place still dedupe.
    let content_hash =
        hash_bytes(format!("IDE snippet\n{project}\n{file_path}\n{content}").as_bytes());
    let mut item = analyze_text(content, &content_hash).await;
    let title = request.title.trim();
    let selected_language = request.language.trim();

    item.content_type = "Code".to_string();
    item.source_application = "IDE snippet".to_string();
    item.category = "Development".to_string();
    if selected_language != "Auto" {
        item.language = Some(selected_language.to_string());
    }
    item.tags.extend(["IDE".to_string(), "Code".to_string()]);
    item.tags.extend(
        request
            .tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
            .map(ToString::to_string),
    );
    item.keywords.extend(
        [title, project, file_path]
            .into_iter()
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
    );
    item.keywords.sort();
    item.keywords.dedup();
    item.keywords.truncate(8);
    item.tags.sort();
    item.tags.dedup();
    item.tags.truncate(12);
    item.ai_summary = match (title.is_empty(), project.is_empty()) {
        (false, _) => format!("Saved IDE snippet: {title}"),
        (true, false) => format!("Saved IDE snippet from project: {project}"),
        (true, true) => "Saved IDE snippet for local retrieval.".to_string(),
    };
    let semantic_content = format!("{title}\n{project}\n{file_path}\n{content}");
    item.semantic_text = semantic::semantic_text(
        &semantic_content,
        "Code",
        item.language.as_deref(),
        &item.ai_summary,
        &item.category,
        &item.keywords,
        &item.tags,
    );
    let embedding = semantic::embed(&item.semantic_text).await;
    item.embedding = embedding.values;
    item.embedding_source = embedding.source;
    item
}

pub fn terminal_history_item(command: &str, shell: &str) -> NewClipboardItem {
    let operational_context = operations::analyze(command, Some(shell));
    let mut keywords = operational_context.services.clone();
    keywords.extend(operational_context.technologies.clone());
    keywords.push(shell.to_string());
    keywords.sort();
    keywords.dedup();
    keywords.truncate(8);
    let tags = operational_context.tags();
    let summary = operational_context.summary();
    let semantic_text = semantic::semantic_text(
        command,
        "Code",
        Some("Bash"),
        &summary,
        "Operations",
        &keywords,
        &tags,
    );

    NewClipboardItem {
        content: command.to_string(),
        content_type: "Code".to_string(),
        source_application: format!("Terminal History ({shell})"),
        content_hash: hash_bytes(command.as_bytes()),
        character_count: command.chars().count() as i64,
        word_count: command.split_whitespace().count() as i64,
        file_size: None,
        image_width: None,
        image_height: None,
        language: Some("Bash".to_string()),
        ai_summary: summary,
        category: "Operations".to_string(),
        keywords,
        reading_time_minutes: 1,
        semantic_text: semantic_text.clone(),
        embedding: semantic::local_embedding(&semantic_text),
        embedding_source: "Local".to_string(),
        operational_context,
        tags,
    }
}

pub fn terminal_command_item(request: &TerminalCommandRequest) -> NewClipboardItem {
    let command = request.command.trim();
    let shell = request.shell.trim();
    let host = request.host.trim();
    let project = request.project.trim();
    // A repeated command is deduplicated only within the same deliberate operational context.
    let content_hash =
        hash_bytes(format!("Terminal command\n{shell}\n{host}\n{project}\n{command}").as_bytes());
    let mut item = terminal_history_item(command, shell);
    item.content_hash = content_hash;
    item.source_application = "Terminal command".to_string();
    item.language = Some(shell.to_string());

    if !host.is_empty() {
        if host.parse::<IpAddr>().is_ok() {
            item.operational_context.ip_addresses.push(host.to_string());
        } else {
            item.operational_context.hostnames.push(host.to_string());
        }
    }
    item.operational_context.hostnames.sort();
    item.operational_context.hostnames.dedup();
    item.operational_context.ip_addresses.sort();
    item.operational_context.ip_addresses.dedup();
    item.tags.extend(item.operational_context.tags());
    item.tags.push("Terminal Capture".to_string());
    item.tags.extend(
        request
            .tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
            .map(ToString::to_string),
    );
    item.keywords.extend(
        [project, host]
            .into_iter()
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
    );
    item.keywords.sort();
    item.keywords.dedup();
    item.keywords.truncate(8);
    item.tags.sort();
    item.tags.dedup();
    item.tags.truncate(12);
    if !host.is_empty() {
        item.ai_summary = format!("{} Target: {host}.", item.ai_summary);
    }
    let semantic_content = format!("{project}\n{host}\n{command}");
    item.semantic_text = semantic::semantic_text(
        &semantic_content,
        "Code",
        item.language.as_deref(),
        &item.ai_summary,
        &item.category,
        &item.keywords,
        &item.tags,
    );
    item.embedding = semantic::local_embedding(&item.semantic_text);
    item.embedding_source = "Local".to_string();
    item
}

pub fn classify_content(content: &str) -> &'static str {
    let trimmed = content.trim();
    let lower = trimmed.to_lowercase();

    if is_url(&lower) {
        return "URL";
    }

    if is_color_code(trimmed) {
        return "Color";
    }

    if looks_like_file_or_folder(trimmed) {
        return if Path::new(trimmed).is_dir() {
            "Folder"
        } else {
            "File"
        };
    }

    if looks_like_html(trimmed) {
        return "HTML";
    }

    if looks_like_table(trimmed) {
        return "Table";
    }

    if detect_language(trimmed).is_some() {
        return "Code";
    }

    "Text"
}

pub fn detect_language(content: &str) -> Option<&'static str> {
    let trimmed = content.trim();
    let lower = trimmed.to_lowercase();
    let upper = trimmed.to_uppercase();

    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return Some("JSON");
    }
    if upper.starts_with("SELECT ")
        || upper.contains("CREATE TABLE")
        || upper.contains("INSERT INTO")
    {
        return Some("SQL");
    }
    if lower.contains("fn main") || lower.contains("let mut ") || lower.contains("use std::") {
        return Some("Rust");
    }
    if lower.contains("def ") || lower.contains("import ") && lower.contains("python") {
        return Some("Python");
    }
    if lower.contains("interface ") || lower.contains(": string") || lower.contains("type ") {
        return Some("TypeScript");
    }
    if lower.contains("function ") || lower.contains("const ") || lower.contains("=>") {
        return Some("JavaScript");
    }
    if lower.contains("public class ") || lower.contains("system.out.println") {
        return Some("Java");
    }
    if lower.contains("#include") || lower.contains("std::cout") {
        return Some("C/C++");
    }
    if looks_like_html(trimmed) {
        return Some("HTML");
    }
    if lower.contains("{") && lower.contains(":") && lower.contains(";") {
        return Some("CSS");
    }

    None
}

fn auto_tags(content: &str, content_type: &str, language: Option<&str>) -> Vec<String> {
    let mut tags = Vec::new();
    let lower = content.to_lowercase();

    tags.push(content_type.to_string());

    if language == Some("JSON") {
        tags.push("JSON".to_string());
    }
    if lower.contains('@') && lower.contains('.') {
        tags.push("Email".to_string());
    }
    if lower.contains(".pdf") {
        tags.push("PDF".to_string());
    }
    if lower.contains("# ") || lower.contains("```") {
        tags.push("Markdown".to_string());
    }
    if lower.chars().filter(|ch| ch.is_ascii_digit()).count() >= 10 {
        tags.push("Phone Number".to_string());
    }

    tags.sort();
    tags.dedup();
    tags
}

fn is_url(lower: &str) -> bool {
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("www.")
}

fn is_color_code(content: &str) -> bool {
    let value = content.trim();
    let hex = value.strip_prefix('#').unwrap_or(value);
    matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn looks_like_html(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("<html")
        || lower.contains("<div")
        || lower.contains("<span")
        || lower.contains("<table")
        || lower.contains("</")
}

fn looks_like_table(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    lines.len() > 1
        && lines
            .iter()
            .filter(|line| line.contains('\t') || line.matches('|').count() >= 2)
            .count()
            >= 2
}

fn looks_like_file_or_folder(content: &str) -> bool {
    let path = Path::new(content);
    path.exists() && (path.is_file() || path.is_dir())
}

fn file_size_for_text(content: &str) -> Option<i64> {
    let path = Path::new(content.trim());
    if path.is_file() {
        fs::metadata(path)
            .ok()
            .map(|metadata| metadata.len() as i64)
    } else {
        None
    }
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::{classify_content, detect_language, hash_bytes};

    #[test]
    fn classifies_url() {
        assert_eq!(classify_content("https://github.com"), "URL");
    }

    #[test]
    fn classifies_code() {
        assert_eq!(classify_content("fn main() {\n}"), "Code");
        assert_eq!(detect_language("fn main() {\n}"), Some("Rust"));
    }

    #[test]
    fn classifies_text() {
        assert_eq!(classify_content("Hello CYMOS"), "Text");
    }

    #[test]
    fn classifies_color() {
        assert_eq!(classify_content("#0f766e"), "Color");
    }

    #[test]
    fn hashes_content() {
        assert_eq!(hash_bytes(b"Hello CYMOS").len(), 64);
    }
}
