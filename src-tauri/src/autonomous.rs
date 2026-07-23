use crate::AppState;
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

#[derive(Debug, Serialize)]
pub struct AutomationTask {
    pub id: i64,
    pub service: String,
    pub status: String,
    pub details: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct SmartNotification {
    pub id: i64,
    pub message: String,
    pub severity: String,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct IntelligenceReport {
    pub id: i64,
    pub report_type: String,
    pub title: String,
    pub summary: String,
    pub bullets: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeHealth {
    pub total_memories: i64,
    pub connected_entities: i64,
    pub graph_relationships: i64,
    pub active_projects: i64,
    pub ai_activity: i64,
    pub background_tasks: i64,
    pub unread_notifications: i64,
    pub storage_bytes: i64,
    pub storage_health: String,
    pub productivity_score: i64,
}

#[derive(Debug, Serialize)]
pub struct AutomationRunResult {
    pub tasks_run: i64,
    pub reports_created: i64,
    pub notifications_created: i64,
    pub backup_path: String,
}

pub fn start_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        sleep(Duration::from_secs(6)).await;

        loop {
            let state = app.state::<AppState>();
            let _maintenance_guard = state.maintenance_lock.lock().await;
            let _graph_guard = state.graph_lock.lock().await;
            match state.database.run_autonomous_cycle().await {
                Ok(_) => {
                    let _ = app.emit("autonomous-cycle-complete", ());
                }
                Err(err) => {
                    eprintln!("autonomous cycle failed: {err}");
                }
            }

            sleep(Duration::from_secs(15 * 60)).await;
        }
    });
}

pub fn productivity_score(total: i64, entities: i64, relationships: i64, tasks: i64) -> i64 {
    let score = (total * 3) + (entities * 2) + relationships + (tasks * 4);
    score.clamp(0, 100)
}

#[cfg(test)]
mod tests {
    use super::productivity_score;

    #[test]
    fn productivity_score_is_capped() {
        assert_eq!(productivity_score(100, 100, 100, 100), 100);
    }
}
