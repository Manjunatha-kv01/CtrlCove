use axum::{
    extract::{State, Query},
    routing::{get, post},
    Json, Router, http::StatusCode,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

use crate::database::{self, NewClipboardItem, ClipboardItem};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct CreateMemoryRequest {
    pub content: String,
    pub source_app: Option<String>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: "1.0.0".to_string(),
    })
}

async fn list_memories(
    State(state): State<AppState>,
) -> Result<Json<Vec<ClipboardItem>>, (StatusCode, String)> {
    match database::get_items(&state.pool, 50).await {
        Ok(items) => Ok(Json(items)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn search_memories(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<ClipboardItem>>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(20);
    // Simple text search fallback since semantic search requires embeddings which is complex here
    match database::search_items(&state.pool, &query.q).await {
        Ok(items) => Ok(Json(items)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn create_memory(
    State(state): State<AppState>,
    Json(req): Json<CreateMemoryRequest>,
) -> Result<Json<ClipboardItem>, (StatusCode, String)> {
    use sha2::{Digest, Sha256};
    
    let mut hasher = Sha256::new();
    hasher.update(&req.content);
    let hash = hex::encode(hasher.finalize());

    let new_item = NewClipboardItem {
        content: req.content.clone(),
        content_type: "Text".to_string(),
        content_hash: hash,
        source_app: Some(req.source_app.unwrap_or_else(|| "API".to_string())),
        char_count: req.content.chars().count() as i64,
        word_count: req.content.split_whitespace().count() as i64,
        file_size: req.content.len() as i64,
        image_width: 0,
        image_height: 0,
        language: None,
        image_data: None,
    };

    match database::insert_item(&state.pool, new_item).await {
        Ok(item) => Ok(Json(item)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// ─── Server Setup ────────────────────────────────────────────────────────────

pub async fn start_server(pool: SqlitePool, port: u16) {
    let state = AppState { pool };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/memories", get(list_memories))
        .route("/api/v1/memories", post(create_memory))
        .route("/api/v1/search", get(search_memories))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("API Server listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
