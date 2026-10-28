use arboard::Clipboard;
use tauri::{AppHandle, Emitter};
use sqlx::SqlitePool;
use std::time::Duration;
use tokio::sync::mpsc;
use sha2::{Sha256, Digest};
use base64::Engine;

// ─── Content Type Detection ───────────────────────────────────────────────────

pub fn detect_content_type(content: &str) -> &'static str {
    let trimmed = content.trim();

    // Email
    if is_email(trimmed) { return "Email"; }

    // Phone number (simple heuristic)
    if is_phone(trimmed) { return "Phone"; }

    // URL
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("www.")
    {
        return "URL";
    }

    // Color code
    if is_color_code(trimmed) { return "Color"; }

    // JSON
    if is_json(trimmed) { return "JSON"; }

    // HTML
    if is_html(trimmed) { return "HTML"; }

    // Code (broad heuristic)
    if is_code(trimmed) { return "Code"; }

    "Text"
}

fn is_email(s: &str) -> bool {
    let parts: Vec<&str> = s.splitn(2, '@').collect();
    if parts.len() != 2 { return false; }
    let domain = parts[1];
    domain.contains('.') && !s.contains(' ') && s.len() < 254
}

fn is_phone(s: &str) -> bool {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    let cleaned = s.trim();
    // Must be primarily digits/separators, 7-15 digits total
    digits.len() >= 7 && digits.len() <= 15
        && cleaned.len() <= 20
        && cleaned.chars().all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | ' ' | '(' | ')'))
}

fn is_color_code(s: &str) -> bool {
    // Hex color
    if s.starts_with('#') && (s.len() == 4 || s.len() == 7 || s.len() == 9) {
        return s[1..].chars().all(|c| c.is_ascii_hexdigit());
    }
    // rgb/rgba/hsl/hsla
    let lower = s.to_lowercase();
    (lower.starts_with("rgb(") || lower.starts_with("rgba(")
        || lower.starts_with("hsl(") || lower.starts_with("hsla("))
        && lower.ends_with(')')
}

fn is_json(s: &str) -> bool {
    let trimmed = s.trim();
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
}

fn is_html(s: &str) -> bool {
    let lower = s.to_lowercase();
    (lower.contains("<html") || lower.contains("<!doctype html")
        || lower.contains("</div>") || lower.contains("</p>")
        || lower.contains("</span>") || lower.contains("</a>"))
        && lower.contains('<') && lower.contains('>')
}

fn is_code(s: &str) -> bool {
    let has_newlines = s.contains('\n');
    let has_brackets = s.contains('{') || s.contains('}');
    let has_semicolon = s.contains(';');

    // Strong code indicators
    let strong: &[&str] = &[
        "fn ", "pub fn ", "impl ", "struct ", "enum ", "trait ",  // Rust
        "def ", "import ", "from ", "class ", "if __name__",       // Python
        "function ", "const ", "let ", "var ", "=>", "async ",     // JS/TS
        "interface ", "type ", "namespace ",                         // TS
        "public static", "private ", "void ", "return ",            // Java/C
        "SELECT ", "FROM ", "WHERE ", "INSERT INTO",                 // SQL
        "#include", "#define", "printf(",                            // C
        "package ", "import java",                                   // Java
        "<?php", "echo ",                                            // PHP
    ];

    for indicator in strong {
        if s.contains(indicator) { return true; }
    }

    // Heuristic: newlines + code structure
    has_newlines && (has_brackets || has_semicolon)
}

// ─── Language Detection ───────────────────────────────────────────────────────

pub fn detect_language(content: &str) -> Option<&'static str> {
    let s = content.trim();

    // JSON — check before others
    if (s.starts_with('{') || s.starts_with('[')) && serde_json::from_str::<serde_json::Value>(s).is_ok() {
        return Some("JSON");
    }

    // SQL
    let upper = s.to_uppercase();
    if upper.contains("SELECT ") && (upper.contains(" FROM ") || upper.contains("FROM\n")) {
        return Some("SQL");
    }
    if upper.starts_with("INSERT INTO") || upper.starts_with("CREATE TABLE") || upper.starts_with("UPDATE ") {
        return Some("SQL");
    }

    // HTML/CSS
    let lower = s.to_lowercase();
    if lower.contains("<html") || lower.contains("<!doctype") || (lower.contains("</") && lower.contains("<div")) {
        return Some("HTML");
    }
    if (lower.contains('{') && lower.contains(':') && lower.contains(';'))
        && (lower.contains("px") || lower.contains("em") || lower.contains("color") || lower.contains("margin"))
    {
        return Some("CSS");
    }

    // Rust
    if s.contains("fn ") && s.contains("->") && (s.contains("let ") || s.contains("pub ")) {
        return Some("Rust");
    }
    if s.contains("println!") || s.contains("eprintln!") || s.contains("vec![") || s.contains("impl ") {
        return Some("Rust");
    }

    // Python
    if s.contains("def ") && s.contains(':') && !s.contains(';') {
        return Some("Python");
    }
    if s.contains("import ") && !s.contains("from '") && !s.contains("from \"") && s.contains('\n') {
        if s.contains("import numpy") || s.contains("import pandas") || s.contains("import os") {
            return Some("Python");
        }
    }
    if s.contains("if __name__") || s.contains("print(") && !s.contains('{') {
        return Some("Python");
    }

    // TypeScript (check before JS because TS is a superset)
    if s.contains("interface ") || s.contains(": string") || s.contains(": number")
        || s.contains(": boolean") || s.contains("readonly ") || s.contains("<T>")
        || s.contains("type ") && s.contains("=") && s.contains("|")
    {
        return Some("TypeScript");
    }

    // JavaScript
    if s.contains("function ") || s.contains("const ") || s.contains("let ")
        || s.contains("var ") || s.contains("=>") || s.contains("require(")
        || s.contains("module.exports") || s.contains("console.log(")
    {
        return Some("JavaScript");
    }

    // Java
    if s.contains("public class ") || s.contains("public static void main")
        || s.contains("import java.") || s.contains("System.out.println")
    {
        return Some("Java");
    }

    // C/C++
    if s.contains("#include") || s.contains("printf(") || s.contains("scanf(")
        || s.contains("std::") || s.contains("cout <<")
    {
        if s.contains("std::") || s.contains("cout") || s.contains("cin") {
            return Some("C++");
        }
        return Some("C");
    }

    // Go
    if s.contains("package main") || s.contains("func ") && s.contains("go ") {
        return Some("Go");
    }

    // Shell/Bash
    if s.starts_with("#!/") || s.contains("echo ") && s.contains('$')
        || s.contains("sudo ") || s.contains("grep ") || s.contains("| awk")
    {
        return Some("Shell");
    }

    // Markdown
    if s.contains("# ") && (s.contains("## ") || s.contains("- ") || s.contains("**")) {
        return Some("Markdown");
    }

    None
}

// ─── Metadata Computation ────────────────────────────────────────────────────

pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn count_words(content: &str) -> i64 {
    content.split_whitespace().count() as i64
}

// ─── Source App Detection (macOS) ────────────────────────────────────────────

pub fn get_source_app() -> Option<String> {
    // Use AppleScript to get the frontmost application name
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first application process whose frontmost is true")
        .output()
        .ok()?;

    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() { Some(name) } else { None }
    } else {
        None
    }
}

// ─── Clipboard Monitor ────────────────────────────────────────────────────────

pub fn start_monitoring(
    app: AppHandle, 
    pool: SqlitePool, 
    ai_tx: mpsc::Sender<i64>, 
    embed_tx: mpsc::Sender<i64>, 
    graph_tx: mpsc::Sender<i64>
) {
    tauri::async_runtime::spawn(async move {
        let mut clipboard = match Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                eprintln!("[CYMOS] Failed to initialize clipboard: {}", e);
                return;
            }
        };

        // Seed last_hash from the most recent DB item to avoid re-inserting on startup
        let mut last_hash: String = sqlx::query_scalar::<_, String>(
            "SELECT COALESCE(content_hash, '') FROM clipboard_items ORDER BY id DESC LIMIT 1"
        )
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

        let mut last_image_hash = String::new();

        loop {
            tokio::time::sleep(Duration::from_millis(600)).await;

            // ── Try image first ──
            if let Ok(img) = clipboard.get_image() {
                // Quick hash the dimensions + first few bytes as a fingerprint
                let img_fingerprint = format!("{}x{}:{}", img.width, img.height, img.bytes.len());
                let img_hash = compute_hash(&img_fingerprint);

                if img_hash != last_image_hash {
                    last_image_hash = img_hash.clone();

                    // Encode PNG to base64
                    let png_b64 = encode_image_to_base64_png(&img);
                    if let Some(b64) = png_b64 {
                        let content_hash = compute_hash(&b64);
                        if content_hash != last_hash {
                            last_hash = content_hash.clone();

                            let source_app = tokio::task::spawn_blocking(get_source_app)
                                .await
                                .ok()
                                .flatten();

                            let new_item = crate::database::NewClipboardItem {
                                content: format!("[Image {}×{}]", img.width, img.height),
                                content_type: "Image".to_string(),
                                content_hash,
                                source_app,
                                char_count: 0,
                                word_count: 0,
                                file_size: b64.len() as i64,
                                image_width: img.width as i64,
                                image_height: img.height as i64,
                                language: None,
                                image_data: Some(b64),
                            };

                            match crate::database::insert_item(&pool, new_item).await {
                                Ok(item) => {
                                    let item_id = item.id;
                                    let _ = app.emit("clipboard-updated", item);
                                    let _ = ai_tx.try_send(item_id);
                                    let _ = embed_tx.try_send(item_id);
                                    let _ = graph_tx.try_send(item_id);
                                }
                                Err(e) => eprintln!("[CYMOS] Image insert error: {}", e),
                            }
                        }
                    }
                }
                // Skip text check this round if we found an image
                continue;
            }

            // ── Try text ──
            if let Ok(current_text) = clipboard.get_text() {
                let trimmed = current_text.trim();
                if trimmed.is_empty() { continue; }

                let hash = compute_hash(trimmed);
                if hash == last_hash { continue; }
                last_hash = hash.clone();

                let content_type = detect_content_type(trimmed);
                let language = if content_type == "Code" || content_type == "JSON" || content_type == "HTML" {
                    detect_language(trimmed)
                } else {
                    None
                };

                let char_count = trimmed.chars().count() as i64;
                let word_count = count_words(trimmed);
                let file_size = trimmed.len() as i64;

                let source_app = tokio::task::spawn_blocking(get_source_app)
                    .await
                    .ok()
                    .flatten();

                let new_item = crate::database::NewClipboardItem {
                    content: trimmed.to_string(),
                    content_type: content_type.to_string(),
                    content_hash: hash,
                    source_app,
                    char_count,
                    word_count,
                    file_size,
                    image_width: 0,
                    image_height: 0,
                    language: language.map(|l| l.to_string()),
                    image_data: None,
                };

                match crate::database::insert_item(&pool, new_item).await {
                    Ok(item) => {
                        let item_id = item.id;
                        let _ = app.emit("clipboard-updated", item);
                        let _ = ai_tx.try_send(item_id);
                        let _ = embed_tx.try_send(item_id);
                        let _ = graph_tx.try_send(item_id);
                    }
                    Err(e) => eprintln!("[CYMOS] Insert error: {}", e),
                }
            }
        }
    });
}

// ─── Image Encoding ───────────────────────────────────────────────────────────

fn encode_image_to_base64_png(img: &arboard::ImageData) -> Option<String> {
    // Encode RGBA bytes as PNG using a minimal pure-Rust approach
    // We build a raw PPM-like structure and encode it. For simplicity,
    // we encode as raw RGBA base64 (the frontend decodes using a canvas).
    // Max image size cap: 8MB raw data
    if img.bytes.len() > 8 * 1024 * 1024 {
        return None;
    }
    let b64 = base64::engine::general_purpose::STANDARD.encode(&img.bytes[..]);
    Some(format!("rgba:{}:{}:{}", img.width, img.height, b64))
}
