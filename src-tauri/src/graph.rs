use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};

// ─── Data Structures ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Entity {
    pub id: i64,
    pub name: String,
    pub entity_type: String,
    pub first_seen_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphEdge {
    pub source: i64,
    pub target: i64,
    pub relationship: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphData {
    pub nodes: Vec<Entity>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedGraphData {
    pub entities: Vec<ExtractedEntity>,
    pub relationships: Vec<ExtractedRelationship>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    pub source: String,
    pub target: String,
    pub relationship_type: String,
}

// ─── Core Logic ──────────────────────────────────────────────────────────────

/// Inserts extracted entities and relationships into the database for a specific clipboard item
pub async fn ingest_graph_data(
    pool: &SqlitePool,
    clipboard_id: i64,
    data: ExtractedGraphData,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for e in data.entities {
        // Upsert Entity
        sqlx::query(
            "INSERT INTO entities (name, entity_type) VALUES (?, ?)
             ON CONFLICT(name) DO UPDATE SET entity_type=excluded.entity_type"
        )
        .bind(&e.name)
        .bind(&e.entity_type)
        .execute(&mut *tx)
        .await?;

        // Link Item to Entity
        sqlx::query(
            "INSERT OR IGNORE INTO item_entities (clipboard_id, entity_id)
             SELECT ?, id FROM entities WHERE name = ?"
        )
        .bind(clipboard_id)
        .bind(&e.name)
        .execute(&mut *tx)
        .await?;
    }

    for rel in data.relationships {
        // Link Entity to Entity
        sqlx::query(
            "INSERT OR IGNORE INTO relationships (source_id, target_id, relationship_type)
             SELECT e1.id, e2.id, ?
             FROM entities e1, entities e2
             WHERE e1.name = ? AND e2.name = ?"
        )
        .bind(&rel.relationship_type)
        .bind(&rel.source)
        .bind(&rel.target)
        .execute(&mut *tx)
        .await?;
    }

    // Mark clipboard item as extracted
    sqlx::query("UPDATE clipboard_items SET entity_extracted = 1 WHERE id = ?")
        .bind(clipboard_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Retrieves the entire graph (or a limited sample) for visualization
pub async fn get_graph(pool: &SqlitePool) -> Result<GraphData, sqlx::Error> {
    // Get all entities
    let nodes: Vec<Entity> = sqlx::query_as(
        "SELECT id, name, entity_type, first_seen_at FROM entities"
    )
    .fetch_all(pool)
    .await?;

    // Get relationships between those entities
    // We construct GraphEdge directly
    let edges_raw: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT source_id, target_id, relationship_type FROM relationships"
    )
    .fetch_all(pool)
    .await?;

    let edges = edges_raw.into_iter().map(|(s, t, r)| GraphEdge {
        source: s,
        target: t,
        relationship: r,
    }).collect();

    Ok(GraphData { nodes, edges })
}

/// Retrieve the graph connected to a specific clipboard item
pub async fn get_item_graph(pool: &SqlitePool, clipboard_id: i64) -> Result<GraphData, sqlx::Error> {
    // Fetch directly linked entities
    let nodes: Vec<Entity> = sqlx::query_as(
        "SELECT e.id, e.name, e.entity_type, e.first_seen_at
         FROM entities e
         JOIN item_entities ie ON e.id = ie.entity_id
         WHERE ie.clipboard_id = ?"
    )
    .bind(clipboard_id)
    .fetch_all(pool)
    .await?;

    Ok(GraphData { nodes, edges: vec![] })
}
