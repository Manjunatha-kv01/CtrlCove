use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

pub mod clipboard;
pub mod database;
pub mod ollama;
pub mod embeddings;
pub mod graph;
pub mod agents;
pub mod scheduler;
pub mod api;

struct DbState(sqlx::SqlitePool);
struct AiConfigState(Arc<Mutex<ollama::OllamaConfig>>);
struct EmbeddingConfigState(Arc<Mutex<embeddings::EmbeddingConfig>>);
struct SchedulerState(Arc<Mutex<bool>>);

// ─── Clipboard Commands ───────────────────────────────────────────────────────

#[tauri::command]
async fn get_clipboard_items(
    state: tauri::State<'_, DbState>,
    limit: i64,
) -> Result<Vec<database::ClipboardItem>, String> {
    database::get_items(&state.0, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_clipboard_items_filtered(
    state: tauri::State<'_, DbState>,
    opts: database::FilterOptions,
) -> Result<Vec<database::ClipboardItem>, String> {
    database::get_items_filtered(&state.0, opts)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_clipboard_items(
    state: tauri::State<'_, DbState>,
    query: String,
) -> Result<Vec<database::ClipboardItem>, String> {
    database::search_items(&state.0, &query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_clipboard_item(
    state: tauri::State<'_, DbState>,
    id: i64,
) -> Result<(), String> {
    database::delete_item(&state.0, id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_clipboard_history(state: tauri::State<'_, DbState>) -> Result<(), String> {
    database::clear_history(&state.0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn write_to_clipboard(content: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| e.to_string())?;
    clipboard.set_text(content)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn toggle_favorite(
    state: tauri::State<'_, DbState>,
    id: i64,
) -> Result<bool, String> {
    database::toggle_favorite(&state.0, id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn move_to_collection(
    state: tauri::State<'_, DbState>,
    clipboard_id: i64,
    collection_id: Option<i64>,
) -> Result<(), String> {
    database::move_to_collection(&state.0, clipboard_id, collection_id)
        .await
        .map_err(|e| e.to_string())
}

// ─── Collection Commands ──────────────────────────────────────────────────────

#[tauri::command]
async fn get_collections(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<database::Collection>, String> {
    database::get_collections(&state.0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_collection(
    state: tauri::State<'_, DbState>,
    name: String,
    color: String,
) -> Result<database::Collection, String> {
    database::create_collection(&state.0, &name, &color)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_collection(
    state: tauri::State<'_, DbState>,
    id: i64,
    name: String,
    color: String,
) -> Result<(), String> {
    database::update_collection(&state.0, id, &name, &color)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_collection(
    state: tauri::State<'_, DbState>,
    id: i64,
) -> Result<(), String> {
    database::delete_collection(&state.0, id)
        .await
        .map_err(|e| e.to_string())
}

// ─── Tag Commands ─────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_tags(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<database::Tag>, String> {
    database::get_tags(&state.0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_tag(
    state: tauri::State<'_, DbState>,
    name: String,
) -> Result<database::Tag, String> {
    database::create_tag(&state.0, &name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_tag(
    state: tauri::State<'_, DbState>,
    id: i64,
) -> Result<(), String> {
    database::delete_tag(&state.0, id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_tag_to_item(
    state: tauri::State<'_, DbState>,
    clipboard_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    database::add_tag_to_item(&state.0, clipboard_id, tag_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_tag_from_item(
    state: tauri::State<'_, DbState>,
    clipboard_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    database::remove_tag_from_item(&state.0, clipboard_id, tag_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_tags_for_item(
    state: tauri::State<'_, DbState>,
    clipboard_id: i64,
) -> Result<Vec<database::Tag>, String> {
    database::get_tags_for_item(&state.0, clipboard_id)
        .await
        .map_err(|e| e.to_string())
}

// ─── Statistics & Export Commands ────────────────────────────────────────────

#[tauri::command]
async fn get_statistics(
    state: tauri::State<'_, DbState>,
) -> Result<database::Statistics, String> {
    database::get_statistics(&state.0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_items_json(
    state: tauri::State<'_, DbState>,
) -> Result<String, String> {
    database::export_items_json(&state.0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_items_csv(
    state: tauri::State<'_, DbState>,
) -> Result<String, String> {
    database::export_items_csv(&state.0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_items_markdown(
    state: tauri::State<'_, DbState>,
) -> Result<String, String> {
    database::export_items_markdown(&state.0)
        .await
        .map_err(|e| e.to_string())
}

// ─── AI Commands ─────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_ollama_status(
    ai_config: tauri::State<'_, AiConfigState>,
) -> Result<ollama::OllamaStatus, String> {
    let config = ai_config.0.lock().await.clone();
    Ok(ollama::get_status(&config).await)
}

#[tauri::command]
async fn get_ai_config(
    ai_config: tauri::State<'_, AiConfigState>,
) -> Result<ollama::OllamaConfig, String> {
    Ok(ai_config.0.lock().await.clone())
}

#[tauri::command]
async fn set_ai_config(
    ai_config: tauri::State<'_, AiConfigState>,
    model: String,
    enabled: bool,
    base_url: String,
) -> Result<(), String> {
    let mut config = ai_config.0.lock().await;
    config.model = model;
    config.enabled = enabled;
    config.base_url = base_url;
    Ok(())
}

#[tauri::command]
async fn get_memory_insights(
    state: tauri::State<'_, DbState>,
) -> Result<database::MemoryInsights, String> {
    database::get_memory_insights(&state.0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn process_item_with_ai(
    state: tauri::State<'_, DbState>,
    ai_config: tauri::State<'_, AiConfigState>,
    app: tauri::AppHandle,
    id: i64,
) -> Result<(), String> {
    let config = ai_config.0.lock().await.clone();
    if !config.enabled { return Ok(()); }

    let item = database::get_item_by_id(&state.0, id)
        .await
        .map_err(|e| e.to_string())?;

    match ollama::enrich_item(
        &item.content,
        &item.content_type,
        item.language.as_deref(),
        &config,
    ).await {
        Ok(result) => {
            database::update_ai_fields(&state.0, id, &result, &config.model)
                .await
                .map_err(|e| e.to_string())?;
            // Emit updated item to frontend
            if let Ok(updated) = database::get_item_by_id(&state.0, id).await {
                let _ = app.emit("ai-enriched", updated);
            }
        }
        Err(e) => eprintln!("[CYMOS AI] Enrichment failed for item {}: {}", id, e),
    }
    Ok(())
}

#[tauri::command]
async fn process_all_with_ai(
    state: tauri::State<'_, DbState>,
    ai_config: tauri::State<'_, AiConfigState>,
    app: tauri::AppHandle,
) -> Result<i64, String> {
    let config = ai_config.0.lock().await.clone();
    if !config.enabled {
        return Ok(0);
    }

    let items = database::get_unprocessed_items(&state.0)
        .await
        .map_err(|e| e.to_string())?;

    let count = items.len() as i64;
    let pool = state.0.clone();
    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        for item in items {
            match ollama::enrich_item(
                &item.content,
                &item.content_type,
                item.language.as_deref(),
                &config,
            ).await {
                Ok(result) => {
                    if let Err(e) = database::update_ai_fields(&pool, item.id, &result, &config.model).await {
                        eprintln!("[CYMOS AI] DB update failed for {}: {}", item.id, e);
                    } else if let Ok(updated) = database::get_item_by_id(&pool, item.id).await {
                        let _ = app_handle.emit("ai-enriched", updated);
                    }
                }
                Err(e) => eprintln!("[CYMOS AI] Failed item {}: {}", item.id, e),
            }
            // Small delay between requests to be kind to the GPU
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        eprintln!("[CYMOS AI] Batch processing complete");
    });

    Ok(count)
}

// ─── Background AI Queue Worker ───────────────────────────────────────────────

fn start_ai_queue_worker(
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    config: Arc<Mutex<ollama::OllamaConfig>>,
    mut rx: mpsc::Receiver<i64>,
) {
    tauri::async_runtime::spawn(async move {
        while let Some(item_id) = rx.recv().await {
            let cfg = config.lock().await.clone();
            if !cfg.enabled { continue; }

            // Fetch item
            let item = match database::get_item_by_id(&pool, item_id).await {
                Ok(i) => i,
                Err(e) => { eprintln!("[CYMOS AI] Fetch item {} failed: {}", item_id, e); continue; }
            };

            // Skip short content and images
            if item.content.len() < cfg.min_chars || item.content_type == "Image" {
                continue;
            }

            // Skip if already processed (e.g. duplicate was found, original already enriched)
            if item.ai_processed == Some(1) { continue; }

            match ollama::enrich_item(
                &item.content,
                &item.content_type,
                item.language.as_deref(),
                &cfg,
            ).await {
                Ok(result) => {
                    if let Err(e) = database::update_ai_fields(&pool, item_id, &result, &cfg.model).await {
                        eprintln!("[CYMOS AI] DB write failed for {}: {}", item_id, e);
                    } else {
                        eprintln!("[CYMOS AI] ✓ Item {} → {} / {}", item_id, result.category, result.summary.chars().take(60).collect::<String>());
                        if let Ok(updated) = database::get_item_by_id(&pool, item_id).await {
                            let _ = app.emit("ai-enriched", updated);
                        }
                    }
                }
                Err(e) => eprintln!("[CYMOS AI] Enrichment failed for item {}: {}", item_id, e),
            }
        }
    });
}

// ─── Embedding Queue Worker ──────────────────────────────────────────────────

fn start_embedding_worker(
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    embed_config: Arc<Mutex<embeddings::EmbeddingConfig>>,
    mut embed_rx: mpsc::Receiver<i64>,
) {
    tauri::async_runtime::spawn(async move {
        while let Some(item_id) = embed_rx.recv().await {
            let cfg = embed_config.lock().await.clone();
            if !cfg.enabled {
                continue;
            }

            // Fetch item content
            let item = match database::get_item_by_id(&pool, item_id).await {
                Ok(i) => i,
                Err(_) => continue,
            };

            // Skip images and very short content
            if item.content_type == "Image" || item.content.len() < cfg.min_chars {
                continue;
            }

            // Skip already embedded
            if item.embedding_status == Some(1) {
                continue;
            }

            // Build text to embed: include AI context if available for richer vectors
            let text_to_embed = if let Some(ref summary) = item.ai_summary {
                format!("{} {}", item.content.chars().take(2000).collect::<String>(), summary)
            } else {
                item.content.chars().take(4096).collect::<String>()
            };

            match embeddings::generate_embedding(&cfg, &text_to_embed).await {
                Ok(vec) => {
                    let json = embeddings::encode_vec(&vec);
                    if let Err(e) = database::update_embedding(&pool, item_id, &json).await {
                        eprintln!("[CYMOS EMBED] DB write failed for {}: {}", item_id, e);
                    } else {
                        eprintln!("[CYMOS EMBED] ✓ Item {} embedded ({} dims)", item_id, vec.len());
                        let _ = app.emit("embedding-ready", item_id);
                    }
                }
                Err(e) => eprintln!("[CYMOS EMBED] Failed for item {}: {}", item_id, e),
            }
        }
    });
}

// ─── Graph Worker ──────────────────────────────────────────────────────────────

fn start_graph_worker(
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    config: Arc<Mutex<ollama::OllamaConfig>>,
    mut graph_rx: mpsc::Receiver<i64>,
) {
    tauri::async_runtime::spawn(async move {
        while let Some(item_id) = graph_rx.recv().await {
            let cfg = config.lock().await.clone();
            if !cfg.enabled { continue; }

            // Fetch item
            let item = match database::get_item_by_id(&pool, item_id).await {
                Ok(i) => i,
                Err(_) => continue,
            };

            // Skip images and short text
            if item.content_type == "Image" || item.content.len() < cfg.min_chars {
                continue;
            }

            // Call extract_graph
            match ollama::extract_graph(&item.content, &cfg).await {
                Ok(extracted_data) => {
                    if let Err(e) = graph::ingest_graph_data(&pool, item_id, extracted_data).await {
                        eprintln!("[CYMOS Graph] Ingest failed for {}: {}", item_id, e);
                    } else {
                        eprintln!("[CYMOS Graph] ✓ Item {} extracted", item_id);
                        let _ = app.emit("graph-ready", ());
                    }
                }
                Err(e) => eprintln!("[CYMOS Graph] Extraction failed for {}: {}", item_id, e),
            }
        }
    });
}

// ─── v0.4 Semantic Search Commands ───────────────────────────────────────────

#[tauri::command]
async fn semantic_search(
    db_state: tauri::State<'_, DbState>,
    embed_state: tauri::State<'_, EmbeddingConfigState>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<database::ClipboardItem>, String> {
    let cfg = embed_state.0.lock().await.clone();
    let limit = limit.unwrap_or(20);

    // 1. Embed the query
    let query_vec = embeddings::generate_embedding(&cfg, &query)
        .await
        .map_err(|e| e.to_string())?;

    // 2. Fetch all embedded rows (id + vector)
    let rows = database::get_embedded_rows(&db_state.0)
        .await
        .map_err(|e| e.to_string())?;

    // 3. Build EmbeddedItem list for scoring
    let now = chrono::Utc::now();
    let items_for_scoring: Vec<embeddings::EmbeddedItem> = rows
        .into_iter()
        .filter_map(|row| {
            let vec = row.embedding.as_deref().and_then(embeddings::decode_vec)?;
            // Calculate age in days
            let created = chrono::NaiveDateTime::parse_from_str(
                &format!("{} +0000", row.created_at.trim()),
                "%Y-%m-%d %H:%M:%S %z",
            )
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(&row.created_at, "%Y-%m-%d %H:%M:%S"))
            .ok()?;
            let days_old = (now.naive_utc() - created).num_hours() as f32 / 24.0;

            Some(embeddings::EmbeddedItem {
                id: row.id,
                embedding: vec,
                is_favorite: row.is_favorite.unwrap_or(0) == 1,
                copy_count: row.copy_count.unwrap_or(1),
                days_old: days_old.max(0.0),
            })
        })
        .collect();

    // 4. Build a simple keyword set from query words
    let keyword_set: std::collections::HashSet<String> = query
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect();

    // 5. Score and rank
    let ranked = embeddings::rank_by_similarity(&items_for_scoring, &query_vec, &query, &keyword_set);

    // 6. Take top N IDs and fetch full items
    let top_ids: Vec<i64> = ranked.iter().take(limit as usize).map(|(id, _)| *id).collect();
    database::get_items_by_ids(&db_state.0, &top_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_similar_items(
    db_state: tauri::State<'_, DbState>,
    item_id: i64,
    limit: Option<i64>,
) -> Result<Vec<database::ClipboardItem>, String> {
    let limit = limit.unwrap_or(5);

    // Get the target item's embedding
    let rows = database::get_embedded_rows(&db_state.0)
        .await
        .map_err(|e| e.to_string())?;

    // Find the target vector
    let target_vec = rows
        .iter()
        .find(|r| r.id == item_id)
        .and_then(|r| r.embedding.as_deref().and_then(embeddings::decode_vec))
        .ok_or_else(|| "Item has no embedding yet".to_string())?;

    // Score all other items against the target vector
    let now = chrono::Utc::now();
    let others: Vec<embeddings::EmbeddedItem> = rows
        .into_iter()
        .filter(|r| r.id != item_id)
        .filter_map(|row| {
            let vec = row.embedding.as_deref().and_then(embeddings::decode_vec)?;
            let created = chrono::NaiveDateTime::parse_from_str(
                &format!("{} +0000", row.created_at.trim()),
                "%Y-%m-%d %H:%M:%S %z",
            )
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(&row.created_at, "%Y-%m-%d %H:%M:%S"))
            .ok()?;
            let days_old = (now.naive_utc() - created).num_hours() as f32 / 24.0;
            Some(embeddings::EmbeddedItem {
                id: row.id,
                embedding: vec,
                is_favorite: row.is_favorite.unwrap_or(0) == 1,
                copy_count: row.copy_count.unwrap_or(1),
                days_old: days_old.max(0.0),
            })
        })
        .collect();

    let empty = std::collections::HashSet::new();
    let ranked = embeddings::rank_by_similarity(&others, &target_vec, "", &empty);
    let top_ids: Vec<i64> = ranked.iter().take(limit as usize).map(|(id, _)| *id).collect();

    database::get_items_by_ids(&db_state.0, &top_ids)
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
struct EmbeddingStatusResponse {
    total_embedded: i64,
    pending: i64,
    model: String,
    model_available: bool,
}

#[tauri::command]
async fn get_embedding_status(
    db_state: tauri::State<'_, DbState>,
    embed_state: tauri::State<'_, EmbeddingConfigState>,
) -> Result<EmbeddingStatusResponse, String> {
    let cfg = embed_state.0.lock().await.clone();
    let (total_embedded, pending, model_available) = tokio::join!(
        database::count_embedded(&db_state.0),
        database::count_pending_embed(&db_state.0),
        embeddings::check_model_available(&cfg),
    );
    Ok(EmbeddingStatusResponse {
        total_embedded: total_embedded.unwrap_or(0),
        pending: pending.unwrap_or(0),
        model: cfg.model,
        model_available,
    })
}

// ─── Graph Commands ───────────────────────────────────────────────────────────

#[tauri::command]
async fn get_graph_data(state: tauri::State<'_, DbState>) -> Result<graph::GraphData, String> {
    graph::get_graph(&state.0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_item_graph(state: tauri::State<'_, DbState>, id: i64) -> Result<graph::GraphData, String> {
    graph::get_item_graph(&state.0, id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn extract_all_graphs(
    state: tauri::State<'_, DbState>,
    ai_config: tauri::State<'_, AiConfigState>,
    app: tauri::AppHandle,
) -> Result<i64, String> {
    let config = ai_config.0.lock().await.clone();
    if !config.enabled {
        return Err("AI is disabled".to_string());
    }

    let items = database::get_unextracted_items(&state.0)
        .await
        .map_err(|e| e.to_string())?;

    let count = items.len() as i64;
    let pool = state.0.clone();

    tauri::async_runtime::spawn(async move {
        for (id, content) in items {
            match ollama::extract_graph(&content, &config).await {
                Ok(extracted_data) => {
                    if let Err(e) = graph::ingest_graph_data(&pool, id, extracted_data).await {
                        eprintln!("[CYMOS Graph Backfill] Ingest failed for {}: {}", id, e);
                    } else {
                        let _ = app.emit("graph-ready", ());
                    }
                }
                Err(e) => eprintln!("[CYMOS Graph Backfill] Extraction failed for {}: {}", id, e),
            }
            // Small delay to be nice to GPU
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    });

    Ok(count)
}

// ─── Assistant Commands (v0.0.6) ──────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct ChatHistoryMessage {
    role: String,
    content: String,
}

#[derive(serde::Serialize)]
struct ChatResponse {
    response: String,
    sources: Vec<database::ClipboardItem>,
}

#[tauri::command]
async fn chat_with_memory(
    db_state: tauri::State<'_, DbState>,
    embed_state: tauri::State<'_, EmbeddingConfigState>,
    ai_config: tauri::State<'_, AiConfigState>,
    query: String,
    history: Vec<ChatHistoryMessage>,
) -> Result<ChatResponse, String> {
    let embed_cfg = embed_state.0.lock().await.clone();
    let ai_cfg = ai_config.0.lock().await.clone();

    if !ai_cfg.enabled {
        return Err("AI Memory Assistant is disabled in settings.".to_string());
    }

    // 1. Retrieve top 5 similar items for context
    let mut sources = Vec::new();
    if let Ok(query_vec) = embeddings::generate_embedding(&embed_cfg, &query).await {
        if let Ok(rows) = database::get_embedded_rows(&db_state.0).await {
            let now = chrono::Utc::now();
            let items_for_scoring: Vec<embeddings::EmbeddedItem> = rows
                .into_iter()
                .filter_map(|row| {
                    let vec = row.embedding.as_deref().and_then(embeddings::decode_vec)?;
                    let created = chrono::NaiveDateTime::parse_from_str(
                        &format!("{} +0000", row.created_at.trim()),
                        "%Y-%m-%d %H:%M:%S %z",
                    )
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(&row.created_at, "%Y-%m-%d %H:%M:%S"))
                    .ok()?;
                    let days_old = (now.naive_utc() - created).num_hours() as f32 / 24.0;
                    Some(embeddings::EmbeddedItem {
                        id: row.id,
                        embedding: vec,
                        is_favorite: row.is_favorite.unwrap_or(0) == 1,
                        copy_count: row.copy_count.unwrap_or(1),
                        days_old: days_old.max(0.0),
                    })
                })
                .collect();

            let keyword_set: std::collections::HashSet<String> = query
                .split_whitespace()
                .map(|w| w.to_lowercase())
                .collect();

            let ranked = embeddings::rank_by_similarity(&items_for_scoring, &query_vec, &query, &keyword_set);
            let top_ids: Vec<i64> = ranked.iter().take(5).map(|(id, _)| *id).collect();
            if let Ok(items) = database::get_items_by_ids(&db_state.0, &top_ids).await {
                sources = items;
            }
        }
    }

    // 2. Build the context block
    let mut context_block = String::new();
    if !sources.is_empty() {
        context_block.push_str("Here is the relevant clipboard context retrieved from the user's memory:\n\n");
        for (idx, item) in sources.iter().enumerate() {
            let category_hint = item.ai_category.as_deref().unwrap_or("General");
            context_block.push_str(&format!(
                "[Source #{}] Type: {} | Category: {} | Copied: {}\nContent:\n\"\"\"\n{}\n\"\"\"\n\n",
                idx + 1,
                item.content_type,
                category_hint,
                item.created_at,
                item.content.chars().take(1000).collect::<String>()
            ));
        }
    } else {
        context_block.push_str("No direct clipboard context could be found for this query.\n");
    }

    // 3. Format history
    let mut history_block = String::new();
    if !history.is_empty() {
        history_block.push_str("Previous Conversation History:\n");
        for msg in &history {
            let role_prefix = if msg.role == "user" { "User" } else { "Assistant" };
            history_block.push_str(&format!("{}: {}\n", role_prefix, msg.content));
        }
        history_block.push_str("\n");
    }

    // 4. Construct the full prompt
    let prompt = format!(
        r#"You are CYMOS, the personal AI Memory Assistant for the user. Your answers MUST be grounded in the provided clipboard history context whenever possible.
If the context does not contain the answer, answer generally but state that the information wasn't explicitly found in their memory.

{context_block}
{history_block}Current User Query: {query}

Formulate your response now. Maintain an helpful, premium, and concise tone."#
    );

    // 5. Query Ollama
    let response = ollama::generate_text(&prompt, &ai_cfg)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ChatResponse { response, sources })
}

#[derive(serde::Serialize)]
struct MemorySummary {
    summary_text: String,
    total_copied: i64,
    top_category: String,
}

#[tauri::command]
async fn get_daily_summary(
    db_state: tauri::State<'_, DbState>,
    ai_config: tauri::State<'_, AiConfigState>,
) -> Result<MemorySummary, String> {
    let ai_cfg = ai_config.0.lock().await.clone();
    if !ai_cfg.enabled {
        return Ok(MemorySummary {
            summary_text: "AI is disabled. Enable it in settings to see daily summaries.".to_string(),
            total_copied: 0,
            top_category: "None".to_string(),
        });
    }

    let items = database::get_items(&db_state.0, 20)
        .await
        .map_err(|e| e.to_string())?;

    if items.is_empty() {
        return Ok(MemorySummary {
            summary_text: "You haven't copied anything today yet. Copy some code, links, or notes to build your digital memory!".to_string(),
            total_copied: 0,
            top_category: "None".to_string(),
        });
    }

    let mut categories = std::collections::HashMap::new();
    let mut item_summaries = Vec::new();
    for item in &items {
        if let Some(ref cat) = item.ai_category {
            *categories.entry(cat.clone()).or_insert(0) += 1;
        }
        if let Some(ref sum) = item.ai_summary {
            item_summaries.push(sum.clone());
        } else {
            item_summaries.push(item.content.chars().take(60).collect());
        }
    }

    let top_category = categories.into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(cat, _)| cat)
        .unwrap_or_else(|| "Other".to_string());

    let recaps = item_summaries.join("\n- ");
    let prompt = format!(
        "You are CYMOS, a personal memory analyzer. Review this list of items the user copied today and write a concise 2-sentence summary of what they did, learned, or focused on. Be friendly and direct. Avoid repeating exact text.\n\nItems copied today:\n- {recaps}"
    );

    let summary_text = ollama::generate_text(&prompt, &ai_cfg).await.unwrap_or_else(|_| "Failed to generate daily AI summary. Please check Ollama connection.".to_string());

    Ok(MemorySummary {
        summary_text,
        total_copied: items.len() as i64,
        top_category,
    })
}

#[tauri::command]
async fn get_weekly_report(
    db_state: tauri::State<'_, DbState>,
    ai_config: tauri::State<'_, AiConfigState>,
) -> Result<MemorySummary, String> {
    let ai_cfg = ai_config.0.lock().await.clone();
    if !ai_cfg.enabled {
        return Ok(MemorySummary {
            summary_text: "AI is disabled. Enable it in settings to see reports.".to_string(),
            total_copied: 0,
            top_category: "None".to_string(),
        });
    }

    let items = database::get_items(&db_state.0, 50)
        .await
        .map_err(|e| e.to_string())?;

    if items.is_empty() {
        return Ok(MemorySummary {
            summary_text: "No memory records found to analyze this week.".to_string(),
            total_copied: 0,
            top_category: "None".to_string(),
        });
    }

    let mut categories = std::collections::HashMap::new();
    let mut item_summaries = Vec::new();
    for item in &items {
        if let Some(ref cat) = item.ai_category {
            *categories.entry(cat.clone()).or_insert(0) += 1;
        }
        if let Some(ref sum) = item.ai_summary {
            item_summaries.push(sum.clone());
        }
    }

    let top_category = categories.into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(cat, _)| cat)
        .unwrap_or_else(|| "Other".to_string());

    let recaps = item_summaries.iter().take(15).cloned().collect::<Vec<String>>().join("\n- ");
    let prompt = format!(
        "You are CYMOS, a personal memory analyzer. Review this list of summaries from the user's past week's clipboard activity and write a 3-sentence summary highlighting the key technologies, topics, or projects they spent the most time on, and suggest a topic to review. Be structured.\n\nWeekly Memory Items:\n- {recaps}"
    );

    let summary_text = ollama::generate_text(&prompt, &ai_cfg).await.unwrap_or_else(|_| "Failed to generate weekly AI report.".to_string());

    Ok(MemorySummary {
        summary_text,
        total_copied: items.len() as i64,
        top_category,
    })
}

// ─── Agent Commands (v0.0.7) ──────────────────────────────────────────────

#[tauri::command]
async fn run_agent_task(
    db_state: tauri::State<'_, DbState>,
    embed_state: tauri::State<'_, EmbeddingConfigState>,
    ai_config: tauri::State<'_, AiConfigState>,
    agent_type: String,
    task: String,
) -> Result<agents::AgentResult, String> {
    let ai_cfg    = ai_config.0.lock().await.clone();
    let embed_cfg = embed_state.0.lock().await.clone();

    if !ai_cfg.enabled {
        return Err("AI is disabled. Enable it in Settings to use Agent Intelligence.".to_string());
    }

    let atype = agents::AgentType::from_str(&agent_type);
    agents::run_agent(atype, &task, &db_state.0, &ai_cfg, &embed_cfg)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn orchestrate_task(
    db_state: tauri::State<'_, DbState>,
    embed_state: tauri::State<'_, EmbeddingConfigState>,
    ai_config: tauri::State<'_, AiConfigState>,
    task: String,
) -> Result<agents::AgentResult, String> {
    let ai_cfg    = ai_config.0.lock().await.clone();
    let embed_cfg = embed_state.0.lock().await.clone();

    if !ai_cfg.enabled {
        return Err("AI is disabled. Enable it in Settings to use Agent Intelligence.".to_string());
    }

    agents::orchestrate(&task, &db_state.0, &ai_cfg, &embed_cfg)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_agent_history(
    db_state: tauri::State<'_, DbState>,
    limit: Option<i64>,
) -> Result<Vec<database::AgentTask>, String> {
    database::get_agent_tasks(&db_state.0, limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}

// ─── Autonomous Intelligence Commands (v0.0.8) ──────────────────────────

#[tauri::command]
async fn get_notifications(
    db_state: tauri::State<'_, DbState>,
    unread_only: bool,
) -> Result<Vec<database::Notification>, String> {
    database::get_notifications(&db_state.0, unread_only)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn mark_notification_read(
    db_state: tauri::State<'_, DbState>,
    id: i64,
) -> Result<(), String> {
    database::mark_notification_read(&db_state.0, id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn dismiss_notification(
    db_state: tauri::State<'_, DbState>,
    id: i64,
) -> Result<(), String> {
    database::dismiss_notification(&db_state.0, id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_knowledge_health(
    db_state: tauri::State<'_, DbState>,
) -> Result<Option<database::KnowledgeHealth>, String> {
    database::get_latest_knowledge_health(&db_state.0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_scheduler_runs(
    db_state: tauri::State<'_, DbState>,
    limit: i64,
) -> Result<Vec<database::SchedulerRun>, String> {
    database::get_scheduler_runs(&db_state.0, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn toggle_scheduler(
    scheduler_state: tauri::State<'_, SchedulerState>,
    enabled: bool,
) -> Result<(), String> {
    let mut state = scheduler_state.0.lock().await;
    *state = enabled;
    Ok(())
}

#[tauri::command]
async fn run_maintenance_now(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbState>,
    ai_config: tauri::State<'_, AiConfigState>,
    embed_config: tauri::State<'_, EmbeddingConfigState>,
) -> Result<(), String> {
    scheduler::run_maintenance_cycle(&app, &db_state.0, &ai_config.0, &embed_config.0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_embedding_model(
    embed_state: tauri::State<'_, EmbeddingConfigState>,
    model: String,
    enabled: bool,
) -> Result<(), String> {
    let mut cfg = embed_state.0.lock().await;
    cfg.model = model;
    cfg.enabled = enabled;
    Ok(())
}

#[tauri::command]
async fn embed_all_items(
    db_state: tauri::State<'_, DbState>,
    embed_state: tauri::State<'_, EmbeddingConfigState>,
    app: tauri::AppHandle,
) -> Result<i64, String> {
    let cfg = embed_state.0.lock().await.clone();
    if !cfg.enabled {
        return Err("Embedding disabled".to_string());
    }

    let items = database::get_unembedded_items(&db_state.0)
        .await
        .map_err(|e| e.to_string())?;

    let count = items.len() as i64;
    let pool = db_state.0.clone();

    // Process in background so the command returns immediately
    tauri::async_runtime::spawn(async move {
        for (id, content) in items {
            let text: String = content.chars().take(4096).collect();
            match embeddings::generate_embedding(&cfg, &text).await {
                Ok(vec) => {
                    let json = embeddings::encode_vec(&vec);
                    if let Ok(()) = database::update_embedding(&pool, id, &json).await {
                        let _ = app.emit("embedding-ready", id);
                    }
                }
                Err(e) => eprintln!("[CYMOS EMBED-ALL] item {}: {}", id, e),
            }
        }
    });

    Ok(count)
}

// ─── App Setup ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create AI and embedding queue channels
    let (ai_tx, ai_rx) = mpsc::channel::<i64>(256);
    let (embed_tx, embed_rx) = mpsc::channel::<i64>(512);
    let (graph_tx, graph_rx) = mpsc::channel::<i64>(256);
    let ai_config = Arc::new(Mutex::new(ollama::OllamaConfig::default()));
    let embed_config = Arc::new(Mutex::new(embeddings::EmbeddingConfig::default()));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let _handle = app.handle().clone();
            // Smart directory resolution: if running from src-tauri, step up to the project root
            let mut project_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            if project_dir.ends_with("src-tauri") {
                project_dir.pop();
            }

            let db_dir = project_dir.join("database");
            if !db_dir.exists() {
                let _ = std::fs::create_dir_all(&db_dir);
            }

            let db_file = db_dir.join("cymos.db");
            let db_url = format!("sqlite:{}", db_file.to_string_lossy());

            // Initialize database connection pool
            let pool = tauri::async_runtime::block_on(async {
                database::init_db(&db_url)
                    .await
                    .expect("Failed to initialize SQLite database")
            });

            // Store state
            app.manage(DbState(pool.clone()));
            app.manage(AiConfigState(ai_config.clone()));
            app.manage(EmbeddingConfigState(embed_config.clone()));
            
            let scheduler_state = Arc::new(Mutex::new(true)); // Enabled by default
            app.manage(SchedulerState(scheduler_state.clone()));

            // Start clipboard monitor (passes AI, embed, and graph queue senders)
            clipboard::start_monitoring(app.handle().clone(), pool.clone(), ai_tx, embed_tx, graph_tx);

            // Start background AI queue worker
            start_ai_queue_worker(
                app.handle().clone(),
                pool.clone(),
                ai_config.clone(),
                ai_rx,
            );

            // Start background embedding queue worker
            start_embedding_worker(
                app.handle().clone(),
                pool.clone(),
                embed_config.clone(),
                embed_rx,
            );

            // Start background graph queue worker
            start_graph_worker(
                app.handle().clone(),
                pool.clone(),
                ai_config.clone(),
                graph_rx,
            );

            // Start Autonomous Intelligence Engine
            scheduler::start_scheduler(
                app.handle().clone(),
                pool.clone(),
                ai_config.clone(),
                embed_config.clone(),
                scheduler_state,
            );

            // Start Local REST API Server
            let api_pool = pool.clone();
            tauri::async_runtime::spawn(async move {
                api::start_server(api_pool, 11435).await;
            });

            // Configure System Tray Menu Items
            let quit_i = MenuItem::with_id(app, "quit", "Quit CYMOS", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            // Build Status Bar / System Tray Icon
            let icon = app.default_window_icon().cloned();
            let mut tray_builder = TrayIconBuilder::new();
            if let Some(icon) = icon {
                tray_builder = tray_builder.icon(icon);
            }

            let _tray = tray_builder
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "quit" => { app.exit(0); }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible { let _ = window.hide(); }
                            else { let _ = window.show(); let _ = window.set_focus(); }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Clipboard
            get_clipboard_items,
            get_clipboard_items_filtered,
            search_clipboard_items,
            delete_clipboard_item,
            clear_clipboard_history,
            write_to_clipboard,
            toggle_favorite,
            move_to_collection,
            // Collections
            get_collections,
            create_collection,
            update_collection,
            delete_collection,
            // Tags
            get_tags,
            create_tag,
            delete_tag,
            add_tag_to_item,
            remove_tag_from_item,
            get_tags_for_item,
            // Statistics & Export
            get_statistics,
            export_items_json,
            export_items_csv,
            export_items_markdown,
            // AI
            get_ollama_status,
            get_ai_config,
            set_ai_config,
            get_memory_insights,
            process_item_with_ai,
            process_all_with_ai,
            // Semantic Search (v0.4)
            semantic_search,
            get_similar_items,
            get_embedding_status,
            set_embedding_model,
            embed_all_items,
            // Graph (v0.5)
            get_graph_data,
            get_item_graph,
            extract_all_graphs,
            // Assistant (v0.0.6)
            chat_with_memory,
            get_daily_summary,
            get_weekly_report,
            // Agents (v0.0.7)
            run_agent_task,
            orchestrate_task,
            get_agent_history,
            // Autonomous (v0.0.8)
            get_notifications,
            mark_notification_read,
            dismiss_notification,
            get_knowledge_health,
            get_scheduler_runs,
            toggle_scheduler,
            run_maintenance_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
