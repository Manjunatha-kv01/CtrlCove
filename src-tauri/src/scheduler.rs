use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter};
use std::time::Duration;
use chrono::{Utc, TimeZone};

use crate::{database, ollama, embeddings, graph};

// ─── Constants ───────────────────────────────────────────────────────────────

const CYCLE_INTERVAL_SECS: u64 = 300; // 5 minutes

// ─── Scheduler Core ──────────────────────────────────────────────────────────

pub fn start_scheduler(
    app: AppHandle,
    pool: SqlitePool,
    ai_config: Arc<Mutex<ollama::OllamaConfig>>,
    embed_config: Arc<Mutex<embeddings::EmbeddingConfig>>,
    enabled_state: Arc<Mutex<bool>>,
) {
    tauri::async_runtime::spawn(async move {
        // Wait a bit on startup before the first cycle
        tokio::time::sleep(Duration::from_secs(15)).await;

        loop {
            let is_enabled = *enabled_state.lock().await;
            if is_enabled {
                let _ = run_maintenance_cycle(&app, &pool, &ai_config, &embed_config).await;
            }
            tokio::time::sleep(Duration::from_secs(CYCLE_INTERVAL_SECS)).await;
        }
    });
}

// ─── Cycle Runner ────────────────────────────────────────────────────────────

pub async fn run_maintenance_cycle(
    app: &AppHandle,
    pool: &SqlitePool,
    ai_config: &Arc<Mutex<ollama::OllamaConfig>>,
    embed_config: &Arc<Mutex<embeddings::EmbeddingConfig>>,
) -> Result<(), String> {
    
    // 1. Snapshot Knowledge Health
    let _ = snapshot_knowledge_health(pool).await;

    let ai_cfg = ai_config.lock().await.clone();
    let embed_cfg = embed_config.lock().await.clone();

    // 2. Auto-Embed Unprocessed Items
    if embed_cfg.enabled {
        if let Ok(unembedded) = database::get_unembedded_items(pool).await {
            let count = unembedded.len();
            if count > 0 {
                let start = std::time::Instant::now();
                let mut processed = 0;
                for (id, content) in unembedded.into_iter().take(20) { // limit per cycle
                    let text: String = content.chars().take(4096).collect();
                    if let Ok(vec) = embeddings::generate_embedding(&embed_cfg, &text).await {
                        let json = embeddings::encode_vec(&vec);
                        if database::update_embedding(pool, id, &json).await.is_ok() {
                            processed += 1;
                        }
                    }
                }
                let dur = start.elapsed().as_millis() as i64;
                let _ = database::insert_scheduler_run(pool, "auto-embed", "done", processed, dur).await;
            }
        }
    }

    // 3. Auto-AI-Process
    if ai_cfg.enabled {
        if let Ok(unprocessed) = database::get_unprocessed_items(pool).await {
            let count = unprocessed.len();
            if count > 0 {
                let start = std::time::Instant::now();
                let mut processed = 0;
                for item in unprocessed.into_iter().take(10) { // limit per cycle
                    let mut text = item.content.clone();
                    if text.len() > 2000 {
                        text = text.chars().take(2000).collect();
                    }
                    if let Ok(result) = ollama::enrich_item(&text, &item.content_type, item.language.as_deref(), &ai_cfg).await {
                        let _ = database::update_ai_fields(pool, item.id, &result, &ai_cfg.model).await;
                        processed += 1;
                    }
                }
                let dur = start.elapsed().as_millis() as i64;
                let _ = database::insert_scheduler_run(pool, "auto-ai", "done", processed, dur).await;
            }
        }
    }

    // 4. Auto-Graph Extraction
    if ai_cfg.enabled {
        if let Ok(unextracted) = database::get_unextracted_items(pool).await {
            let count = unextracted.len();
            if count > 0 {
                let start = std::time::Instant::now();
                let mut processed = 0;
                for (id, content) in unextracted.into_iter().take(5) { // very slow, small limit
                    let mut text = content;
                    if text.len() > 1500 {
                        text = text.chars().take(1500).collect();
                    }
                    if let Ok(graph_data) = ollama::extract_graph(&text, &ai_cfg).await {
                        if graph::ingest_graph_data(pool, id, graph_data).await.is_ok() {
                            let _ = sqlx::query("UPDATE clipboard_items SET entity_extracted = 1 WHERE id = ?").bind(id).execute(pool).await;
                            processed += 1;
                        }
                    }
                }
                let dur = start.elapsed().as_millis() as i64;
                let _ = database::insert_scheduler_run(pool, "auto-graph", "done", processed, dur).await;
            }
        }
    }

    // Notify UI that a cycle completed
    let _ = app.emit("scheduler-cycle-complete", ());
    
    Ok(())
}

// ─── Health Snapshot ─────────────────────────────────────────────────────────

async fn snapshot_knowledge_health(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let total_memories: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items")
        .fetch_one(pool)
        .await?;

    let embedded_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE embedding_status = 1")
        .fetch_one(pool)
        .await?;

    let ai_processed_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE ai_processed = 1")
        .fetch_one(pool)
        .await?;

    let graph_entities: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entities")
        .fetch_one(pool)
        .await?;

    let graph_relationships: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM relationships")
        .fetch_one(pool)
        .await?;
        
    let avg_connections: f64 = if graph_entities > 0 {
        graph_relationships as f64 / graph_entities as f64
    } else {
        0.0
    };

    let health = database::KnowledgeHealth {
        id: 0,
        total_memories,
        embedded_count,
        ai_processed_count,
        graph_entities,
        graph_relationships,
        duplicate_groups: 0, // Simplified for now
        avg_connections,
        created_at: String::new(),
    };

    database::insert_knowledge_health(pool, &health).await
}
