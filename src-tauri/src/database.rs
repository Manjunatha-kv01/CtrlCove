use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

// ─── Data Structures ─────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow, Clone)]
pub struct ClipboardItem {
    pub id: i64,
    pub content: String,
    pub content_type: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub content_hash: Option<String>,
    pub source_app: Option<String>,
    pub char_count: Option<i64>,
    pub word_count: Option<i64>,
    pub file_size: Option<i64>,
    pub image_width: Option<i64>,
    pub image_height: Option<i64>,
    pub language: Option<String>,
    pub is_favorite: Option<i64>,
    pub collection_id: Option<i64>,
    pub image_data: Option<String>,
    // v0.3 AI fields
    pub ai_summary: Option<String>,
    pub ai_category: Option<String>,
    pub ai_keywords: Option<String>,
    pub ai_tags: Option<String>,
    pub reading_time: Option<i64>,
    pub copy_count: Option<i64>,
    pub ai_processed: Option<i64>,
    pub ai_model: Option<String>,
    // v0.4 Semantic fields
    pub embedding_status: Option<i64>,  // 0 = pending, 1 = done
}

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow, Clone)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Statistics {
    pub total_items: i64,
    pub text_items: i64,
    pub url_items: i64,
    pub code_items: i64,
    pub image_items: i64,
    pub file_items: i64,
    pub html_items: i64,
    pub json_items: i64,
    pub email_items: i64,
    pub color_items: i64,
    pub favorites_count: i64,
    pub collections_count: i64,
    pub tags_count: i64,
    pub storage_bytes: i64,
    pub ai_processed_count: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, sqlx::FromRow)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct MemoryInsights {
    pub top_categories: Vec<CategoryCount>,
    pub top_keywords: Vec<String>,
    pub most_copied: Vec<ClipboardItem>,
    pub ai_processed_count: i64,
    pub ai_pending_count: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct FilterOptions {
    pub content_type: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub collection_id: Option<i64>,
    pub tag_id: Option<i64>,
    pub is_favorite: Option<bool>,
    pub language: Option<String>,
    pub source_app: Option<String>,
    pub limit: Option<i64>,
}

// ─── Database Initialization & Migration ─────────────────────────────────────

pub async fn init_db(db_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(db_url)?
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options).await?;

    // Enable WAL mode for better concurrent performance
    sqlx::query("PRAGMA journal_mode=WAL;").execute(&pool).await?;
    sqlx::query("PRAGMA synchronous=NORMAL;").execute(&pool).await?;

    // Create base table (if fresh install)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS clipboard_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            content_type TEXT NOT NULL DEFAULT 'Text',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
         );"
    )
    .execute(&pool)
    .await?;

    // ── Migration: safely add new columns if they don't exist ──
    let migrations: &[(&str, &str)] = &[
        ("updated_at",    "ALTER TABLE clipboard_items ADD COLUMN updated_at TEXT"),
        ("content_hash",  "ALTER TABLE clipboard_items ADD COLUMN content_hash TEXT"),
        ("source_app",    "ALTER TABLE clipboard_items ADD COLUMN source_app TEXT"),
        ("char_count",    "ALTER TABLE clipboard_items ADD COLUMN char_count INTEGER DEFAULT 0"),
        ("word_count",    "ALTER TABLE clipboard_items ADD COLUMN word_count INTEGER DEFAULT 0"),
        ("file_size",     "ALTER TABLE clipboard_items ADD COLUMN file_size INTEGER DEFAULT 0"),
        ("image_width",   "ALTER TABLE clipboard_items ADD COLUMN image_width INTEGER DEFAULT 0"),
        ("image_height",  "ALTER TABLE clipboard_items ADD COLUMN image_height INTEGER DEFAULT 0"),
        ("language",      "ALTER TABLE clipboard_items ADD COLUMN language TEXT"),
        ("is_favorite",   "ALTER TABLE clipboard_items ADD COLUMN is_favorite INTEGER DEFAULT 0"),
        ("collection_id", "ALTER TABLE clipboard_items ADD COLUMN collection_id INTEGER"),
        ("image_data",    "ALTER TABLE clipboard_items ADD COLUMN image_data TEXT"),
        // v0.3 AI columns
        ("ai_summary",    "ALTER TABLE clipboard_items ADD COLUMN ai_summary TEXT"),
        ("ai_category",   "ALTER TABLE clipboard_items ADD COLUMN ai_category TEXT"),
        ("ai_keywords",   "ALTER TABLE clipboard_items ADD COLUMN ai_keywords TEXT"),
        ("ai_tags",       "ALTER TABLE clipboard_items ADD COLUMN ai_tags TEXT"),
        ("reading_time",  "ALTER TABLE clipboard_items ADD COLUMN reading_time INTEGER DEFAULT 0"),
        ("copy_count",    "ALTER TABLE clipboard_items ADD COLUMN copy_count INTEGER DEFAULT 1"),
        ("ai_processed",  "ALTER TABLE clipboard_items ADD COLUMN ai_processed INTEGER DEFAULT 0"),
        ("ai_model",      "ALTER TABLE clipboard_items ADD COLUMN ai_model TEXT"),
        // v0.4 Semantic columns
        ("embedding",        "ALTER TABLE clipboard_items ADD COLUMN embedding TEXT"),
        ("embedding_status", "ALTER TABLE clipboard_items ADD COLUMN embedding_status INTEGER DEFAULT 0"),
        // v0.5 Graph columns
        ("entity_extracted", "ALTER TABLE clipboard_items ADD COLUMN entity_extracted INTEGER DEFAULT 0"),
    ];

    let existing: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM pragma_table_info('clipboard_items')"
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    for (col, stmt) in migrations {
        if !existing.contains(&col.to_string()) {
            let _ = sqlx::query(stmt).execute(&pool).await;
        }
    }

    // ── Create new tables ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            color TEXT DEFAULT '#3b82f6',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
         );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE
         );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS clipboard_tags (
            clipboard_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY (clipboard_id, tag_id)
         );"
    ).execute(&pool).await?;

    // ── v0.5 Graph Tables ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            entity_type TEXT NOT NULL,
            first_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
         );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS item_entities (
            clipboard_id INTEGER NOT NULL,
            entity_id INTEGER NOT NULL,
            PRIMARY KEY (clipboard_id, entity_id)
         );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS relationships (
            source_id INTEGER NOT NULL,
            target_id INTEGER NOT NULL,
            relationship_type TEXT NOT NULL,
            weight REAL DEFAULT 1.0,
            PRIMARY KEY (source_id, target_id, relationship_type)
         );"
    ).execute(&pool).await?;

    // ── v0.7 Agent Tasks Table ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS agent_tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_type TEXT NOT NULL,
            task TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            result TEXT,
            steps TEXT,
            source_ids TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );"
    ).execute(&pool).await?;

    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_tasks_created ON agent_tasks(created_at DESC);").execute(&pool).await;

    // ── v0.8 Autonomous Intelligence Tables ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS notifications (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            notification_type TEXT NOT NULL,
            title TEXT NOT NULL,
            message TEXT NOT NULL,
            is_read INTEGER DEFAULT 0,
            related_ids TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS scheduler_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_type TEXT NOT NULL,
            status TEXT NOT NULL,
            items_processed INTEGER DEFAULT 0,
            duration_ms INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS knowledge_health (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            total_memories INTEGER DEFAULT 0,
            embedded_count INTEGER DEFAULT 0,
            ai_processed_count INTEGER DEFAULT 0,
            graph_entities INTEGER DEFAULT 0,
            graph_relationships INTEGER DEFAULT 0,
            duplicate_groups INTEGER DEFAULT 0,
            avg_connections REAL DEFAULT 0.0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );"
    ).execute(&pool).await?;

    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_notifications_unread ON notifications(is_read);").execute(&pool).await;

    // ── Indexes for performance ──
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_items_type ON clipboard_items(content_type);").execute(&pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_items_created ON clipboard_items(created_at DESC);").execute(&pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_items_fav ON clipboard_items(is_favorite);").execute(&pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_items_collection ON clipboard_items(collection_id);").execute(&pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_items_hash ON clipboard_items(content_hash);").execute(&pool).await;

    Ok(pool)
}

// ─── Insert ──────────────────────────────────────────────────────────────────

pub struct NewClipboardItem {
    pub content: String,
    pub content_type: String,
    pub content_hash: String,
    pub source_app: Option<String>,
    pub char_count: i64,
    pub word_count: i64,
    pub file_size: i64,
    pub image_width: i64,
    pub image_height: i64,
    pub language: Option<String>,
    pub image_data: Option<String>,
}

pub async fn insert_item(pool: &SqlitePool, item: NewClipboardItem) -> Result<ClipboardItem, sqlx::Error> {
    // Check for duplicate by content hash — increment copy_count instead of inserting
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM clipboard_items WHERE content_hash = ? ORDER BY id DESC LIMIT 1"
    )
    .bind(&item.content_hash)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = existing {
        // Update timestamp and increment copy_count
        sqlx::query(
            "UPDATE clipboard_items SET created_at = CURRENT_TIMESTAMP, copy_count = COALESCE(copy_count, 1) + 1 WHERE id = ?"
        )
            .bind(id)
            .execute(pool)
            .await?;
        return get_item_by_id(pool, id).await;
    }

    sqlx::query_as::<_, ClipboardItem>(
        "INSERT INTO clipboard_items
            (content, content_type, content_hash, source_app, char_count, word_count,
             file_size, image_width, image_height, language, image_data, copy_count, ai_processed)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 0)
         RETURNING *"
    )
    .bind(&item.content)
    .bind(&item.content_type)
    .bind(&item.content_hash)
    .bind(&item.source_app)
    .bind(item.char_count)
    .bind(item.word_count)
    .bind(item.file_size)
    .bind(item.image_width)
    .bind(item.image_height)
    .bind(&item.language)
    .bind(&item.image_data)
    .fetch_one(pool)
    .await
}

pub async fn get_item_by_id(pool: &SqlitePool, id: i64) -> Result<ClipboardItem, sqlx::Error> {
    sqlx::query_as::<_, ClipboardItem>(
        "SELECT * FROM clipboard_items WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

// ─── Read ─────────────────────────────────────────────────────────────────────

pub async fn get_items(pool: &SqlitePool, limit: i64) -> Result<Vec<ClipboardItem>, sqlx::Error> {
    sqlx::query_as::<_, ClipboardItem>(
        "SELECT * FROM clipboard_items ORDER BY created_at DESC LIMIT ?"
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_items_filtered(
    pool: &SqlitePool,
    opts: FilterOptions,
) -> Result<Vec<ClipboardItem>, sqlx::Error> {
    let limit = opts.limit.unwrap_or(200);
    let mut conditions: Vec<String> = vec![];

    if let Some(ref ct) = opts.content_type {
        conditions.push(format!("content_type = '{}'", ct.replace('\'', "''")));
    }
    if let Some(ref df) = opts.date_from {
        conditions.push(format!("created_at >= '{}'", df.replace('\'', "''")));
    }
    if let Some(ref dt) = opts.date_to {
        conditions.push(format!("created_at <= '{}'", dt.replace('\'', "''")));
    }
    if let Some(cid) = opts.collection_id {
        conditions.push(format!("collection_id = {}", cid));
    }
    if let Some(true) = opts.is_favorite {
        conditions.push("is_favorite = 1".to_string());
    }
    if let Some(ref lang) = opts.language {
        conditions.push(format!("language = '{}'", lang.replace('\'', "''")));
    }
    if let Some(ref app) = opts.source_app {
        conditions.push(format!("source_app = '{}'", app.replace('\'', "''")));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // If filtering by tag, use a subquery
    if let Some(tag_id) = opts.tag_id {
        let sql = format!(
            "SELECT ci.* FROM clipboard_items ci
             INNER JOIN clipboard_tags ct ON ci.id = ct.clipboard_id
             WHERE ct.tag_id = {}
             {} ORDER BY ci.created_at DESC LIMIT {}",
            tag_id,
            if conditions.is_empty() { String::new() } else { format!("AND {}", conditions.join(" AND ")) },
            limit
        );
        return sqlx::query_as::<_, ClipboardItem>(&sql)
            .fetch_all(pool)
            .await;
    }

    let sql = format!(
        "SELECT * FROM clipboard_items {} ORDER BY created_at DESC LIMIT {}",
        where_clause, limit
    );

    sqlx::query_as::<_, ClipboardItem>(&sql)
        .fetch_all(pool)
        .await
}

pub async fn search_items(pool: &SqlitePool, query: &str) -> Result<Vec<ClipboardItem>, sqlx::Error> {
    sqlx::query_as::<_, ClipboardItem>(
        "SELECT * FROM clipboard_items
         WHERE content LIKE ? OR source_app LIKE ? OR language LIKE ?
               OR ai_summary LIKE ? OR ai_category LIKE ? OR ai_keywords LIKE ?
         ORDER BY created_at DESC LIMIT 200"
    )
    .bind(format!("%{}%", query))
    .bind(format!("%{}%", query))
    .bind(format!("%{}%", query))
    .bind(format!("%{}%", query))
    .bind(format!("%{}%", query))
    .bind(format!("%{}%", query))
    .fetch_all(pool)
    .await
}

// ─── AI Functions ─────────────────────────────────────────────────────────────

pub async fn update_ai_fields(
    pool: &SqlitePool,
    id: i64,
    result: &crate::ollama::AiResult,
    model: &str,
) -> Result<(), sqlx::Error> {
    let keywords_json = serde_json::to_string(&result.keywords).unwrap_or_default();
    let tags_json = serde_json::to_string(&result.auto_tags).unwrap_or_default();
    sqlx::query(
        "UPDATE clipboard_items SET
            ai_summary = ?,
            ai_category = ?,
            ai_keywords = ?,
            ai_tags = ?,
            reading_time = ?,
            ai_processed = 1,
            ai_model = ?
         WHERE id = ?"
    )
    .bind(&result.summary)
    .bind(&result.category)
    .bind(&keywords_json)
    .bind(&tags_json)
    .bind(result.reading_time_seconds)
    .bind(model)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_unprocessed_items(pool: &SqlitePool) -> Result<Vec<ClipboardItem>, sqlx::Error> {
    sqlx::query_as::<_, ClipboardItem>(
        "SELECT * FROM clipboard_items
         WHERE (ai_processed IS NULL OR ai_processed = 0)
           AND content_type != 'Image'
           AND LENGTH(content) >= 50
         ORDER BY id DESC
         LIMIT 500"
    )
    .fetch_all(pool)
    .await
}

pub async fn get_memory_insights(pool: &SqlitePool) -> Result<MemoryInsights, sqlx::Error> {
    // Top categories
    let top_categories = sqlx::query_as::<_, CategoryCount>(
        "SELECT ai_category as category, COUNT(*) as count
         FROM clipboard_items
         WHERE ai_category IS NOT NULL AND ai_category != ''
         GROUP BY ai_category
         ORDER BY count DESC
         LIMIT 8"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Top keywords (parse JSON arrays and aggregate)
    let keyword_rows: Vec<String> = sqlx::query_scalar(
        "SELECT ai_keywords FROM clipboard_items WHERE ai_keywords IS NOT NULL AND ai_keywords != '' LIMIT 200"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut keyword_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for row in &keyword_rows {
        if let Ok(kws) = serde_json::from_str::<Vec<String>>(row) {
            for kw in kws {
                *keyword_counts.entry(kw.to_lowercase()).or_insert(0) += 1;
            }
        }
    }
    let mut top_kw: Vec<(String, usize)> = keyword_counts.into_iter().collect();
    top_kw.sort_by(|a, b| b.1.cmp(&a.1));
    let top_keywords: Vec<String> = top_kw.into_iter().take(20).map(|(k, _)| k).collect();

    // Most copied items (copy_count > 1)
    let most_copied = sqlx::query_as::<_, ClipboardItem>(
        "SELECT * FROM clipboard_items
         WHERE copy_count > 1
         ORDER BY copy_count DESC
         LIMIT 5"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let ai_processed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM clipboard_items WHERE ai_processed = 1"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let ai_pending_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM clipboard_items WHERE (ai_processed IS NULL OR ai_processed = 0) AND content_type != 'Image' AND LENGTH(content) >= 50"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    Ok(MemoryInsights {
        top_categories,
        top_keywords,
        most_copied,
        ai_processed_count,
        ai_pending_count,
    })
}

// ─── Update ──────────────────────────────────────────────────────────────────

pub async fn toggle_favorite(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let current: Option<i64> = sqlx::query_scalar(
        "SELECT is_favorite FROM clipboard_items WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let new_val = match current {
        Some(1) => 0,
        _ => 1,
    };

    sqlx::query("UPDATE clipboard_items SET is_favorite = ? WHERE id = ?")
        .bind(new_val)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(new_val == 1)
}

pub async fn move_to_collection(
    pool: &SqlitePool,
    clipboard_id: i64,
    collection_id: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE clipboard_items SET collection_id = ? WHERE id = ?")
        .bind(collection_id)
        .bind(clipboard_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ─── Delete ──────────────────────────────────────────────────────────────────

pub async fn delete_item(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM clipboard_tags WHERE clipboard_id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM clipboard_items WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn clear_history(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM clipboard_tags").execute(pool).await?;
    sqlx::query("DELETE FROM clipboard_items").execute(pool).await?;
    Ok(())
}

// ─── Collections ─────────────────────────────────────────────────────────────

pub async fn get_collections(pool: &SqlitePool) -> Result<Vec<Collection>, sqlx::Error> {
    sqlx::query_as::<_, Collection>(
        "SELECT * FROM collections ORDER BY created_at ASC"
    )
    .fetch_all(pool)
    .await
}

pub async fn create_collection(pool: &SqlitePool, name: &str, color: &str) -> Result<Collection, sqlx::Error> {
    sqlx::query_as::<_, Collection>(
        "INSERT INTO collections (name, color) VALUES (?, ?) RETURNING *"
    )
    .bind(name)
    .bind(color)
    .fetch_one(pool)
    .await
}

pub async fn update_collection(pool: &SqlitePool, id: i64, name: &str, color: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE collections SET name = ?, color = ? WHERE id = ?")
        .bind(name)
        .bind(color)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_collection(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    // Unlink items from this collection
    sqlx::query("UPDATE clipboard_items SET collection_id = NULL WHERE collection_id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ─── Tags ────────────────────────────────────────────────────────────────────

pub async fn get_tags(pool: &SqlitePool) -> Result<Vec<Tag>, sqlx::Error> {
    sqlx::query_as::<_, Tag>("SELECT * FROM tags ORDER BY name ASC")
        .fetch_all(pool)
        .await
}

pub async fn create_tag(pool: &SqlitePool, name: &str) -> Result<Tag, sqlx::Error> {
    // Upsert pattern
    sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES (?)")
        .bind(name)
        .execute(pool)
        .await?;
    sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE name = ?")
        .bind(name)
        .fetch_one(pool)
        .await
}

pub async fn delete_tag(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM clipboard_tags WHERE tag_id = ?").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM tags WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn add_tag_to_item(pool: &SqlitePool, clipboard_id: i64, tag_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR IGNORE INTO clipboard_tags (clipboard_id, tag_id) VALUES (?, ?)")
        .bind(clipboard_id)
        .bind(tag_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn remove_tag_from_item(pool: &SqlitePool, clipboard_id: i64, tag_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM clipboard_tags WHERE clipboard_id = ? AND tag_id = ?")
        .bind(clipboard_id)
        .bind(tag_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_tags_for_item(pool: &SqlitePool, clipboard_id: i64) -> Result<Vec<Tag>, sqlx::Error> {
    sqlx::query_as::<_, Tag>(
        "SELECT t.* FROM tags t
         INNER JOIN clipboard_tags ct ON t.id = ct.tag_id
         WHERE ct.clipboard_id = ?
         ORDER BY t.name ASC"
    )
    .bind(clipboard_id)
    .fetch_all(pool)
    .await
}

// ─── Statistics ──────────────────────────────────────────────────────────────

pub async fn get_statistics(pool: &SqlitePool) -> Result<Statistics, sqlx::Error> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items")
        .fetch_one(pool).await.unwrap_or(0);
    let text: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE content_type = 'Text'")
        .fetch_one(pool).await.unwrap_or(0);
    let url: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE content_type = 'URL'")
        .fetch_one(pool).await.unwrap_or(0);
    let code: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE content_type = 'Code'")
        .fetch_one(pool).await.unwrap_or(0);
    let image: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE content_type = 'Image'")
        .fetch_one(pool).await.unwrap_or(0);
    let file: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE content_type = 'File'")
        .fetch_one(pool).await.unwrap_or(0);
    let html: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE content_type = 'HTML'")
        .fetch_one(pool).await.unwrap_or(0);
    let json: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE content_type = 'JSON'")
        .fetch_one(pool).await.unwrap_or(0);
    let email: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE content_type = 'Email'")
        .fetch_one(pool).await.unwrap_or(0);
    let color: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE content_type = 'Color'")
        .fetch_one(pool).await.unwrap_or(0);
    let favs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE is_favorite = 1")
        .fetch_one(pool).await.unwrap_or(0);
    let collections: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM collections")
        .fetch_one(pool).await.unwrap_or(0);
    let tags: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags")
        .fetch_one(pool).await.unwrap_or(0);
    let storage: i64 = sqlx::query_scalar("SELECT SUM(LENGTH(content) + COALESCE(LENGTH(image_data), 0)) FROM clipboard_items")
        .fetch_one(pool).await.unwrap_or(0);
    let ai_processed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE ai_processed = 1")
        .fetch_one(pool).await.unwrap_or(0);

    Ok(Statistics {
        total_items: total,
        text_items: text,
        url_items: url,
        code_items: code,
        image_items: image,
        file_items: file,
        html_items: html,
        json_items: json,
        email_items: email,
        color_items: color,
        favorites_count: favs,
        collections_count: collections,
        tags_count: tags,
        storage_bytes: storage,
        ai_processed_count: ai_processed,
    })
}

// ─── Export ──────────────────────────────────────────────────────────────────

pub async fn export_items_json(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let items = get_items(pool, 10000).await?;
    Ok(serde_json::to_string_pretty(&items).unwrap_or_default())
}

pub async fn export_items_csv(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let items = get_items(pool, 10000).await?;
    let mut csv = String::from("id,content_type,language,source_app,char_count,word_count,is_favorite,created_at,content\n");
    for item in items {
        let content_escaped = item.content.replace('"', "\"\"").replace('\n', "\\n");
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},\"{}\"\n",
            item.id,
            item.content_type,
            item.language.unwrap_or_default(),
            item.source_app.unwrap_or_default(),
            item.char_count.unwrap_or(0),
            item.word_count.unwrap_or(0),
            item.is_favorite.unwrap_or(0),
            item.created_at,
            content_escaped,
        ));
    }
    Ok(csv)
}

pub async fn export_items_markdown(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let items = get_items(pool, 10000).await?;
    let mut md = String::from("# CYMOS Clipboard Export\n\n");
    for item in items {
        md.push_str(&format!("## Item #{} — {}\n", item.id, item.content_type));
        md.push_str(&format!("*Captured: {}*", item.created_at));
        if let Some(app) = &item.source_app {
            md.push_str(&format!(" | *Source: {}*", app));
        }
        md.push_str("\n\n");
        if item.content_type == "Code" {
            let lang = item.language.as_deref().unwrap_or("");
            md.push_str(&format!("```{}\n{}\n```\n\n", lang.to_lowercase(), item.content));
        } else {
            md.push_str(&format!("{}\n\n", item.content));
        }
        md.push_str("---\n\n");
    }
    Ok(md)
}

// ─── v0.4 Semantic / Embedding Functions ──────────────────────────────────────

/// Store a serialised embedding vector and mark the item as embedded.
pub async fn update_embedding(
    pool: &SqlitePool,
    id: i64,
    embedding_json: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE clipboard_items SET embedding = ?, embedding_status = 1 WHERE id = ?"
    )
    .bind(embedding_json)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Lightweight row for similarity scoring — avoids loading full content into memory.
#[derive(Debug, sqlx::FromRow)]
pub struct EmbedRow {
    pub id: i64,
    pub embedding: Option<String>,
    pub is_favorite: Option<i64>,
    pub copy_count: Option<i64>,
    pub created_at: String,
}

/// Fetch all rows that have embeddings (used for semantic search).
pub async fn get_embedded_rows(pool: &SqlitePool) -> Result<Vec<EmbedRow>, sqlx::Error> {
    sqlx::query_as::<_, EmbedRow>(
        "SELECT id, embedding, is_favorite, copy_count, created_at
         FROM clipboard_items
         WHERE embedding_status = 1 AND embedding IS NOT NULL"
    )
    .fetch_all(pool)
    .await
}

/// Count how many items have been embedded.
pub async fn count_embedded(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM clipboard_items WHERE embedding_status = 1"
    )
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Count items eligible for embedding but not yet done.
pub async fn count_pending_embed(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM clipboard_items
         WHERE embedding_status = 0
           AND content_type != 'Image'
           AND LENGTH(content) >= 20"
    )
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Retrieve full ClipboardItems by an ordered list of IDs (for semantic search results).
/// Preserves the ordering of the provided id list.
pub async fn get_items_by_ids(
    pool: &SqlitePool,
    ids: &[i64],
) -> Result<Vec<ClipboardItem>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let placeholders: String = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id, content, content_type, created_at, updated_at, content_hash, source_app,
                char_count, word_count, file_size, image_width, image_height, language,
                is_favorite, collection_id, image_data,
                ai_summary, ai_category, ai_keywords, ai_tags, reading_time,
                copy_count, ai_processed, ai_model, embedding_status
         FROM clipboard_items WHERE id IN ({placeholders})"
    );
    let mut q = sqlx::query_as::<_, ClipboardItem>(&sql);
    for id in ids {
        q = q.bind(id);
    }
    let mut items = q.fetch_all(pool).await?;
    // Re-order to match the requested id order (semantic score order)
    items.sort_by_key(|item| ids.iter().position(|&x| x == item.id).unwrap_or(usize::MAX));
    Ok(items)
}

/// Fetch all items eligible for embedding but not yet processed (for batch embed-all).
pub async fn get_unembedded_items(pool: &SqlitePool) -> Result<Vec<(i64, String)>, sqlx::Error> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, content FROM clipboard_items
         WHERE embedding_status = 0
           AND content_type != 'Image'
           AND LENGTH(content) >= 20
         ORDER BY id DESC"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Fetch all items eligible for graph extraction but not yet processed.
pub async fn get_unextracted_items(pool: &SqlitePool) -> Result<Vec<(i64, String)>, sqlx::Error> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, content FROM clipboard_items
         WHERE (entity_extracted = 0 OR entity_extracted IS NULL)
           AND content_type != 'Image'
           AND LENGTH(content) >= 50
         ORDER BY id DESC"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ─── Agent Task CRUD (v0.0.7) ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct AgentTask {
    pub id: i64,
    pub agent_type: String,
    pub task: String,
    pub status: String,
    pub result: Option<String>,
    pub steps: Option<String>,      // JSON array of {agent, output}
    pub source_ids: Option<String>, // JSON array of clipboard item IDs
    pub created_at: String,
}

pub async fn insert_agent_task(
    pool: &SqlitePool,
    agent_type: &str,
    task: &str,
) -> Result<AgentTask, sqlx::Error> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO agent_tasks (agent_type, task, status) VALUES (?, ?, 'running') RETURNING id"
    )
    .bind(agent_type)
    .bind(task)
    .fetch_one(pool)
    .await?;

    sqlx::query_as::<_, AgentTask>("SELECT * FROM agent_tasks WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn update_agent_task_result(
    pool: &SqlitePool,
    id: i64,
    result: &str,
    steps: &str,
    source_ids: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE agent_tasks SET result = ?, steps = ?, source_ids = ?, status = ? WHERE id = ?"
    )
    .bind(result)
    .bind(steps)
    .bind(source_ids)
    .bind(status)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_agent_tasks(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<AgentTask>, sqlx::Error> {
    sqlx::query_as::<_, AgentTask>(
        "SELECT * FROM agent_tasks ORDER BY created_at DESC LIMIT ?"
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

// ─── Autonomous Intelligence CRUD (v0.0.8) ───────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: i64,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub is_read: i64,
    pub related_ids: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct SchedulerRun {
    pub id: i64,
    pub task_type: String,
    pub status: String,
    pub items_processed: i64,
    pub duration_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct KnowledgeHealth {
    pub id: i64,
    pub total_memories: i64,
    pub embedded_count: i64,
    pub ai_processed_count: i64,
    pub graph_entities: i64,
    pub graph_relationships: i64,
    pub duplicate_groups: i64,
    pub avg_connections: f64,
    pub created_at: String,
}

pub async fn insert_notification(
    pool: &SqlitePool,
    notification_type: &str,
    title: &str,
    message: &str,
    related_ids: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO notifications (notification_type, title, message, related_ids) VALUES (?, ?, ?, ?)"
    )
    .bind(notification_type)
    .bind(title)
    .bind(message)
    .bind(related_ids)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_notifications(
    pool: &SqlitePool,
    unread_only: bool,
) -> Result<Vec<Notification>, sqlx::Error> {
    if unread_only {
        sqlx::query_as::<_, Notification>("SELECT * FROM notifications WHERE is_read = 0 ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
    } else {
        sqlx::query_as::<_, Notification>("SELECT * FROM notifications ORDER BY created_at DESC LIMIT 50")
            .fetch_all(pool)
            .await
    }
}

pub async fn mark_notification_read(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE notifications SET is_read = 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn dismiss_notification(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM notifications WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_scheduler_run(
    pool: &SqlitePool,
    task_type: &str,
    status: &str,
    items_processed: i64,
    duration_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO scheduler_runs (task_type, status, items_processed, duration_ms) VALUES (?, ?, ?, ?)"
    )
    .bind(task_type)
    .bind(status)
    .bind(items_processed)
    .bind(duration_ms)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_scheduler_runs(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<SchedulerRun>, sqlx::Error> {
    sqlx::query_as::<_, SchedulerRun>("SELECT * FROM scheduler_runs ORDER BY created_at DESC LIMIT ?")
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn insert_knowledge_health(
    pool: &SqlitePool,
    health: &KnowledgeHealth,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO knowledge_health 
        (total_memories, embedded_count, ai_processed_count, graph_entities, graph_relationships, duplicate_groups, avg_connections) 
        VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(health.total_memories)
    .bind(health.embedded_count)
    .bind(health.ai_processed_count)
    .bind(health.graph_entities)
    .bind(health.graph_relationships)
    .bind(health.duplicate_groups)
    .bind(health.avg_connections)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_latest_knowledge_health(
    pool: &SqlitePool,
) -> Result<Option<KnowledgeHealth>, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeHealth>("SELECT * FROM knowledge_health ORDER BY created_at DESC LIMIT 1")
        .fetch_optional(pool)
        .await
}
