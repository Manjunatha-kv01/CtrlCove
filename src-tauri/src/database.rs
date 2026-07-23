use crate::agent;
use crate::autonomous;
use crate::graph;
use crate::insight_trail::{
    self, InsightIncident, InsightTrailEvent, InsightTrailOverview, InsightTrailSearchRequest,
    InsightTrailSettings, NewInsightTrailEvent,
};
use crate::operations::OperationalContext;
use crate::pcos;
use crate::platform;
use crate::privacy::{self, PrivacySettings, PrivacyStatus, TextCaptureDecision};
use crate::semantic;
use crate::workspace::{
    self, CognitiveWorkspace, HandoffRecipientTrustRecord, HandoffSignerTrustRecord,
    IncidentReopenRequest, IncidentResolution, IncidentResolutionRequest, ManualRunbookRequest,
    ManualRunbookReviewRequest, ManualRunbookRevisionRestoreRequest, ManualRunbookUpdateRequest,
    RunbookEntry, RunbookRevision, RunbookSearchRequest, WorkspaceContextUpdate,
    WorkspaceCreateRequest, WorkspaceDocumentImportRequest, WorkspaceDocumentImportResult,
    WorkspaceHandoffExportRecord, WorkspaceHandoffInspectionRecord, WorkspaceImportProvenance,
    WorkspaceSession, WorkspaceSessionStartRequest, WorkspaceSnapshot,
};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;

const MAX_AUTOMATED_BACKUPS: usize = 14;
const SQLITE_CONNECT_ATTEMPTS: u8 = 3;
const DEFAULT_VAULT_RETENTION_DAYS: i64 = 365;
const DEFAULT_VAULT_MAX_ITEMS: i64 = 10_000;
const DEFAULT_VAULT_MAX_STORAGE_MB: i64 = 1_024;
const MAX_HANDOFF_INSPECTION_RECORDS: i64 = 200;
const MAX_HANDOFF_EXPORT_RECORDS: i64 = 200;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
    data_dir: PathBuf,
    database_file: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct ClipboardItem {
    pub id: i64,
    pub content: String,
    pub content_type: String,
    pub source_application: String,
    pub created_at: String,
    pub updated_at: String,
    pub content_hash: String,
    pub character_count: i64,
    pub word_count: i64,
    pub file_size: Option<i64>,
    pub image_width: Option<i64>,
    pub image_height: Option<i64>,
    pub language: Option<String>,
    pub is_favorite: bool,
    pub collection_id: Option<i64>,
    pub collection_name: Option<String>,
    pub collection_color: Option<String>,
    pub ai_summary: String,
    pub category: String,
    pub keywords: Vec<String>,
    pub reading_time_minutes: i64,
    pub copy_count: i64,
    pub last_copied_at: String,
    pub semantic_score: f32,
    pub rank_reason: String,
    pub embedding_source: String,
    pub operational_context: OperationalContext,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub content_type: String,
    pub favorite_only: bool,
    pub collection_id: Option<i64>,
    pub tag: String,
    pub category: String,
    pub semantic: bool,
}

#[derive(Debug, Serialize)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ClipboardStats {
    pub total_items: i64,
    pub text_items: i64,
    pub image_items: i64,
    pub code_items: i64,
    pub url_items: i64,
    pub file_items: i64,
    pub storage_used: i64,
    pub favorite_items: i64,
    pub most_used_application: String,
}

#[derive(Debug, Serialize)]
pub struct DatabaseReliabilityStatus {
    pub integrity_status: String,
    pub foreign_key_issues: i64,
    pub journal_mode: String,
    pub database_bytes: i64,
    pub migration_count: i64,
    pub backup_count: i64,
    pub last_backup: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DatabaseReliabilityChecksum {
    pub integrity_status: String,
    pub snapshot_count: i64,
    pub report_data_sha256: String,
}

#[derive(Debug, Serialize)]
pub struct DatabaseReliabilityReportExport {
    pub path: String,
    pub integrity_status: String,
    pub snapshot_count: i64,
    pub report_data_sha256: String,
}

#[derive(Debug, Serialize)]
pub struct DatabaseReliabilityReportSnapshot {
    pub path: String,
    pub file_name: String,
    pub bytes: i64,
    pub modified_at_unix: i64,
}

#[derive(Debug, Serialize)]
pub struct DatabaseBackup {
    pub path: String,
    pub verified: bool,
    pub backup_count: i64,
}

#[derive(Debug, Serialize)]
pub struct DatabaseBackupSnapshot {
    pub path: String,
    pub file_name: String,
    pub bytes: i64,
    pub modified_at_unix: i64,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseBackupVerificationRequest {
    pub file_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VaultRetentionSettings {
    pub retention_days: i64,
    pub max_items: i64,
    pub max_storage_mb: i64,
    pub preserve_favorites: bool,
    pub updated_at: String,
}

impl Default for VaultRetentionSettings {
    fn default() -> Self {
        Self {
            retention_days: DEFAULT_VAULT_RETENTION_DAYS,
            max_items: DEFAULT_VAULT_MAX_ITEMS,
            max_storage_mb: DEFAULT_VAULT_MAX_STORAGE_MB,
            preserve_favorites: true,
            updated_at: String::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VaultRetentionResult {
    pub removed_items: i64,
    pub removed_images: i64,
    pub remaining_items: i64,
    pub remaining_storage_bytes: i64,
    pub protected_favorites: i64,
    pub limits_met: bool,
}

struct VaultRetentionCandidate {
    id: i64,
    content_type: String,
    storage_bytes: i64,
    is_favorite: bool,
    expired: bool,
}

fn select_retention_candidate(
    selected_ids: &mut HashSet<i64>,
    remaining_items: &mut i64,
    remaining_storage_bytes: &mut i64,
    item: &VaultRetentionCandidate,
) {
    if selected_ids.insert(item.id) {
        *remaining_items -= 1;
        *remaining_storage_bytes -= item.storage_bytes;
    }
}

pub use agent::AgentWorkflowRecord;
pub use autonomous::{
    AutomationRunResult, AutomationTask, IntelligenceReport, KnowledgeHealth, SmartNotification,
};
pub use graph::KnowledgeGraph;
pub use pcos::{
    CognitiveModule, CognitiveOverview, CognitiveReleaseResult, CognitiveUseCase, EnterpriseControl,
};
pub use platform::{
    ApiClient, AuditLog, IntegrationConnector, PlatformSummary, PluginRecord, SyncDevice,
    TeamSharingDeviceRequest, TeamSharingDeviceStatusRequest, TeamSharingManifestInspection,
    TeamSharingManifestInspectionRequest, TeamSharingManifestLedgerChecksum,
    TeamSharingManifestLedgerExportRequest, TeamSharingPolicy, TeamSharingReadiness,
    TeamSharingSyncDryRun, UniversalSyncResult,
};

pub struct NewClipboardItem {
    pub content: String,
    pub content_type: String,
    pub source_application: String,
    pub content_hash: String,
    pub character_count: i64,
    pub word_count: i64,
    pub file_size: Option<i64>,
    pub image_width: Option<i64>,
    pub image_height: Option<i64>,
    pub language: Option<String>,
    pub ai_summary: String,
    pub category: String,
    pub keywords: Vec<String>,
    pub reading_time_minutes: i64,
    pub semantic_text: String,
    pub embedding: Vec<f32>,
    pub embedding_source: String,
    pub operational_context: OperationalContext,
    pub tags: Vec<String>,
}

impl Database {
    pub async fn connect(data_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("cymos.db");
        migrate_legacy_local_data(&data_dir, &db_path)?;
        let options =
            SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.to_string_lossy()))?
                .create_if_missing(true)
                .foreign_keys(true)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal)
                .busy_timeout(Duration::from_secs(5));

        let pool = connect_sqlite_pool(options).await?;

        let database = Self {
            pool,
            data_dir,
            database_file: db_path,
        };
        database.migrate().await?;
        database.update_legacy_image_paths().await?;
        database.seed_collections().await?;
        database.seed_workspace_context().await?;
        database.seed_cognitive_workspaces().await?;
        database.seed_privacy_settings().await?;
        database.seed_vault_retention_settings().await?;
        Ok(database)
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.data_dir.join("assets")
    }

    pub fn exports_dir(&self) -> PathBuf {
        self.data_dir.join("exports")
    }

    pub fn handoff_signing_key_path(&self) -> PathBuf {
        self.data_dir.join("handoff-signing-key.bin")
    }

    pub async fn database_reliability(&self) -> Result<DatabaseReliabilityStatus, sqlx::Error> {
        let (integrity_status, foreign_key_issues) = self.check_database_integrity().await?;
        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode;")
            .fetch_one(&self.pool)
            .await?;
        let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM schema_migrations;")
            .fetch_one(&self.pool)
            .await?;
        let backups = self.backup_files()?;
        let database_bytes = fs::metadata(&self.database_file)
            .map(|metadata| metadata.len() as i64)
            .unwrap_or(0);

        Ok(DatabaseReliabilityStatus {
            integrity_status,
            foreign_key_issues,
            journal_mode: journal_mode.to_uppercase(),
            database_bytes,
            migration_count,
            backup_count: backups.len() as i64,
            last_backup: backups
                .first()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().to_string()),
        })
    }

    pub async fn create_verified_backup(&self) -> Result<DatabaseBackup, sqlx::Error> {
        let backup = self.create_backup_snapshot("manual").await?;
        self.insert_audit_log(
            "Local user",
            "Created verified database backup",
            "Local vault",
            "Info",
        )
        .await?;
        Ok(backup)
    }

    pub async fn verify_latest_backup(&self) -> Result<DatabaseBackup, sqlx::Error> {
        let backups = self.backup_files()?;
        let path = backups.first().ok_or_else(|| {
            sqlx::Error::Protocol("No local backup is available to verify.".to_string())
        })?;
        self.verify_backup_path(path, backups.len()).await
    }

    pub async fn verify_backup_snapshot(
        &self,
        file_name: &str,
    ) -> Result<DatabaseBackup, sqlx::Error> {
        if file_name.trim().is_empty()
            || file_name.contains('/')
            || file_name.contains('\\')
            || file_name == "."
            || file_name == ".."
        {
            return Err(sqlx::Error::Protocol(
                "Backup verification requires a local CYMOS backup filename.".to_string(),
            ));
        }
        let backups = self.backup_files()?;
        let path = backups
            .iter()
            .find(|path| path.file_name().and_then(|name| name.to_str()) == Some(file_name))
            .ok_or_else(|| {
                sqlx::Error::Protocol("Selected local backup was not found.".to_string())
            })?;
        self.verify_backup_path(path, backups.len()).await
    }

    async fn verify_backup_path(
        &self,
        path: &PathBuf,
        backup_count: usize,
    ) -> Result<DatabaseBackup, sqlx::Error> {
        if !verify_backup_file(path).await? {
            return Err(sqlx::Error::Protocol(
                "Backup integrity verification failed.".to_string(),
            ));
        }
        let path_string = path.to_string_lossy().to_string();
        self.insert_audit_log(
            "Local user",
            "Re-verified latest database backup",
            &path_string,
            "Info",
        )
        .await?;

        Ok(DatabaseBackup {
            path: path_string,
            verified: true,
            backup_count: backup_count as i64,
        })
    }

    pub fn recent_backup_snapshots(&self) -> Result<Vec<DatabaseBackupSnapshot>, sqlx::Error> {
        self.backup_files()?
            .into_iter()
            .take(12)
            .map(|path| {
                let metadata = fs::metadata(&path).map_err(sqlx::Error::Io)?;
                let modified_at_unix = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs() as i64)
                    .unwrap_or(0);
                Ok(DatabaseBackupSnapshot {
                    file_name: path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Unknown backup".to_string()),
                    path: path.to_string_lossy().to_string(),
                    bytes: metadata.len() as i64,
                    modified_at_unix,
                })
            })
            .collect()
    }

    pub fn recent_database_reliability_reports(
        &self,
    ) -> Result<Vec<DatabaseReliabilityReportSnapshot>, sqlx::Error> {
        let reports_dir = self.exports_dir();
        if !reports_dir.exists() {
            return Ok(Vec::new());
        }

        let mut reports = fs::read_dir(reports_dir)
            .map_err(sqlx::Error::Io)?
            .filter_map(Result::ok)
            .filter(|entry| {
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                entry
                    .file_type()
                    .map(|kind| kind.is_file())
                    .unwrap_or(false)
                    && file_name.starts_with("vault-reliability-")
                    && file_name.ends_with(".md")
            })
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        reports.sort_by(|left, right| right.file_name().cmp(&left.file_name()));

        Ok(reports
            .into_iter()
            .filter_map(|path| {
                let metadata = fs::metadata(&path).ok()?;
                let modified_at_unix = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs() as i64)
                    .unwrap_or(0);
                Some(DatabaseReliabilityReportSnapshot {
                    file_name: path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Unknown report".to_string()),
                    path: path.to_string_lossy().to_string(),
                    bytes: metadata.len() as i64,
                    modified_at_unix,
                })
            })
            .take(12)
            .collect())
    }

    pub async fn record_database_reliability_report_export(
        &self,
        path: &str,
    ) -> Result<(), sqlx::Error> {
        self.insert_audit_log(
            "Local user",
            "Exported vault reliability report",
            path,
            "Info",
        )
        .await
    }

    pub async fn privacy_settings(&self) -> Result<PrivacySettings, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                protection_enabled,
                capture_text,
                capture_images,
                block_sensitive_text,
                strftime('%Y-%m-%d %H:%M', updated_at, 'localtime') AS updated_at
            FROM privacy_settings
            WHERE id = 1;
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(PrivacySettings {
            protection_enabled: row.get::<i64, _>("protection_enabled") == 1,
            capture_text: row.get::<i64, _>("capture_text") == 1,
            capture_images: row.get::<i64, _>("capture_images") == 1,
            block_sensitive_text: row.get::<i64, _>("block_sensitive_text") == 1,
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn privacy_status(&self) -> Result<PrivacyStatus, sqlx::Error> {
        let settings = self.privacy_settings().await?;
        let blocked_capture_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM privacy_events;")
            .fetch_one(&self.pool)
            .await?;
        let last_blocked_at: Option<String> = sqlx::query_scalar(
            "SELECT strftime('%Y-%m-%d %H:%M', MAX(created_at), 'localtime') FROM privacy_events;",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(PrivacyStatus {
            settings,
            blocked_capture_count,
            last_blocked_at,
        })
    }

    pub async fn update_privacy_settings(
        &self,
        settings: &PrivacySettings,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE privacy_settings
            SET protection_enabled = ?1,
                capture_text = ?2,
                capture_images = ?3,
                block_sensitive_text = ?4,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = 1;
            "#,
        )
        .bind(settings.protection_enabled)
        .bind(settings.capture_text)
        .bind(settings.capture_images)
        .bind(settings.block_sensitive_text)
        .execute(&self.pool)
        .await?;
        self.insert_audit_log(
            "Local user",
            "Updated capture privacy controls",
            "Local vault",
            "Info",
        )
        .await?;
        Ok(())
    }

    pub async fn vault_retention_settings(&self) -> Result<VaultRetentionSettings, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                retention_days,
                max_items,
                max_storage_mb,
                preserve_favorites,
                strftime('%Y-%m-%d %H:%M', updated_at, 'localtime') AS updated_at
            FROM vault_retention_settings
            WHERE id = 1;
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_vault_retention_settings).unwrap_or_default())
    }

    pub async fn update_vault_retention_settings(
        &self,
        settings: &VaultRetentionSettings,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO vault_retention_settings (
                id,
                retention_days,
                max_items,
                max_storage_mb,
                preserve_favorites
            )
            VALUES (1, ?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                retention_days = excluded.retention_days,
                max_items = excluded.max_items,
                max_storage_mb = excluded.max_storage_mb,
                preserve_favorites = excluded.preserve_favorites,
                updated_at = CURRENT_TIMESTAMP;
            "#,
        )
        .bind(settings.retention_days)
        .bind(settings.max_items)
        .bind(settings.max_storage_mb)
        .bind(settings.preserve_favorites)
        .execute(&self.pool)
        .await?;
        self.insert_audit_log(
            "Local user",
            "Updated vault retention policy",
            "Local vault",
            "Info",
        )
        .await?;
        Ok(())
    }

    pub async fn apply_vault_retention(&self) -> Result<VaultRetentionResult, sqlx::Error> {
        let settings = self.vault_retention_settings().await?;
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                content_type,
                CASE
                    WHEN content_type = 'Image' THEN COALESCE(file_size, 0)
                    ELSE length(CAST(content AS BLOB))
                END AS storage_bytes,
                is_favorite,
                last_copied_at < datetime('now', ?1) AS expired
            FROM clipboard_items
            ORDER BY last_copied_at ASC, id ASC;
            "#,
        )
        .bind(format!("-{} days", settings.retention_days))
        .fetch_all(&self.pool)
        .await?;

        let candidates = rows
            .into_iter()
            .map(|row| VaultRetentionCandidate {
                id: row.get("id"),
                content_type: row.get("content_type"),
                storage_bytes: row.get::<i64, _>("storage_bytes").max(0),
                is_favorite: row.get::<i64, _>("is_favorite") == 1,
                expired: row.get::<i64, _>("expired") == 1,
            })
            .collect::<Vec<_>>();

        let protected_favorites = candidates.iter().filter(|item| item.is_favorite).count() as i64;
        let mut selected_ids = HashSet::new();
        let mut remaining_items = candidates.len() as i64;
        let mut remaining_storage_bytes = candidates
            .iter()
            .map(|item| item.storage_bytes)
            .sum::<i64>();

        for item in &candidates {
            if item.expired && (!settings.preserve_favorites || !item.is_favorite) {
                select_retention_candidate(
                    &mut selected_ids,
                    &mut remaining_items,
                    &mut remaining_storage_bytes,
                    item,
                );
            }
        }

        for item in &candidates {
            if remaining_items <= settings.max_items {
                break;
            }
            if !selected_ids.contains(&item.id)
                && (!settings.preserve_favorites || !item.is_favorite)
            {
                select_retention_candidate(
                    &mut selected_ids,
                    &mut remaining_items,
                    &mut remaining_storage_bytes,
                    item,
                );
            }
        }

        let max_storage_bytes = settings.max_storage_mb.saturating_mul(1024 * 1024);
        for item in &candidates {
            if remaining_storage_bytes <= max_storage_bytes {
                break;
            }
            if !selected_ids.contains(&item.id)
                && (!settings.preserve_favorites || !item.is_favorite)
            {
                select_retention_candidate(
                    &mut selected_ids,
                    &mut remaining_items,
                    &mut remaining_storage_bytes,
                    item,
                );
            }
        }

        let removed = candidates
            .iter()
            .filter(|item| selected_ids.contains(&item.id))
            .collect::<Vec<_>>();
        let removed_images = removed
            .iter()
            .filter(|item| item.content_type == "Image")
            .count() as i64;
        let removed_items = self
            .delete_items_by_ids(&removed.iter().map(|item| item.id).collect::<Vec<_>>())
            .await?;

        if removed_items > 0 {
            self.insert_audit_log(
                "Vault maintenance",
                "Applied vault retention policy",
                &format!("{removed_items} memories removed"),
                "Info",
            )
            .await?;
        }

        Ok(VaultRetentionResult {
            removed_items,
            removed_images,
            remaining_items,
            remaining_storage_bytes,
            protected_favorites,
            limits_met: remaining_items <= settings.max_items
                && remaining_storage_bytes <= max_storage_bytes,
        })
    }

    pub async fn record_privacy_block(
        &self,
        source: &str,
        reason: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO privacy_events (source, reason) VALUES (?1, ?2);")
            .bind(source)
            .bind(reason)
            .execute(&self.pool)
            .await?;
        self.insert_audit_log(
            "Capture guard",
            "Blocked sensitive capture",
            source,
            "Warning",
        )
        .await?;
        Ok(())
    }

    async fn update_legacy_image_paths(&self) -> Result<(), sqlx::Error> {
        let legacy_assets = legacy_data_dir().join("assets");
        sqlx::query(
            "UPDATE clipboard_items SET content = REPLACE(content, ?1, ?2) WHERE content_type = 'Image' AND content LIKE ?3;",
        )
        .bind(legacy_assets.to_string_lossy().to_string())
        .bind(self.assets_dir().to_string_lossy().to_string())
        .bind(format!("{}%", legacy_assets.to_string_lossy()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_item(&self, item: NewClipboardItem) -> Result<Option<i64>, sqlx::Error> {
        if item.content.trim().is_empty() {
            return Ok(None);
        }

        if item.content_type != "Image" {
            let settings = self.privacy_settings().await?;
            match privacy::text_capture_decision(&settings, &item.content) {
                TextCaptureDecision::Allow => {}
                TextCaptureDecision::Skip => return Ok(None),
                TextCaptureDecision::Block(reason) => {
                    self.record_privacy_block(&item.source_application, &reason)
                        .await?;
                    return Ok(None);
                }
            }
        }

        let result = sqlx::query(
            r#"
            INSERT INTO clipboard_items (
                content,
                content_type,
                source_application,
                content_hash,
                character_count,
                word_count,
                file_size,
                image_width,
                image_height,
                language,
                ai_summary,
                category,
                keywords,
                reading_time_minutes,
                semantic_text,
                embedding,
                embedding_source,
                operational_context
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT DO NOTHING;
            "#,
        )
        .bind(&item.content)
        .bind(&item.content_type)
        .bind(&item.source_application)
        .bind(&item.content_hash)
        .bind(item.character_count)
        .bind(item.word_count)
        .bind(item.file_size)
        .bind(item.image_width)
        .bind(item.image_height)
        .bind(&item.language)
        .bind(&item.ai_summary)
        .bind(&item.category)
        .bind(item.keywords.join(","))
        .bind(item.reading_time_minutes)
        .bind(&item.semantic_text)
        .bind(semantic::serialize_embedding(&item.embedding))
        .bind(&item.embedding_source)
        .bind(serde_json::to_string(&item.operational_context).unwrap_or_else(|_| "{}".to_string()))
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            self.record_duplicate_by_hash(&item.content_hash).await?;
            return Ok(None);
        }

        let clipboard_id = result.last_insert_rowid();
        let mut all_tags = item.tags.clone();
        all_tags.extend(item.keywords.clone());
        all_tags.push(item.category.clone());
        self.set_tags(clipboard_id, all_tags).await?;
        self.index_graph_for_item(clipboard_id, &item).await?;
        if let Err(error) = self
            .record_insight_event_from_memory(clipboard_id, &item)
            .await
        {
            eprintln!("failed to create InsightTrail event: {error}");
        }
        Ok(Some(clipboard_id))
    }

    pub async fn search_items(
        &self,
        request: &SearchRequest,
    ) -> Result<Vec<ClipboardItem>, sqlx::Error> {
        let query_text = request.query.trim().to_string();
        let query_embedding = if request.semantic && !query_text.is_empty() {
            Some(semantic::embed(&query_text).await.values)
        } else {
            None
        };

        let mut builder = QueryBuilder::<Sqlite>::new(
            r#"
            SELECT
                c.id,
                c.content,
                c.content_type,
                c.source_application,
                strftime('%Y-%m-%d %H:%M', c.created_at, 'localtime') AS created_at,
                strftime('%Y-%m-%d %H:%M', c.updated_at, 'localtime') AS updated_at,
                c.content_hash,
                c.character_count,
                c.word_count,
                c.file_size,
                c.image_width,
                c.image_height,
                c.language,
                c.is_favorite,
                c.collection_id,
                collections.name AS collection_name,
                collections.color AS collection_color,
                c.ai_summary,
                c.category,
                c.keywords,
                c.reading_time_minutes,
                c.copy_count,
                COALESCE(strftime('%Y-%m-%d %H:%M', c.last_copied_at, 'localtime'), c.created_at) AS last_copied_at,
                c.semantic_text,
                c.embedding,
                c.embedding_source,
                c.operational_context,
                COALESCE(GROUP_CONCAT(tags.name, ','), '') AS tags
            FROM clipboard_items c
            LEFT JOIN collections ON collections.id = c.collection_id
            LEFT JOIN clipboard_tags ON clipboard_tags.clipboard_id = c.id
            LEFT JOIN tags ON tags.id = clipboard_tags.tag_id
            "#,
        );

        let mut needs_where = true;

        if !request.semantic && !query_text.is_empty() {
            push_filter(&mut builder, &mut needs_where, "(c.content LIKE ");
            let like_query = format!("%{}%", query_text);
            builder.push_bind(like_query);
            builder.push(" OR c.ai_summary LIKE ");
            builder.push_bind(format!("%{}%", query_text));
            builder.push(" OR c.keywords LIKE ");
            builder.push_bind(format!("%{}%", query_text));
            builder.push(")");
        }

        if request.content_type != "All" {
            push_filter(&mut builder, &mut needs_where, "c.content_type = ");
            builder.push_bind(&request.content_type);
        }

        if request.favorite_only {
            push_filter(&mut builder, &mut needs_where, "c.is_favorite = 1");
        }

        if let Some(collection_id) = request.collection_id {
            push_filter(&mut builder, &mut needs_where, "c.collection_id = ");
            builder.push_bind(collection_id);
        }

        if !request.tag.trim().is_empty() && request.tag != "All" {
            push_filter(&mut builder, &mut needs_where, "tags.name = ");
            builder.push_bind(request.tag.trim());
        }

        if !request.category.trim().is_empty() && request.category != "All" {
            push_filter(&mut builder, &mut needs_where, "c.category = ");
            builder.push_bind(request.category.trim());
        }

        builder.push(" GROUP BY c.id ORDER BY c.id DESC LIMIT 500;");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut items = rows
            .into_iter()
            .map(|row| row_to_item(row, query_embedding.as_deref(), &query_text))
            .collect::<Vec<_>>();

        if !query_text.is_empty() {
            items.sort_by(|left, right| {
                right
                    .semantic_score
                    .partial_cmp(&left.semantic_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| right.id.cmp(&left.id))
            });
            items.truncate(100);
        }

        Ok(items)
    }

    pub async fn collections(&self) -> Result<Vec<Collection>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, color, strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
            FROM collections
            ORDER BY name;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Collection {
                id: row.get("id"),
                name: row.get("name"),
                color: row.get("color"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    pub async fn similar_items(&self, item_id: i64) -> Result<Vec<ClipboardItem>, sqlx::Error> {
        let source_embedding = self.embedding_for_item(item_id).await?;
        if source_embedding.is_empty() {
            return Ok(Vec::new());
        }

        let request = SearchRequest {
            query: String::new(),
            content_type: "All".to_string(),
            favorite_only: false,
            collection_id: None,
            tag: "All".to_string(),
            category: "All".to_string(),
            semantic: true,
        };

        let mut items = self.search_items(&request).await?;
        for item in &mut items {
            if item.id == item_id {
                item.semantic_score = -1.0;
                continue;
            }

            let embedding = self.embedding_for_item(item.id).await?;
            item.semantic_score = semantic::cosine_similarity(&source_embedding, &embedding);
            item.rank_reason = "Similar vector memory".to_string();
        }

        items.retain(|item| item.id != item_id);
        items.sort_by(|left, right| {
            right
                .semantic_score
                .partial_cmp(&left.semantic_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        items.truncate(6);
        Ok(items)
    }

    pub async fn rebuild_semantic_index(&self) -> Result<(), sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, content, content_type, language, ai_summary, category, keywords
            FROM clipboard_items;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        for row in rows {
            let id: i64 = row.get("id");
            let content: String = row.get("content");
            let content_type: String = row.get("content_type");
            let language: Option<String> = row.get("language");
            let summary: String = row.get("ai_summary");
            let category: String = row.get("category");
            let keyword_csv: String = row.get("keywords");
            let keywords = keyword_csv
                .split(',')
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            let semantic_text = semantic::semantic_text(
                &content,
                &content_type,
                language.as_deref(),
                &summary,
                &category,
                &keywords,
                &[],
            );
            let embedding = semantic::embed(&semantic_text).await;

            sqlx::query(
                r#"
                UPDATE clipboard_items
                SET semantic_text = ?1,
                    embedding = ?2,
                    embedding_source = ?3,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = ?4;
                "#,
            )
            .bind(semantic_text)
            .bind(semantic::serialize_embedding(&embedding.values))
            .bind(embedding.source)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn knowledge_graph(&self) -> Result<KnowledgeGraph, sqlx::Error> {
        let node_rows = sqlx::query(
            r#"
            SELECT id, name, entity_type, weight, cluster
            FROM graph_entities
            ORDER BY weight DESC, name
            LIMIT 80;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let nodes = node_rows
            .into_iter()
            .map(|row| graph::GraphNode {
                id: row.get("id"),
                name: row.get("name"),
                entity_type: row.get("entity_type"),
                weight: row.get("weight"),
                cluster: row.get("cluster"),
            })
            .collect::<Vec<_>>();

        let edge_rows = sqlx::query(
            r#"
            SELECT source_entity_id, target_entity_id, relationship, weight
            FROM graph_relationships
            ORDER BY weight DESC
            LIMIT 160;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let edges = edge_rows
            .into_iter()
            .map(|row| graph::GraphEdge {
                source: row.get("source_entity_id"),
                target: row.get("target_entity_id"),
                relationship: row.get("relationship"),
                weight: row.get("weight"),
            })
            .collect::<Vec<_>>();

        let clusters = graph::clusters_from_nodes(&nodes);
        let recommendations = graph::recommendations(&nodes, &clusters);

        Ok(KnowledgeGraph {
            nodes,
            edges,
            clusters,
            recommendations,
        })
    }

    pub async fn rebuild_knowledge_graph(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM memory_entities;")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM graph_relationships;")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM graph_entities;")
            .execute(&self.pool)
            .await?;

        let rows = sqlx::query(
            r#"
            SELECT id, content, content_type, language, category, keywords
            FROM clipboard_items
            ORDER BY id;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        for row in rows {
            let id: i64 = row.get("id");
            let content: String = row.get("content");
            let content_type: String = row.get("content_type");
            let language: Option<String> = row.get("language");
            let category: String = row.get("category");
            let keyword_csv: String = row.get("keywords");
            let keywords = keyword_csv
                .split(',')
                .filter(|keyword| !keyword.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            let tags = self.tags_for_item(id).await?;
            let item = NewClipboardItem {
                content,
                content_type,
                source_application: "Unknown".to_string(),
                content_hash: String::new(),
                character_count: 0,
                word_count: 0,
                file_size: None,
                image_width: None,
                image_height: None,
                language,
                ai_summary: String::new(),
                category,
                keywords,
                reading_time_minutes: 1,
                semantic_text: String::new(),
                embedding: Vec::new(),
                embedding_source: "Local".to_string(),
                operational_context: OperationalContext::default(),
                tags,
            };
            self.index_graph_for_item(id, &item).await?;
        }

        Ok(())
    }

    pub async fn insight_trail_events(
        &self,
        request: &InsightTrailSearchRequest,
    ) -> Result<Vec<InsightTrailEvent>, sqlx::Error> {
        let mut builder = QueryBuilder::<Sqlite>::new(
            r#"
            SELECT
                id,
                event_type,
                title,
                details,
                source_application,
                severity,
                strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at,
                memory_id,
                screenshot_path,
                incident_id,
                NULL AS session_id,
                tags
            FROM insight_trail_events
            "#,
        );
        let mut needs_where = true;
        let query = request.query.trim();

        if !query.is_empty() {
            push_filter(&mut builder, &mut needs_where, "(title LIKE ");
            builder.push_bind(format!("%{query}%"));
            builder.push(" OR details LIKE ");
            builder.push_bind(format!("%{query}%"));
            builder.push(" OR tags LIKE ");
            builder.push_bind(format!("%{query}%"));
            builder.push(" OR source_application LIKE ");
            builder.push_bind(format!("%{query}%"));
            builder.push(")");
        }

        if request.event_type != "All" {
            push_filter(&mut builder, &mut needs_where, "event_type = ");
            builder.push_bind(request.event_type.trim());
        }

        builder.push(" ORDER BY created_at DESC, id DESC LIMIT ");
        builder.push_bind(request.limit);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_insight_event).collect())
    }

    pub async fn insight_trail_settings(&self) -> Result<InsightTrailSettings, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                enabled,
                capture_clipboard,
                capture_terminal_history,
                capture_copied_images,
                create_incidents,
                retention_days,
                max_storage_mb,
                excluded_applications,
                strftime('%Y-%m-%d %H:%M', updated_at, 'localtime') AS updated_at
            FROM insight_trail_settings
            WHERE id = 1;
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_insight_settings).unwrap_or_default())
    }

    pub async fn update_insight_trail_settings(
        &self,
        settings: &InsightTrailSettings,
    ) -> Result<(), sqlx::Error> {
        let excluded_applications = insight_trail::clean_tags(&settings.excluded_applications);
        sqlx::query(
            r#"
            INSERT INTO insight_trail_settings (
                id,
                enabled,
                capture_clipboard,
                capture_terminal_history,
                capture_copied_images,
                create_incidents,
                retention_days,
                max_storage_mb,
                excluded_applications
            )
            VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                enabled = excluded.enabled,
                capture_clipboard = excluded.capture_clipboard,
                capture_terminal_history = excluded.capture_terminal_history,
                capture_copied_images = excluded.capture_copied_images,
                create_incidents = excluded.create_incidents,
                retention_days = excluded.retention_days,
                max_storage_mb = excluded.max_storage_mb,
                excluded_applications = excluded.excluded_applications,
                updated_at = CURRENT_TIMESTAMP;
            "#,
        )
        .bind(settings.enabled)
        .bind(settings.capture_clipboard)
        .bind(settings.capture_terminal_history)
        .bind(settings.capture_copied_images)
        .bind(settings.create_incidents)
        .bind(settings.retention_days)
        .bind(settings.max_storage_mb)
        .bind(serde_json::to_string(&excluded_applications).unwrap_or_else(|_| "[]".to_string()))
        .execute(&self.pool)
        .await?;
        self.prune_insight_trail_events(settings.retention_days)
            .await?;
        Ok(())
    }

    pub async fn insight_trail_overview(&self) -> Result<InsightTrailOverview, sqlx::Error> {
        let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM insight_trail_events;")
            .fetch_one(&self.pool)
            .await?;
        let active_incident_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM insight_incidents WHERE status = 'Open';")
                .fetch_one(&self.pool)
                .await?;
        let screenshot_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM insight_trail_events WHERE event_type = 'Screenshot';",
        )
        .fetch_one(&self.pool)
        .await?;
        let error_signal_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM insight_trail_events WHERE event_type = 'Error';",
        )
        .fetch_one(&self.pool)
        .await?;
        let settings = self.insight_trail_settings().await?;

        Ok(InsightTrailOverview {
            event_count,
            active_incident_count,
            screenshot_count,
            error_signal_count,
            capture_state: if settings.enabled {
                "Active".to_string()
            } else {
                "Paused".to_string()
            },
            retention_days: settings.retention_days,
        })
    }

    pub async fn insight_incidents(&self) -> Result<Vec<InsightIncident>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                title,
                status,
                summary,
                strftime('%Y-%m-%d %H:%M', first_seen_at, 'localtime') AS first_seen_at,
                strftime('%Y-%m-%d %H:%M', last_seen_at, 'localtime') AS last_seen_at,
                event_count,
                recommended_steps
            FROM insight_incidents
            ORDER BY CASE status WHEN 'Open' THEN 0 ELSE 1 END, last_seen_at DESC, id DESC
            LIMIT 24;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_insight_incident).collect())
    }

    pub async fn record_insight_trail_note(
        &self,
        title: &str,
        details: &str,
        tags: &[String],
    ) -> Result<(), sqlx::Error> {
        let event = insight_trail::manual_note(title, details, tags);
        self.insert_insight_event(&event).await?;
        Ok(())
    }

    pub async fn resolve_insight_incident(&self, incident_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE insight_incidents
            SET status = 'Resolved', resolved_at = CURRENT_TIMESTAMP, last_seen_at = CURRENT_TIMESTAMP
            WHERE id = ?1;
            "#,
        )
        .bind(incident_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn prune_insight_trail(&self) -> Result<i64, sqlx::Error> {
        let settings = self.insight_trail_settings().await?;
        self.prune_insight_trail_events(settings.retention_days)
            .await
    }

    pub async fn cognitive_workspaces(&self) -> Result<Vec<CognitiveWorkspace>, sqlx::Error> {
        let ids = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM cognitive_workspaces ORDER BY updated_at DESC, id DESC;",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut workspaces = Vec::with_capacity(ids.len());
        for id in ids {
            workspaces.push(self.cognitive_workspace(Some(id)).await?);
        }
        Ok(workspaces)
    }

    pub async fn cognitive_workspace(
        &self,
        workspace_id: Option<i64>,
    ) -> Result<CognitiveWorkspace, sqlx::Error> {
        let row = if let Some(workspace_id) = workspace_id {
            sqlx::query(
                r#"
                SELECT id, name, project, status, is_imported,
                    strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at,
                    strftime('%Y-%m-%d %H:%M', updated_at, 'localtime') AS updated_at
                FROM cognitive_workspaces WHERE id = ?1;
                "#,
            )
            .bind(workspace_id)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT cw.id, cw.name, cw.project, cw.status, cw.is_imported,
                    strftime('%Y-%m-%d %H:%M', cw.created_at, 'localtime') AS created_at,
                    strftime('%Y-%m-%d %H:%M', cw.updated_at, 'localtime') AS updated_at
                FROM cognitive_workspaces cw
                LEFT JOIN workspace_sessions ws ON ws.workspace_id = cw.id AND ws.status = 'Active'
                WHERE cw.status <> 'Archived'
                ORDER BY CASE WHEN ws.id IS NULL THEN 1 ELSE 0 END, cw.updated_at DESC, cw.id DESC
                LIMIT 1;
                "#,
            )
            .fetch_one(&self.pool)
            .await?
        };
        let workspace_id: i64 = row.get("id");
        let event_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM workspace_event_links WHERE workspace_id = ?1;",
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await?;
        let memory_count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM workspace_event_links wel
               JOIN insight_trail_events ite ON ite.id = wel.insight_event_id
               WHERE wel.workspace_id = ?1 AND ite.memory_id IS NOT NULL;"#,
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await?;
        let error_count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM workspace_event_links wel
               JOIN insight_trail_events ite ON ite.id = wel.insight_event_id
               WHERE wel.workspace_id = ?1 AND ite.event_type = 'Error';"#,
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await?;
        let last_event_at: Option<String> = sqlx::query_scalar(
            r#"SELECT strftime('%Y-%m-%d %H:%M', MAX(ite.created_at), 'localtime')
               FROM workspace_event_links wel
               JOIN insight_trail_events ite ON ite.id = wel.insight_event_id
               WHERE wel.workspace_id = ?1;"#,
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await?;
        let source_rows = sqlx::query(
            r#"SELECT DISTINCT ite.event_type FROM workspace_event_links wel
               JOIN insight_trail_events ite ON ite.id = wel.insight_event_id
               WHERE wel.workspace_id = ?1 ORDER BY ite.event_type;"#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;
        let sources = source_rows
            .into_iter()
            .map(|source| source.get::<String, _>("event_type"))
            .collect::<Vec<_>>();
        let tag_rows = sqlx::query(
            r#"SELECT ite.tags FROM workspace_event_links wel
               JOIN insight_trail_events ite ON ite.id = wel.insight_event_id
               WHERE wel.workspace_id = ?1 AND ite.tags <> ''
               ORDER BY ite.created_at DESC LIMIT 200;"#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;
        let mut topic_counts = HashMap::<String, i64>::new();
        for row in tag_rows {
            let tags: String = row.get("tags");
            for tag in tags.split(',').map(str::trim).filter(|tag| !tag.is_empty()) {
                *topic_counts.entry(tag.to_string()).or_default() += 1;
            }
        }
        let mut top_topics = topic_counts.into_iter().collect::<Vec<_>>();
        top_topics.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        top_topics.truncate(6);
        let top_topics = top_topics
            .into_iter()
            .map(|(topic, _)| topic)
            .collect::<Vec<_>>();
        let name: String = row.get("name");
        let project: String = row.get("project");

        Ok(CognitiveWorkspace {
            id: workspace_id,
            name,
            project: project.clone(),
            status: row.get("status"),
            is_imported: row.get::<i64, _>("is_imported") == 1,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            last_event_at,
            event_count,
            memory_count,
            error_count,
            sources,
            next_signal: workspace::next_signal(error_count, event_count, &top_topics),
            summary: workspace::summary(event_count, memory_count, error_count, &project),
            top_topics,
        })
    }

    pub async fn workspace_snapshot(
        &self,
        workspace_id: Option<i64>,
    ) -> Result<WorkspaceSnapshot, sqlx::Error> {
        let workspace = self.cognitive_workspace(workspace_id).await?;
        let sessions = self.workspace_sessions(workspace.id).await?;
        let active_session = sessions
            .iter()
            .find(|session| session.status == "Active")
            .cloned();
        let events = self.workspace_events(workspace.id).await?;
        let incidents = self.workspace_incidents(workspace.id).await?;
        let resolutions = self.workspace_resolutions(workspace.id).await?;
        let import_provenance = self.workspace_import_provenance(workspace.id).await?;
        Ok(WorkspaceSnapshot {
            workspace,
            sessions,
            active_session,
            events,
            incidents,
            resolutions,
            import_provenance,
        })
    }

    pub async fn workspace_handoff_exports(
        &self,
        workspace_id: i64,
    ) -> Result<Vec<WorkspaceHandoffExportRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT id, workspace_id, session_id, scope, recipient, purpose, classification,
                      expires_at_unix, signer_fingerprint,
                      package_sha256, package_bytes,
                      event_count, excluded_event_count, incident_count, resolution_count,
                      strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
               FROM workspace_handoff_exports
               WHERE workspace_id = ?1
               ORDER BY id DESC
               LIMIT 12;"#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(row_to_workspace_handoff_export)
            .collect())
    }

    pub async fn handoff_recipient_trust_records(
        &self,
    ) -> Result<Vec<HandoffRecipientTrustRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT id, recipient, max_classification, note, is_active, export_count,
                      strftime('%Y-%m-%d %H:%M', last_used_at, 'localtime') AS last_used_at,
                      strftime('%Y-%m-%d %H:%M', revoked_at, 'localtime') AS revoked_at,
                      strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
               FROM handoff_recipient_trust
               ORDER BY is_active DESC, recipient COLLATE NOCASE ASC;"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(row_to_handoff_recipient_trust)
            .collect())
    }

    pub async fn trusted_handoff_recipient(
        &self,
        recipient: &str,
    ) -> Result<Option<HandoffRecipientTrustRecord>, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT id, recipient, max_classification, note, is_active, export_count,
                      strftime('%Y-%m-%d %H:%M', last_used_at, 'localtime') AS last_used_at,
                      strftime('%Y-%m-%d %H:%M', revoked_at, 'localtime') AS revoked_at,
                      strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
               FROM handoff_recipient_trust
               WHERE lower(recipient) = lower(?1)
                 AND is_active = 1
               LIMIT 1;"#,
        )
        .bind(recipient.trim())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_handoff_recipient_trust))
    }

    pub async fn trust_handoff_recipient(
        &self,
        recipient: &str,
        max_classification: &str,
        note: &str,
    ) -> Result<HandoffRecipientTrustRecord, sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO handoff_recipient_trust (recipient, max_classification, note)
               VALUES (?1, ?2, ?3)
               ON CONFLICT(recipient) DO UPDATE SET
                   max_classification = excluded.max_classification,
                   note = excluded.note,
                   is_active = 1,
                   revoked_at = NULL;"#,
        )
        .bind(recipient.trim())
        .bind(max_classification)
        .bind(note.trim())
        .execute(&self.pool)
        .await?;
        self.insert_audit_log(
            "local-user",
            "workspace.handoff_recipient_trusted",
            recipient.trim(),
            "Info",
        )
        .await?;
        Ok(self
            .trusted_handoff_recipient(recipient)
            .await?
            .expect("trusted recipient should exist after upsert"))
    }

    pub async fn revoke_handoff_recipient(&self, recipient: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE handoff_recipient_trust
               SET is_active = 0, revoked_at = CURRENT_TIMESTAMP
               WHERE lower(recipient) = lower(?1);"#,
        )
        .bind(recipient.trim())
        .execute(&self.pool)
        .await?;
        self.insert_audit_log(
            "local-user",
            "workspace.handoff_recipient_revoked",
            recipient.trim(),
            "Warning",
        )
        .await
    }

    pub async fn handoff_signer_trust_records(
        &self,
    ) -> Result<Vec<HandoffSignerTrustRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT id, signer_fingerprint, label, is_active, import_count,
                      strftime('%Y-%m-%d %H:%M', last_used_at, 'localtime') AS last_used_at,
                      strftime('%Y-%m-%d %H:%M', revoked_at, 'localtime') AS revoked_at,
                      strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
               FROM handoff_signer_trust
               ORDER BY is_active DESC, signer_fingerprint ASC;"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_handoff_signer_trust).collect())
    }

    pub async fn trusted_handoff_signer(
        &self,
        signer_fingerprint: &str,
    ) -> Result<Option<HandoffSignerTrustRecord>, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT id, signer_fingerprint, label, is_active, import_count,
                      strftime('%Y-%m-%d %H:%M', last_used_at, 'localtime') AS last_used_at,
                      strftime('%Y-%m-%d %H:%M', revoked_at, 'localtime') AS revoked_at,
                      strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
               FROM handoff_signer_trust
               WHERE signer_fingerprint = ?1
                 AND is_active = 1
               LIMIT 1;"#,
        )
        .bind(signer_fingerprint.trim())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_handoff_signer_trust))
    }

    pub async fn trust_handoff_signer(
        &self,
        signer_fingerprint: &str,
        label: &str,
    ) -> Result<HandoffSignerTrustRecord, sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO handoff_signer_trust (signer_fingerprint, label)
               VALUES (?1, ?2)
               ON CONFLICT(signer_fingerprint) DO UPDATE SET
                   label = excluded.label,
                   is_active = 1,
                   revoked_at = NULL;"#,
        )
        .bind(signer_fingerprint.trim())
        .bind(label.trim())
        .execute(&self.pool)
        .await?;
        self.insert_audit_log(
            "local-user",
            "workspace.handoff_signer_trusted",
            signer_fingerprint.trim(),
            "Info",
        )
        .await?;
        Ok(self
            .trusted_handoff_signer(signer_fingerprint)
            .await?
            .expect("trusted signer should exist after upsert"))
    }

    pub async fn revoke_handoff_signer(&self, signer_fingerprint: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE handoff_signer_trust
               SET is_active = 0, revoked_at = CURRENT_TIMESTAMP
               WHERE signer_fingerprint = ?1;"#,
        )
        .bind(signer_fingerprint.trim())
        .execute(&self.pool)
        .await?;
        self.insert_audit_log(
            "local-user",
            "workspace.handoff_signer_revoked",
            signer_fingerprint.trim(),
            "Warning",
        )
        .await
    }

    async fn mark_handoff_signer_used(&self, signer_fingerprint: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE handoff_signer_trust
               SET import_count = import_count + 1, last_used_at = CURRENT_TIMESTAMP
               WHERE signer_fingerprint = ?1;"#,
        )
        .bind(signer_fingerprint.trim())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_handoff_recipient_used(&self, recipient: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE handoff_recipient_trust
               SET export_count = export_count + 1, last_used_at = CURRENT_TIMESTAMP
               WHERE lower(recipient) = lower(?1);"#,
        )
        .bind(recipient.trim())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn workspace_handoff_inspection_records(
        &self,
    ) -> Result<Vec<WorkspaceHandoffInspectionRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT id, status, workspace_name, classification, signer_fingerprint,
                      package_sha256, payload_sha256, failure_reason, package_bytes,
                      strftime('%Y-%m-%d %H:%M', inspected_at, 'localtime') AS inspected_at
               FROM workspace_handoff_inspections
               ORDER BY id DESC
               LIMIT 12;"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(row_to_workspace_handoff_inspection)
            .collect())
    }

    pub async fn record_workspace_handoff_inspection(
        &self,
        status: &str,
        workspace_name: Option<&str>,
        classification: Option<&str>,
        signer_fingerprint: Option<&str>,
        package_sha256: &str,
        payload_sha256: Option<&str>,
        failure_reason: Option<&str>,
        package_bytes: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO workspace_handoff_inspections (
                   status, workspace_name, classification, signer_fingerprint,
                   package_sha256, payload_sha256, failure_reason, package_bytes
               ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);"#,
        )
        .bind(status)
        .bind(workspace_name)
        .bind(classification)
        .bind(signer_fingerprint)
        .bind(package_sha256)
        .bind(payload_sha256)
        .bind(failure_reason)
        .bind(package_bytes)
        .execute(&self.pool)
        .await?;
        self.prune_workspace_handoff_inspections().await?;
        self.insert_audit_log(
            "local-user",
            "workspace.handoff_inspected",
            status,
            if status == "Rejected" {
                "Warning"
            } else {
                "Info"
            },
        )
        .await
    }

    async fn prune_workspace_handoff_inspections(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM workspace_handoff_inspections
               WHERE id NOT IN (
                   SELECT id FROM workspace_handoff_inspections
                   ORDER BY id DESC
                   LIMIT ?1
               );"#,
        )
        .bind(MAX_HANDOFF_INSPECTION_RECORDS)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn record_workspace_handoff_export(
        &self,
        workspace_id: i64,
        session_id: Option<i64>,
        scope: &str,
        recipient: &str,
        purpose: &str,
        classification: &str,
        expires_at_unix: Option<i64>,
        signer_fingerprint: &str,
        package_sha256: &str,
        package_bytes: i64,
        event_count: i64,
        excluded_event_count: i64,
        incident_count: i64,
        resolution_count: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO workspace_handoff_exports (
                   workspace_id, session_id, scope, recipient, purpose, classification,
                   expires_at_unix, signer_fingerprint, package_sha256, package_bytes,
                   event_count, excluded_event_count, incident_count, resolution_count
               ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14);"#,
        )
        .bind(workspace_id)
        .bind(session_id)
        .bind(scope)
        .bind(recipient)
        .bind(purpose)
        .bind(classification)
        .bind(expires_at_unix)
        .bind(signer_fingerprint)
        .bind(package_sha256)
        .bind(package_bytes)
        .bind(event_count)
        .bind(excluded_event_count)
        .bind(incident_count)
        .bind(resolution_count)
        .execute(&self.pool)
        .await?;
        self.prune_workspace_handoff_exports().await?;
        self.mark_handoff_recipient_used(recipient).await?;
        self.insert_audit_log(
            "local-user",
            "workspace.handoff_exported",
            &format!("workspace:{workspace_id}"),
            "Info",
        )
        .await
    }

    async fn prune_workspace_handoff_exports(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM workspace_handoff_exports
               WHERE id NOT IN (
                   SELECT id FROM workspace_handoff_exports
                   ORDER BY id DESC
                   LIMIT ?1
               );"#,
        )
        .bind(MAX_HANDOFF_EXPORT_RECORDS)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn create_cognitive_workspace(
        &self,
        request: &WorkspaceCreateRequest,
    ) -> Result<WorkspaceSnapshot, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO cognitive_workspaces (name, project, status) VALUES (?1, ?2, 'Ready');",
        )
        .bind(request.name.trim())
        .bind(request.project.trim())
        .execute(&self.pool)
        .await?;
        let workspace_id = result.last_insert_rowid();
        self.insert_audit_log(
            "local-user",
            "workspace.created",
            request.name.trim(),
            "Info",
        )
        .await?;
        self.workspace_snapshot(Some(workspace_id)).await
    }

    pub async fn update_cognitive_workspace(
        &self,
        workspace_id: i64,
        update: &WorkspaceContextUpdate,
    ) -> Result<WorkspaceSnapshot, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE cognitive_workspaces SET name = ?1, project = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3 AND is_imported = 0;",
        )
        .bind(update.name.trim())
        .bind(update.project.trim())
        .bind(workspace_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        self.insert_audit_log(
            "local-user",
            "workspace.updated",
            update.name.trim(),
            "Info",
        )
        .await?;
        self.workspace_snapshot(Some(workspace_id)).await
    }

    pub async fn start_workspace_session(
        &self,
        request: &WorkspaceSessionStartRequest,
    ) -> Result<WorkspaceSnapshot, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        let workspace =
            sqlx::query("SELECT status, is_imported FROM cognitive_workspaces WHERE id = ?1;")
                .bind(request.workspace_id)
                .fetch_one(&mut *transaction)
                .await?;
        if workspace.get::<String, _>("status") == "Archived"
            || workspace.get::<i64, _>("is_imported") == 1
        {
            return Err(sqlx::Error::RowNotFound);
        }
        sqlx::query(
            "UPDATE workspace_sessions SET status = 'Completed', ended_at = CURRENT_TIMESTAMP WHERE status = 'Active';",
        )
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"INSERT INTO workspace_sessions (workspace_id, title, status)
               VALUES (?1, ?2, 'Active');"#,
        )
        .bind(request.workspace_id)
        .bind(request.title.trim())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE cognitive_workspaces SET status = 'Active', updated_at = CURRENT_TIMESTAMP WHERE id = ?1;",
        )
        .bind(request.workspace_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.insert_audit_log(
            "local-user",
            "workspace.session_started",
            request.title.trim(),
            "Info",
        )
        .await?;
        self.workspace_snapshot(Some(request.workspace_id)).await
    }

    pub async fn end_workspace_session(
        &self,
        session_id: i64,
    ) -> Result<WorkspaceSnapshot, sqlx::Error> {
        let workspace_id: i64 = sqlx::query_scalar(
            "SELECT workspace_id FROM workspace_sessions WHERE id = ?1 AND status = 'Active';",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;
        sqlx::query(
            "UPDATE workspace_sessions SET status = 'Completed', ended_at = CURRENT_TIMESTAMP WHERE id = ?1;",
        )
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        self.insert_audit_log(
            "local-user",
            "workspace.session_ended",
            &session_id.to_string(),
            "Info",
        )
        .await?;
        self.workspace_snapshot(Some(workspace_id)).await
    }

    pub async fn archive_cognitive_workspace(
        &self,
        workspace_id: i64,
    ) -> Result<WorkspaceSnapshot, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        let available_workspace_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cognitive_workspaces WHERE status <> 'Archived';",
        )
        .fetch_one(&mut *transaction)
        .await?;
        if available_workspace_count <= 1 {
            return Err(sqlx::Error::RowNotFound);
        }
        let result = sqlx::query(
            "UPDATE cognitive_workspaces SET status = 'Archived', updated_at = CURRENT_TIMESTAMP WHERE id = ?1 AND status <> 'Archived';",
        )
        .bind(workspace_id)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        sqlx::query(
            "UPDATE workspace_sessions SET status = 'Completed', ended_at = CURRENT_TIMESTAMP WHERE workspace_id = ?1 AND status = 'Active';",
        )
        .bind(workspace_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.insert_audit_log(
            "local-user",
            "workspace.archived",
            &workspace_id.to_string(),
            "Info",
        )
        .await?;
        self.workspace_snapshot(Some(workspace_id)).await
    }

    pub async fn restore_cognitive_workspace(
        &self,
        workspace_id: i64,
    ) -> Result<WorkspaceSnapshot, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE cognitive_workspaces SET status = 'Ready', updated_at = CURRENT_TIMESTAMP WHERE id = ?1 AND status = 'Archived';",
        )
        .bind(workspace_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        self.insert_audit_log(
            "local-user",
            "workspace.restored",
            &workspace_id.to_string(),
            "Info",
        )
        .await?;
        self.workspace_snapshot(Some(workspace_id)).await
    }

    pub async fn import_workspace_document(
        &self,
        request: &WorkspaceDocumentImportRequest,
    ) -> Result<WorkspaceDocumentImportResult, sqlx::Error> {
        let session_id: i64 = sqlx::query_scalar(
            r#"SELECT ws.id
               FROM workspace_sessions ws
               JOIN cognitive_workspaces cw ON cw.id = ws.workspace_id
               WHERE ws.workspace_id = ?1
                 AND ws.status = 'Active'
                 AND cw.status <> 'Archived'
                 AND cw.is_imported = 0
               LIMIT 1;"#,
        )
        .bind(request.workspace_id)
        .fetch_one(&self.pool)
        .await?;

        let content_hash = crate::clipboard::hash_bytes(
            format!(
                "local-file:{}\n{}",
                request.file_name.trim(),
                request.content
            )
            .as_bytes(),
        );
        let mut item = crate::clipboard::analyze_text(&request.content, &content_hash).await;
        item.source_application = format!("Local file import: {}", request.file_name.trim());
        item.file_size = Some(request.content.as_bytes().len() as i64);
        item.tags.push("Local file".to_string());
        item.tags.sort();
        item.tags.dedup();

        let stored = match self.insert_item(item).await? {
            Some(memory_id) => {
                self.link_imported_memory_to_workspace(memory_id, request.workspace_id, session_id)
                    .await?;
                self.insert_audit_log(
                    "local-user",
                    "workspace.document_imported",
                    request.file_name.trim(),
                    "Info",
                )
                .await?;
                true
            }
            None => false,
        };

        Ok(WorkspaceDocumentImportResult {
            stored,
            snapshot: self.workspace_snapshot(Some(request.workspace_id)).await?,
        })
    }

    pub async fn import_workspace_handoff(
        &self,
        package: &serde_json::Value,
    ) -> Result<WorkspaceSnapshot, sqlx::Error> {
        let payload = package.get("payload").ok_or(sqlx::Error::RowNotFound)?;
        let source_workspace =
            handoff_text(payload.pointer("/workspace/name"), "Imported workspace");
        let source_project = handoff_text(payload.pointer("/workspace/project"), "Shared handoff");
        let source_recipient =
            handoff_text(payload.pointer("/handoff_intent/recipient"), "Unspecified");
        let source_purpose =
            handoff_text(payload.pointer("/handoff_intent/purpose"), "Unspecified");
        let source_classification = handoff_text(
            payload.pointer("/handoff_intent/classification"),
            "Unspecified",
        );
        let source_generated_at =
            handoff_text(package.get("generated_locally_at"), "Unknown source time");
        let source_expires_at_unix = payload
            .pointer("/handoff_intent/expires_in_days")
            .and_then(serde_json::Value::as_i64)
            .filter(|days| matches!(*days, 1 | 7 | 30))
            .and_then(|days| {
                source_generated_at
                    .parse::<i64>()
                    .ok()
                    .and_then(|generated_at| generated_at.checked_add(days * 86_400))
            });
        let checksum = handoff_text(package.pointer("/integrity/payload_sha256"), "unknown");
        let source_signer_fingerprint = package
            .pointer("/authenticity/signer_fingerprint")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string);
        let short_checksum = checksum.chars().take(8).collect::<String>();
        let workspace_name =
            handoff_name(&format!("Handoff: {source_workspace} [{short_checksum}]"));
        if let Some(existing_id) = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM cognitive_workspaces WHERE name = ?1 LIMIT 1;",
        )
        .bind(&workspace_name)
        .fetch_optional(&self.pool)
        .await?
        {
            return self.workspace_snapshot(Some(existing_id)).await;
        }

        let mut transaction = self.pool.begin().await?;
        let result = sqlx::query(
            "INSERT INTO cognitive_workspaces (name, project, status, is_imported) VALUES (?1, ?2, 'Ready', 1);",
        )
        .bind(&workspace_name)
        .bind(handoff_name(&format!("Imported from {source_project}")))
        .execute(&mut *transaction)
        .await?;
        let workspace_id = result.last_insert_rowid();
        let scope = handoff_text(
            payload.pointer("/scope/session_title"),
            "All workspace sessions",
        );
        let session = sqlx::query(
            r#"INSERT INTO workspace_sessions (workspace_id, title, status, ended_at)
               VALUES (?1, ?2, 'Completed', CURRENT_TIMESTAMP);"#,
        )
        .bind(workspace_id)
        .bind(handoff_name(&format!("Imported handoff: {scope}")))
        .execute(&mut *transaction)
        .await?;
        let session_id = session.last_insert_rowid();
        let mut incident_ids = HashMap::<i64, i64>::new();

        for (index, incident) in handoff_array(payload.get("incidents")).iter().enumerate() {
            let source_id = incident
                .get("id")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(index as i64 + 1);
            let title = handoff_name(&handoff_text(incident.get("title"), "Imported incident"));
            let status = match incident.get("status").and_then(serde_json::Value::as_str) {
                Some("Resolved") => "Resolved",
                _ => "Open",
            };
            let summary = handoff_text(incident.get("summary"), "Imported incident context.");
            let steps = incident
                .get("recommended_steps")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([]));
            let inserted = sqlx::query(
                r#"INSERT INTO insight_incidents (
                       signature, title, status, summary, recommended_steps, event_count, resolved_at
                   ) VALUES (?1, ?2, ?3, ?4, ?5, 1, CASE WHEN ?3 = 'Resolved' THEN CURRENT_TIMESTAMP END);"#,
            )
            .bind(format!("handoff:{checksum}:{source_id}"))
            .bind(title)
            .bind(status)
            .bind(summary)
            .bind(steps.to_string())
            .execute(&mut *transaction)
            .await?;
            incident_ids.insert(source_id, inserted.last_insert_rowid());
        }

        for event in handoff_array(payload.get("events")) {
            let event_type = match event.get("event_type").and_then(serde_json::Value::as_str) {
                Some("Clipboard" | "Terminal" | "Screenshot" | "Error" | "Note") => event
                    .get("event_type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Note"),
                _ => "Note",
            };
            let severity = match event.get("severity").and_then(serde_json::Value::as_str) {
                Some("Warning" | "Critical") => event
                    .get("severity")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Info"),
                _ => "Info",
            };
            let source = handoff_text(event.get("source_application"), "Unknown");
            let source_incident = event.get("incident_id").and_then(serde_json::Value::as_i64);
            let incident_id = source_incident.and_then(|id| incident_ids.get(&id).copied());
            let tags = handoff_tags(event.get("tags"));
            let inserted = sqlx::query(
                r#"INSERT INTO insight_trail_events (
                       event_type, title, details, source_application, severity, incident_id, tags
                   ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);"#,
            )
            .bind(event_type)
            .bind(handoff_name(&handoff_text(
                event.get("title"),
                "Imported event",
            )))
            .bind(handoff_text(
                event.get("details"),
                "Imported handoff event.",
            ))
            .bind(handoff_name(&format!(
                "Handoff: {source_workspace} - {source}"
            )))
            .bind(severity)
            .bind(incident_id)
            .bind(tags.join(","))
            .execute(&mut *transaction)
            .await?;
            sqlx::query(
                r#"INSERT INTO workspace_event_links (workspace_id, session_id, insight_event_id)
                   VALUES (?1, ?2, ?3);"#,
            )
            .bind(workspace_id)
            .bind(session_id)
            .bind(inserted.last_insert_rowid())
            .execute(&mut *transaction)
            .await?;
        }

        for resolution in handoff_array(payload.get("resolutions")) {
            let Some(source_incident_id) = resolution
                .get("incident_id")
                .and_then(serde_json::Value::as_i64)
            else {
                continue;
            };
            let Some(incident_id) = incident_ids.get(&source_incident_id) else {
                continue;
            };
            sqlx::query(
                r#"INSERT INTO incident_resolutions (incident_id, workspace_id, session_id, title, details)
                   VALUES (?1, ?2, ?3, ?4, ?5);"#,
            )
            .bind(incident_id)
            .bind(workspace_id)
            .bind(session_id)
            .bind(handoff_name(&format!(
                "Imported: {}",
                handoff_text(resolution.get("title"), "Recorded remedy")
            )))
            .bind(format!(
                "{}\n\nImported from workspace: {source_workspace}",
                handoff_text(resolution.get("details"), "Imported remediation details.")
            ))
            .execute(&mut *transaction)
            .await?;
        }

        sqlx::query(
            r#"INSERT INTO workspace_handoff_imports (
                   workspace_id, source_workspace, source_project, source_scope,
                   source_recipient, source_purpose, source_classification,
                   source_expires_at_unix, source_signer_fingerprint, source_generated_at, checksum
               ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11);"#,
        )
        .bind(workspace_id)
        .bind(&source_workspace)
        .bind(&source_project)
        .bind(&scope)
        .bind(&source_recipient)
        .bind(&source_purpose)
        .bind(&source_classification)
        .bind(source_expires_at_unix)
        .bind(&source_signer_fingerprint)
        .bind(&source_generated_at)
        .bind(&checksum)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        if let Some(fingerprint) = source_signer_fingerprint.as_deref() {
            self.mark_handoff_signer_used(fingerprint).await?;
        }
        self.insert_audit_log(
            "local-user",
            "workspace.handoff_imported",
            &workspace_name,
            "Info",
        )
        .await?;
        self.workspace_snapshot(Some(workspace_id)).await
    }

    async fn link_imported_memory_to_workspace(
        &self,
        memory_id: i64,
        workspace_id: i64,
        session_id: i64,
    ) -> Result<(), sqlx::Error> {
        let event_id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM insight_trail_events WHERE memory_id = ?1 ORDER BY id DESC LIMIT 1;",
        )
        .bind(memory_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(event_id) = event_id else {
            return Ok(());
        };

        sqlx::query("DELETE FROM workspace_event_links WHERE insight_event_id = ?1;")
            .bind(event_id)
            .execute(&self.pool)
            .await?;
        sqlx::query(
            r#"INSERT OR IGNORE INTO workspace_event_links (workspace_id, session_id, insight_event_id)
               VALUES (?1, ?2, ?3);"#,
        )
        .bind(workspace_id)
        .bind(session_id)
        .bind(event_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn workspace_sessions(
        &self,
        workspace_id: i64,
    ) -> Result<Vec<WorkspaceSession>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT ws.id, ws.workspace_id, ws.title, ws.status,
                    strftime('%Y-%m-%d %H:%M', ws.started_at, 'localtime') AS started_at,
                    CASE WHEN ws.ended_at IS NULL THEN NULL ELSE strftime('%Y-%m-%d %H:%M', ws.ended_at, 'localtime') END AS ended_at,
                    COUNT(wel.insight_event_id) AS event_count
                FROM workspace_sessions ws
                LEFT JOIN workspace_event_links wel ON wel.session_id = ws.id
                WHERE ws.workspace_id = ?1
                GROUP BY ws.id
                ORDER BY CASE ws.status WHEN 'Active' THEN 0 ELSE 1 END, ws.started_at DESC, ws.id DESC
                LIMIT 30;"#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| WorkspaceSession {
                id: row.get("id"),
                workspace_id: row.get("workspace_id"),
                title: row.get("title"),
                status: row.get("status"),
                started_at: row.get("started_at"),
                ended_at: row.get("ended_at"),
                event_count: row.get("event_count"),
            })
            .collect())
    }

    async fn workspace_events(
        &self,
        workspace_id: i64,
    ) -> Result<Vec<InsightTrailEvent>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT ite.id, ite.event_type, ite.title, ite.details, ite.source_application,
                    ite.severity, strftime('%Y-%m-%d %H:%M', ite.created_at, 'localtime') AS created_at,
                    ite.memory_id, ite.screenshot_path, ite.incident_id, wel.session_id, ite.tags
                FROM workspace_event_links wel
                JOIN insight_trail_events ite ON ite.id = wel.insight_event_id
                WHERE wel.workspace_id = ?1
                ORDER BY ite.created_at DESC, ite.id DESC
                LIMIT 200;"#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_insight_event).collect())
    }

    async fn workspace_incidents(
        &self,
        workspace_id: i64,
    ) -> Result<Vec<InsightIncident>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT ii.id, ii.title, ii.status, ii.summary,
                    strftime('%Y-%m-%d %H:%M', ii.first_seen_at, 'localtime') AS first_seen_at,
                    strftime('%Y-%m-%d %H:%M', ii.last_seen_at, 'localtime') AS last_seen_at,
                    COUNT(DISTINCT wel.insight_event_id) AS event_count,
                    ii.recommended_steps
                FROM insight_incidents ii
                JOIN insight_trail_events ite ON ite.incident_id = ii.id
                JOIN workspace_event_links wel ON wel.insight_event_id = ite.id
                WHERE wel.workspace_id = ?1
                GROUP BY ii.id
                ORDER BY CASE ii.status WHEN 'Open' THEN 0 ELSE 1 END, MAX(ite.created_at) DESC, ii.id DESC
                LIMIT 24;"#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_insight_incident).collect())
    }

    async fn workspace_resolutions(
        &self,
        workspace_id: i64,
    ) -> Result<Vec<IncidentResolution>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT ir.id, ir.incident_id, ir.workspace_id, cw.name AS workspace_name,
                    ir.session_id, ir.title, ir.details,
                    strftime('%Y-%m-%d %H:%M', ir.created_at, 'localtime') AS created_at
                FROM incident_resolutions ir
                JOIN cognitive_workspaces cw ON cw.id = ir.workspace_id
                WHERE ir.incident_id IN (
                    SELECT DISTINCT ite.incident_id
                    FROM workspace_event_links wel
                    JOIN insight_trail_events ite ON ite.id = wel.insight_event_id
                    WHERE wel.workspace_id = ?1 AND ite.incident_id IS NOT NULL
                )
                ORDER BY ir.created_at DESC, ir.id DESC
                LIMIT 50;"#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_incident_resolution).collect())
    }

    async fn workspace_import_provenance(
        &self,
        workspace_id: i64,
    ) -> Result<Option<WorkspaceImportProvenance>, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT source_workspace, source_project, source_scope,
                    source_recipient, source_purpose, source_classification, source_expires_at_unix,
                    source_signer_fingerprint, source_generated_at,
                    checksum,
                    strftime('%Y-%m-%d %H:%M', imported_at, 'localtime') AS imported_at
               FROM workspace_handoff_imports
               WHERE workspace_id = ?1;"#,
        )
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| WorkspaceImportProvenance {
            source_workspace: row.get("source_workspace"),
            source_project: row.get("source_project"),
            source_scope: row.get("source_scope"),
            source_recipient: row.get("source_recipient"),
            source_purpose: row.get("source_purpose"),
            source_classification: row.get("source_classification"),
            source_expires_at_unix: row.get("source_expires_at_unix"),
            source_signer_fingerprint: row.get("source_signer_fingerprint"),
            source_generated_at: row.get("source_generated_at"),
            checksum: row.get("checksum"),
            imported_at: row.get("imported_at"),
        }))
    }

    pub async fn record_incident_resolution(
        &self,
        request: &IncidentResolutionRequest,
    ) -> Result<WorkspaceSnapshot, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        let is_imported: i64 =
            sqlx::query_scalar("SELECT is_imported FROM cognitive_workspaces WHERE id = ?1;")
                .bind(request.workspace_id)
                .fetch_one(&mut *transaction)
                .await?;
        if is_imported == 1 {
            return Err(sqlx::Error::RowNotFound);
        }
        let incident_is_in_workspace: Option<i64> = sqlx::query_scalar(
            r#"SELECT 1 FROM workspace_event_links wel
               JOIN insight_trail_events ite ON ite.id = wel.insight_event_id
               WHERE wel.workspace_id = ?1 AND ite.incident_id = ?2
               LIMIT 1;"#,
        )
        .bind(request.workspace_id)
        .bind(request.incident_id)
        .fetch_optional(&mut *transaction)
        .await?;
        if incident_is_in_workspace.is_none() {
            return Err(sqlx::Error::RowNotFound);
        }
        let session_id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM workspace_sessions WHERE workspace_id = ?1 AND status = 'Active' LIMIT 1;",
        )
        .bind(request.workspace_id)
        .fetch_optional(&mut *transaction)
        .await?;
        sqlx::query(
            r#"INSERT INTO incident_resolutions (incident_id, workspace_id, session_id, title, details)
               VALUES (?1, ?2, ?3, ?4, ?5);"#,
        )
        .bind(request.incident_id)
        .bind(request.workspace_id)
        .bind(session_id)
        .bind(request.title.trim())
        .bind(request.details.trim())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"UPDATE insight_incidents
               SET status = 'Resolved', resolved_at = CURRENT_TIMESTAMP, last_seen_at = CURRENT_TIMESTAMP
               WHERE id = ?1;"#,
        )
        .bind(request.incident_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.insert_audit_log(
            "local-user",
            "incident.resolved",
            request.title.trim(),
            "Info",
        )
        .await?;
        self.workspace_snapshot(Some(request.workspace_id)).await
    }

    pub async fn reopen_incident(
        &self,
        request: &IncidentReopenRequest,
    ) -> Result<WorkspaceSnapshot, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        let is_imported: i64 =
            sqlx::query_scalar("SELECT is_imported FROM cognitive_workspaces WHERE id = ?1;")
                .bind(request.workspace_id)
                .fetch_one(&mut *transaction)
                .await?;
        if is_imported == 1 {
            return Err(sqlx::Error::RowNotFound);
        }
        let session_id: i64 = sqlx::query_scalar(
            r#"SELECT ws.id
               FROM workspace_sessions ws
               JOIN cognitive_workspaces cw ON cw.id = ws.workspace_id
               WHERE ws.workspace_id = ?1
                 AND ws.status = 'Active'
                 AND cw.status <> 'Archived'
               LIMIT 1;"#,
        )
        .bind(request.workspace_id)
        .fetch_one(&mut *transaction)
        .await?;
        let incident_title: String = sqlx::query_scalar(
            r#"SELECT ii.title
               FROM insight_incidents ii
               JOIN insight_trail_events ite ON ite.incident_id = ii.id
               JOIN workspace_event_links wel ON wel.insight_event_id = ite.id
               WHERE wel.workspace_id = ?1 AND ii.id = ?2
               LIMIT 1;"#,
        )
        .bind(request.workspace_id)
        .bind(request.incident_id)
        .fetch_one(&mut *transaction)
        .await?;
        let result = sqlx::query(
            r#"UPDATE insight_incidents
               SET status = 'Open', resolved_at = NULL, last_seen_at = CURRENT_TIMESTAMP
               WHERE id = ?1 AND status = 'Resolved';"#,
        )
        .bind(request.incident_id)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        let event = sqlx::query(
            r#"INSERT INTO insight_trail_events (
                   event_type, title, details, source_application, severity, incident_id, tags
               ) VALUES ('Note', ?1, ?2, 'CYMOS', 'Warning', ?3, 'Incident,Follow-up');"#,
        )
        .bind(format!("Incident reopened: {incident_title}"))
        .bind(request.reason.trim())
        .bind(request.incident_id)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"INSERT INTO workspace_event_links (workspace_id, session_id, insight_event_id)
               VALUES (?1, ?2, ?3);"#,
        )
        .bind(request.workspace_id)
        .bind(session_id)
        .bind(event.last_insert_rowid())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.insert_audit_log(
            "local-user",
            "incident.reopened",
            request.reason.trim(),
            "Warning",
        )
        .await?;
        self.workspace_snapshot(Some(request.workspace_id)).await
    }

    pub async fn link_workspace_event_to_incident(
        &self,
        request: &workspace::IncidentEvidenceLinkRequest,
    ) -> Result<WorkspaceSnapshot, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        let workspace_is_mutable: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM cognitive_workspaces WHERE id = ?1 AND is_imported = 0 AND status <> 'Archived';",
        )
        .bind(request.workspace_id)
        .fetch_optional(&mut *transaction)
        .await?;
        if workspace_is_mutable.is_none() {
            return Err(sqlx::Error::RowNotFound);
        }
        let incident_is_in_workspace: Option<i64> = sqlx::query_scalar(
            r#"SELECT 1 FROM workspace_event_links wel
               JOIN insight_trail_events ite ON ite.id = wel.insight_event_id
               WHERE wel.workspace_id = ?1 AND ite.incident_id = ?2
               LIMIT 1;"#,
        )
        .bind(request.workspace_id)
        .bind(request.incident_id)
        .fetch_optional(&mut *transaction)
        .await?;
        if incident_is_in_workspace.is_none() {
            return Err(sqlx::Error::RowNotFound);
        }
        let evidence_title: Option<String> = sqlx::query_scalar(
            r#"SELECT ite.title FROM workspace_event_links wel
               JOIN insight_trail_events ite ON ite.id = wel.insight_event_id
               WHERE wel.workspace_id = ?1
                 AND ite.id = ?2
                 AND ite.memory_id IS NOT NULL
                 AND ite.incident_id IS NULL
               LIMIT 1;"#,
        )
        .bind(request.workspace_id)
        .bind(request.event_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(evidence_title) = evidence_title else {
            return Err(sqlx::Error::RowNotFound);
        };
        let linked = sqlx::query(
            "UPDATE insight_trail_events SET incident_id = ?1 WHERE id = ?2 AND incident_id IS NULL;",
        )
        .bind(request.incident_id)
        .bind(request.event_id)
        .execute(&mut *transaction)
        .await?;
        if linked.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        sqlx::query(
            r#"UPDATE insight_incidents
               SET event_count = (SELECT COUNT(*) FROM insight_trail_events WHERE incident_id = ?1),
                   last_seen_at = CURRENT_TIMESTAMP
               WHERE id = ?1;"#,
        )
        .bind(request.incident_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.insert_audit_log(
            "local-user",
            "incident.evidence_linked",
            &evidence_title,
            "Info",
        )
        .await?;
        self.workspace_snapshot(Some(request.workspace_id)).await
    }

    pub async fn runbook_entries(
        &self,
        request: &RunbookSearchRequest,
    ) -> Result<Vec<RunbookEntry>, sqlx::Error> {
        let resolution_rows = sqlx::query(
            r#"SELECT ir.id, ir.incident_id, ii.title AS incident_title,
                    cw.name AS workspace_name, ir.title, ir.details, '' AS tags,
                    strftime('%Y-%m-%d %H:%M', ir.created_at, 'localtime') AS created_at,
                    0 AS latest_revision, NULL AS last_reviewed_revision,
                    NULL AS last_reviewed_at, NULL AS last_review_note,
                    'Incident evidence' AS review_status
                FROM incident_resolutions ir
                JOIN insight_incidents ii ON ii.id = ir.incident_id
                JOIN cognitive_workspaces cw ON cw.id = ir.workspace_id
                ORDER BY ir.created_at DESC, ir.id DESC LIMIT 100;"#,
        )
        .fetch_all(&self.pool)
        .await?;
        let manual_rows = sqlx::query(
            r#"SELECT -mr.id AS id, NULL AS incident_id, 'Manual runbook' AS incident_title,
                    'Local vault' AS workspace_name, mr.title, mr.details, mr.tags,
                    strftime('%Y-%m-%d %H:%M', mr.created_at, 'localtime') AS created_at,
                    COALESCE((SELECT MAX(revision) FROM manual_runbook_revisions WHERE runbook_id = mr.id), 0) AS latest_revision,
                    mr.last_reviewed_revision,
                    strftime('%Y-%m-%d %H:%M', mr.last_reviewed_at, 'localtime') AS last_reviewed_at,
                    (SELECT note FROM manual_runbook_reviews WHERE runbook_id = mr.id AND revision = mr.last_reviewed_revision ORDER BY id DESC LIMIT 1) AS last_review_note,
                    CASE
                        WHEN mr.last_reviewed_revision IS NULL
                            OR mr.last_reviewed_revision != COALESCE((SELECT MAX(revision) FROM manual_runbook_revisions WHERE runbook_id = mr.id), 0)
                            THEN 'Needs review'
                        WHEN mr.last_reviewed_at < datetime('now', '-90 days') THEN 'Review due'
                        ELSE 'Reviewed'
                    END AS review_status
                FROM manual_runbooks mr
                ORDER BY mr.created_at DESC, mr.id DESC LIMIT 100;"#,
        )
        .fetch_all(&self.pool)
        .await?;
        let query = request.query.trim().to_lowercase();
        let mut entries = resolution_rows
            .into_iter()
            .chain(manual_rows)
            .map(row_to_runbook_entry)
            .filter(|entry| {
                let matches_query = query.is_empty()
                    || [
                        entry.incident_title.as_str(),
                        entry.workspace_name.as_str(),
                        entry.title.as_str(),
                        entry.details.as_str(),
                        &entry.tags.join(" "),
                    ]
                    .join(" ")
                    .to_lowercase()
                    .contains(&query);
                let matches_review = match request.review_status.as_str() {
                    "All" => true,
                    "Needs review" => {
                        entry.incident_id.is_none() && entry.review_status != "Reviewed"
                    }
                    "Review due" => entry.review_status == "Review due",
                    "Reviewed" => entry.review_status == "Reviewed",
                    _ => false,
                };
                matches_query && matches_review
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then(right.id.cmp(&left.id))
        });
        entries.truncate(50);
        Ok(entries)
    }

    pub async fn runbook_entry(&self, entry_id: i64) -> Result<RunbookEntry, sqlx::Error> {
        if entry_id < 0 {
            return self
                .manual_runbook_entry(entry_id.checked_neg().ok_or(sqlx::Error::RowNotFound)?)
                .await;
        }

        let row = sqlx::query(
            r#"SELECT ir.id, ir.incident_id, ii.title AS incident_title,
                    cw.name AS workspace_name, ir.title, ir.details, '' AS tags,
                    strftime('%Y-%m-%d %H:%M', ir.created_at, 'localtime') AS created_at,
                    0 AS latest_revision, NULL AS last_reviewed_revision,
                    NULL AS last_reviewed_at, NULL AS last_review_note,
                    'Incident evidence' AS review_status
                FROM incident_resolutions ir
                JOIN insight_incidents ii ON ii.id = ir.incident_id
                JOIN cognitive_workspaces cw ON cw.id = ir.workspace_id
                WHERE ir.id = ?1;"#,
        )
        .bind(entry_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row_to_runbook_entry(row))
    }

    pub async fn record_runbook_export(&self, entry: &RunbookEntry) -> Result<(), sqlx::Error> {
        self.insert_audit_log("local-user", "runbook.exported", &entry.title, "Info")
            .await
    }

    pub async fn record_runbook_copy(&self, entry: &RunbookEntry) -> Result<(), sqlx::Error> {
        self.insert_audit_log("local-user", "runbook.copied", &entry.title, "Info")
            .await
    }

    pub async fn manual_runbook_revisions(
        &self,
        entry_id: i64,
    ) -> Result<Vec<RunbookRevision>, sqlx::Error> {
        let runbook_id = entry_id.checked_neg().ok_or(sqlx::Error::RowNotFound)?;
        let rows = sqlx::query(
            r#"SELECT id, runbook_id, revision, title, details, tags,
                    strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
                FROM manual_runbook_revisions
                WHERE runbook_id = ?1
                ORDER BY revision DESC;"#,
        )
        .bind(runbook_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_runbook_revision).collect())
    }

    pub async fn restore_manual_runbook_revision(
        &self,
        request: &ManualRunbookRevisionRestoreRequest,
    ) -> Result<RunbookEntry, sqlx::Error> {
        let runbook_id = request
            .entry_id
            .checked_neg()
            .ok_or(sqlx::Error::RowNotFound)?;
        let mut transaction = self.pool.begin().await?;
        let revision = sqlx::query(
            "SELECT title, details, tags FROM manual_runbook_revisions WHERE id = ?1 AND runbook_id = ?2;",
        )
        .bind(request.revision_id)
        .bind(runbook_id)
        .fetch_one(&mut *transaction)
        .await?;
        let title: String = revision.get("title");
        let details: String = revision.get("details");
        let tags: String = revision.get("tags");
        let result = sqlx::query(
            "UPDATE manual_runbooks SET title = ?1, details = ?2, tags = ?3 WHERE id = ?4;",
        )
        .bind(&title)
        .bind(&details)
        .bind(&tags)
        .bind(runbook_id)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        let next_revision = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(revision), 0) + 1 FROM manual_runbook_revisions WHERE runbook_id = ?1;",
        )
        .bind(runbook_id)
        .fetch_one(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO manual_runbook_revisions (runbook_id, revision, title, details, tags) VALUES (?1, ?2, ?3, ?4, ?5);",
        )
        .bind(runbook_id)
        .bind(next_revision)
        .bind(&title)
        .bind(&details)
        .bind(&tags)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.insert_audit_log("local-user", "runbook.revision_restored", &title, "Warning")
            .await?;
        self.manual_runbook_entry(runbook_id).await
    }

    pub async fn review_manual_runbook(
        &self,
        request: &ManualRunbookReviewRequest,
    ) -> Result<RunbookEntry, sqlx::Error> {
        let runbook_id = request
            .entry_id
            .checked_neg()
            .ok_or(sqlx::Error::RowNotFound)?;
        let mut transaction = self.pool.begin().await?;
        let revision = sqlx::query_scalar::<_, i64>(
            "SELECT MAX(revision) FROM manual_runbook_revisions WHERE runbook_id = ?1;",
        )
        .bind(runbook_id)
        .fetch_one(&mut *transaction)
        .await?;
        let result = sqlx::query(
            r#"UPDATE manual_runbooks
                SET last_reviewed_revision = ?1,
                    last_reviewed_at = CURRENT_TIMESTAMP
                WHERE id = ?2;"#,
        )
        .bind(revision)
        .bind(runbook_id)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        sqlx::query(
            "INSERT INTO manual_runbook_reviews (runbook_id, revision, note) VALUES (?1, ?2, ?3);",
        )
        .bind(runbook_id)
        .bind(revision)
        .bind(request.note.trim())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        let entry = self.manual_runbook_entry(runbook_id).await?;
        self.insert_audit_log("local-user", "runbook.reviewed", &entry.title, "Info")
            .await?;
        Ok(entry)
    }

    pub async fn create_manual_runbook(
        &self,
        request: &ManualRunbookRequest,
    ) -> Result<RunbookEntry, sqlx::Error> {
        let tags = insight_trail::clean_tags(&request.tags);
        let serialized_tags = tags.join(",");
        let mut transaction = self.pool.begin().await?;
        let result =
            sqlx::query("INSERT INTO manual_runbooks (title, details, tags) VALUES (?1, ?2, ?3);")
                .bind(request.title.trim())
                .bind(request.details.trim())
                .bind(&serialized_tags)
                .execute(&mut *transaction)
                .await?;
        let id = result.last_insert_rowid();
        sqlx::query(
            "INSERT INTO manual_runbook_revisions (runbook_id, revision, title, details, tags) VALUES (?1, 1, ?2, ?3, ?4);",
        )
        .bind(id)
        .bind(request.title.trim())
        .bind(request.details.trim())
        .bind(&serialized_tags)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.insert_audit_log(
            "local-user",
            "runbook.created",
            request.title.trim(),
            "Info",
        )
        .await?;
        Ok(RunbookEntry {
            id: -id,
            incident_id: None,
            incident_title: "Manual runbook".to_string(),
            workspace_name: "Local vault".to_string(),
            title: request.title.trim().to_string(),
            details: request.details.trim().to_string(),
            tags,
            created_at: String::new(),
            latest_revision: 1,
            last_reviewed_revision: None,
            last_reviewed_at: None,
            last_review_note: None,
            review_status: "Needs review".to_string(),
        })
    }

    pub async fn update_manual_runbook(
        &self,
        request: &ManualRunbookUpdateRequest,
    ) -> Result<RunbookEntry, sqlx::Error> {
        let id = request.id.checked_neg().ok_or(sqlx::Error::RowNotFound)?;
        let tags = insight_trail::clean_tags(&request.tags);
        let serialized_tags = tags.join(",");
        let mut transaction = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE manual_runbooks SET title = ?1, details = ?2, tags = ?3 WHERE id = ?4;",
        )
        .bind(request.title.trim())
        .bind(request.details.trim())
        .bind(&serialized_tags)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        let revision = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(revision), 0) + 1 FROM manual_runbook_revisions WHERE runbook_id = ?1;",
        )
        .bind(id)
        .fetch_one(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO manual_runbook_revisions (runbook_id, revision, title, details, tags) VALUES (?1, ?2, ?3, ?4, ?5);",
        )
        .bind(id)
        .bind(revision)
        .bind(request.title.trim())
        .bind(request.details.trim())
        .bind(&serialized_tags)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.insert_audit_log(
            "local-user",
            "runbook.updated",
            request.title.trim(),
            "Info",
        )
        .await?;
        self.manual_runbook_entry(id).await
    }

    pub async fn delete_manual_runbook(&self, entry_id: i64) -> Result<(), sqlx::Error> {
        let id = entry_id.checked_neg().ok_or(sqlx::Error::RowNotFound)?;
        let result = sqlx::query("DELETE FROM manual_runbooks WHERE id = ?1;")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        self.insert_audit_log("local-user", "runbook.deleted", &id.to_string(), "Warning")
            .await?;
        Ok(())
    }

    async fn manual_runbook_entry(&self, id: i64) -> Result<RunbookEntry, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT -mr.id AS id, NULL AS incident_id, 'Manual runbook' AS incident_title,
                    'Local vault' AS workspace_name, mr.title, mr.details, mr.tags,
                    strftime('%Y-%m-%d %H:%M', mr.created_at, 'localtime') AS created_at,
                    COALESCE((SELECT MAX(revision) FROM manual_runbook_revisions WHERE runbook_id = mr.id), 0) AS latest_revision,
                    mr.last_reviewed_revision,
                    strftime('%Y-%m-%d %H:%M', mr.last_reviewed_at, 'localtime') AS last_reviewed_at,
                    (SELECT note FROM manual_runbook_reviews WHERE runbook_id = mr.id AND revision = mr.last_reviewed_revision ORDER BY id DESC LIMIT 1) AS last_review_note,
                    CASE
                        WHEN mr.last_reviewed_revision IS NULL
                            OR mr.last_reviewed_revision != COALESCE((SELECT MAX(revision) FROM manual_runbook_revisions WHERE runbook_id = mr.id), 0)
                            THEN 'Needs review'
                        WHEN mr.last_reviewed_at < datetime('now', '-90 days') THEN 'Review due'
                        ELSE 'Reviewed'
                    END AS review_status
                FROM manual_runbooks mr WHERE mr.id = ?1;"#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row_to_runbook_entry(row))
    }

    async fn record_insight_event_from_memory(
        &self,
        memory_id: i64,
        item: &NewClipboardItem,
    ) -> Result<(), sqlx::Error> {
        let settings = self.insight_trail_settings().await?;
        let event = insight_trail::event_from_memory(
            memory_id,
            &item.content,
            &item.content_type,
            &item.source_application,
            &item.operational_context.kind,
            &item.ai_summary,
            &item.tags,
        );
        if !insight_trail::settings_allow(&settings, &event) {
            return Ok(());
        }

        let event_id = self.insert_insight_event(&event).await?;
        if settings.create_incidents {
            if let Some(signature) = event.incident_signature.as_deref() {
                self.upsert_insight_incident(event_id, signature, &event)
                    .await?;
            }
        }
        Ok(())
    }

    async fn insert_insight_event(&self, event: &NewInsightTrailEvent) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO insight_trail_events (
                event_type,
                title,
                details,
                source_application,
                severity,
                memory_id,
                screenshot_path,
                tags
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);
            "#,
        )
        .bind(&event.event_type)
        .bind(&event.title)
        .bind(&event.details)
        .bind(&event.source_application)
        .bind(&event.severity)
        .bind(event.memory_id)
        .bind(&event.screenshot_path)
        .bind(event.tags.join(","))
        .execute(&self.pool)
        .await?;
        let event_id = result.last_insert_rowid();
        self.link_event_to_active_workspace(event_id).await?;
        Ok(event_id)
    }

    async fn link_event_to_active_workspace(&self, event_id: i64) -> Result<(), sqlx::Error> {
        let active_session = sqlx::query(
            "SELECT id, workspace_id FROM workspace_sessions WHERE status = 'Active' LIMIT 1;",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(session) = active_session else {
            return Ok(());
        };
        sqlx::query(
            r#"INSERT OR IGNORE INTO workspace_event_links (workspace_id, session_id, insight_event_id)
               VALUES (?1, ?2, ?3);"#,
        )
        .bind(session.get::<i64, _>("workspace_id"))
        .bind(session.get::<i64, _>("id"))
        .bind(event_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn upsert_insight_incident(
        &self,
        event_id: i64,
        signature: &str,
        event: &NewInsightTrailEvent,
    ) -> Result<(), sqlx::Error> {
        let title = format!("{} incident", signature.replace(':', " "));
        let steps =
            serde_json::to_string(&event.recommended_steps).unwrap_or_else(|_| "[]".to_string());
        sqlx::query(
            r#"
            INSERT INTO insight_incidents (signature, title, status, summary, recommended_steps, event_count)
            VALUES (?1, ?2, 'Open', ?3, ?4, 1)
            ON CONFLICT(signature) DO UPDATE SET
                status = 'Open',
                summary = excluded.summary,
                recommended_steps = excluded.recommended_steps,
                event_count = insight_incidents.event_count + 1,
                last_seen_at = CURRENT_TIMESTAMP,
                resolved_at = NULL;
            "#,
        )
        .bind(signature)
        .bind(title)
        .bind(&event.details)
        .bind(steps)
        .execute(&self.pool)
        .await?;

        let incident_id: i64 =
            sqlx::query_scalar("SELECT id FROM insight_incidents WHERE signature = ?1;")
                .bind(signature)
                .fetch_one(&self.pool)
                .await?;
        sqlx::query("UPDATE insight_trail_events SET incident_id = ?1 WHERE id = ?2;")
            .bind(incident_id)
            .bind(event_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn prune_insight_trail_events(&self, retention_days: i64) -> Result<i64, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM insight_trail_events WHERE created_at < datetime('now', ?1);")
                .bind(format!("-{retention_days} days"))
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() as i64)
    }

    pub async fn create_collection(&self, name: &str, color: &str) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT OR IGNORE INTO collections (name, color) VALUES (?1, ?2);")
            .bind(name.trim())
            .bind(color)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn move_to_collection(
        &self,
        item_id: i64,
        collection_id: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE clipboard_items SET collection_id = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2;",
        )
        .bind(collection_id)
        .bind(item_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn toggle_favorite(&self, item_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE clipboard_items
            SET is_favorite = CASE is_favorite WHEN 1 THEN 0 ELSE 1 END,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?1;
            "#,
        )
        .bind(item_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_item(&self, item_id: i64) -> Result<(), sqlx::Error> {
        self.delete_items_by_ids(&[item_id]).await?;
        Ok(())
    }

    async fn delete_items_by_ids(&self, item_ids: &[i64]) -> Result<i64, sqlx::Error> {
        if item_ids.is_empty() {
            return Ok(0);
        }

        let mut image_assets = QueryBuilder::<Sqlite>::new(
            "SELECT content FROM clipboard_items WHERE content_type = 'Image' AND id IN (",
        );
        {
            let mut values = image_assets.separated(", ");
            for item_id in item_ids {
                values.push_bind(item_id);
            }
        }
        image_assets.push(");");
        let image_assets = image_assets.build().fetch_all(&self.pool).await?;

        let mut transaction = self.pool.begin().await?;
        let mut clear_events = QueryBuilder::<Sqlite>::new(
            "UPDATE insight_trail_events SET memory_id = NULL, screenshot_path = NULL WHERE memory_id IN (",
        );
        {
            let mut values = clear_events.separated(", ");
            for item_id in item_ids {
                values.push_bind(item_id);
            }
        }
        clear_events.push(");");
        clear_events.build().execute(&mut *transaction).await?;

        let mut delete_entities =
            QueryBuilder::<Sqlite>::new("DELETE FROM memory_entities WHERE clipboard_id IN (");
        {
            let mut values = delete_entities.separated(", ");
            for item_id in item_ids {
                values.push_bind(item_id);
            }
        }
        delete_entities.push(");");
        delete_entities.build().execute(&mut *transaction).await?;

        let mut delete_tags =
            QueryBuilder::<Sqlite>::new("DELETE FROM clipboard_tags WHERE clipboard_id IN (");
        {
            let mut values = delete_tags.separated(", ");
            for item_id in item_ids {
                values.push_bind(item_id);
            }
        }
        delete_tags.push(");");
        delete_tags.build().execute(&mut *transaction).await?;

        let mut delete_items =
            QueryBuilder::<Sqlite>::new("DELETE FROM clipboard_items WHERE id IN (");
        {
            let mut values = delete_items.separated(", ");
            for item_id in item_ids {
                values.push_bind(item_id);
            }
        }
        delete_items.push(");");
        let deleted = delete_items.build().execute(&mut *transaction).await?;
        sqlx::query(
            "DELETE FROM tags WHERE NOT EXISTS (SELECT 1 FROM clipboard_tags WHERE clipboard_tags.tag_id = tags.id);",
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;

        for asset in image_assets {
            self.remove_owned_asset(&asset.get::<String, _>("content"));
        }
        Ok(deleted.rows_affected() as i64)
    }

    pub async fn item(&self, item_id: i64) -> Result<Option<ClipboardItem>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                c.id,
                c.content,
                c.content_type,
                c.source_application,
                strftime('%Y-%m-%d %H:%M', c.created_at, 'localtime') AS created_at,
                strftime('%Y-%m-%d %H:%M', c.updated_at, 'localtime') AS updated_at,
                c.content_hash,
                c.character_count,
                c.word_count,
                c.file_size,
                c.image_width,
                c.image_height,
                c.language,
                c.is_favorite,
                c.collection_id,
                collections.name AS collection_name,
                collections.color AS collection_color,
                c.ai_summary,
                c.category,
                c.keywords,
                c.reading_time_minutes,
                c.copy_count,
                COALESCE(strftime('%Y-%m-%d %H:%M', c.last_copied_at, 'localtime'), c.created_at) AS last_copied_at,
                c.semantic_text,
                c.embedding,
                c.embedding_source,
                c.operational_context,
                COALESCE(GROUP_CONCAT(tags.name, ','), '') AS tags
            FROM clipboard_items c
            LEFT JOIN collections ON collections.id = c.collection_id
            LEFT JOIN clipboard_tags ON clipboard_tags.clipboard_id = c.id
            LEFT JOIN tags ON tags.id = clipboard_tags.tag_id
            WHERE c.id = ?1
            GROUP BY c.id;
            "#,
        )
        .bind(item_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .next()
            .map(|row| row_to_item(row, None, "")))
    }

    async fn embedding_for_item(&self, item_id: i64) -> Result<Vec<f32>, sqlx::Error> {
        let embedding: Option<String> =
            sqlx::query_scalar("SELECT embedding FROM clipboard_items WHERE id = ?1;")
                .bind(item_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(embedding
            .map(|value| semantic::deserialize_embedding(&value))
            .unwrap_or_default())
    }

    async fn tags_for_item(&self, item_id: i64) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT tags.name
            FROM clipboard_tags
            JOIN tags ON tags.id = clipboard_tags.tag_id
            WHERE clipboard_tags.clipboard_id = ?1;
            "#,
        )
        .bind(item_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect())
    }

    fn remove_owned_asset(&self, value: &str) {
        let Ok(assets_dir) = fs::canonicalize(self.assets_dir()) else {
            return;
        };
        let Ok(asset_path) = fs::canonicalize(value) else {
            return;
        };
        if asset_path.starts_with(assets_dir) {
            let _ = fs::remove_file(asset_path);
        }
    }

    async fn index_graph_for_item(
        &self,
        item_id: i64,
        item: &NewClipboardItem,
    ) -> Result<(), sqlx::Error> {
        let entities = graph::extract_entities(
            &item.content,
            &item.content_type,
            item.language.as_deref(),
            &item.category,
            &item.keywords,
            &item.tags,
        );
        if entities.is_empty() {
            return Ok(());
        }

        let cluster = graph::cluster_for_entities(&item.category, &entities);
        let mut entity_ids = Vec::new();

        for entity in &entities {
            sqlx::query(
                r#"
                INSERT INTO graph_entities (name, entity_type, weight, cluster, first_seen_at, last_seen_at)
                VALUES (?1, ?2, 1, ?3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                ON CONFLICT(name) DO UPDATE SET
                    weight = weight + 1,
                    entity_type = excluded.entity_type,
                    cluster = excluded.cluster,
                    last_seen_at = CURRENT_TIMESTAMP;
                "#,
            )
            .bind(&entity.name)
            .bind(&entity.entity_type)
            .bind(&cluster)
            .execute(&self.pool)
            .await?;

            let entity_id: i64 =
                sqlx::query_scalar("SELECT id FROM graph_entities WHERE name = ?1;")
                    .bind(&entity.name)
                    .fetch_one(&self.pool)
                    .await?;

            sqlx::query(
                "INSERT OR IGNORE INTO memory_entities (clipboard_id, entity_id) VALUES (?1, ?2);",
            )
            .bind(item_id)
            .bind(entity_id)
            .execute(&self.pool)
            .await?;

            entity_ids.push((entity_id, entity));
        }

        for left_index in 0..entity_ids.len() {
            for right_index in (left_index + 1)..entity_ids.len() {
                let (left_id, left_entity) = entity_ids[left_index];
                let (right_id, right_entity) = entity_ids[right_index];
                let (source, target) = if left_id <= right_id {
                    (left_id, right_id)
                } else {
                    (right_id, left_id)
                };
                let relationship = graph::relationship_between(left_entity, right_entity);

                sqlx::query(
                    r#"
                    INSERT INTO graph_relationships (
                        source_entity_id,
                        target_entity_id,
                        relationship,
                        weight,
                        last_seen_at
                    )
                    VALUES (?1, ?2, ?3, 1, CURRENT_TIMESTAMP)
                    ON CONFLICT(source_entity_id, target_entity_id, relationship) DO UPDATE SET
                        weight = weight + 1,
                        last_seen_at = CURRENT_TIMESTAMP;
                    "#,
                )
                .bind(source)
                .bind(target)
                .bind(relationship)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn stats(&self) -> Result<ClipboardStats, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) AS total_items,
                COALESCE(SUM(CASE WHEN content_type = 'Text' THEN 1 ELSE 0 END), 0) AS text_items,
                COALESCE(SUM(CASE WHEN content_type = 'Image' THEN 1 ELSE 0 END), 0) AS image_items,
                COALESCE(SUM(CASE WHEN content_type = 'Code' THEN 1 ELSE 0 END), 0) AS code_items,
                COALESCE(SUM(CASE WHEN content_type = 'URL' THEN 1 ELSE 0 END), 0) AS url_items,
                COALESCE(SUM(CASE WHEN content_type IN ('File', 'Folder') THEN 1 ELSE 0 END), 0) AS file_items,
                COALESCE(SUM(
                    CASE
                        WHEN content_type = 'Image' THEN COALESCE(file_size, 0)
                        ELSE length(CAST(content AS BLOB))
                    END
                ), 0) AS storage_used,
                COALESCE(SUM(CASE WHEN is_favorite = 1 THEN 1 ELSE 0 END), 0) AS favorite_items
            FROM clipboard_items;
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let most_used_application = sqlx::query_scalar::<_, String>(
            r#"
            SELECT source_application
            FROM clipboard_items
            GROUP BY source_application
            ORDER BY COUNT(*) DESC
            LIMIT 1;
            "#,
        )
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or_else(|| "Unknown".to_string());

        Ok(ClipboardStats {
            total_items: row.get::<i64, _>("total_items"),
            text_items: row.get::<i64, _>("text_items"),
            image_items: row.get::<i64, _>("image_items"),
            code_items: row.get::<i64, _>("code_items"),
            url_items: row.get::<i64, _>("url_items"),
            file_items: row.get::<i64, _>("file_items"),
            storage_used: row.get::<i64, _>("storage_used"),
            favorite_items: row.get::<i64, _>("favorite_items"),
            most_used_application,
        })
    }

    pub async fn save_agent_workflow(
        &self,
        workflow: &agent::AgentWorkflow,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO agent_workflows (
                goal,
                status,
                agents,
                plan,
                answer,
                recommendations,
                context_memory_ids
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);
            "#,
        )
        .bind(&workflow.goal)
        .bind(&workflow.status)
        .bind(serde_json::to_string(&workflow.agents).unwrap_or_else(|_| "[]".to_string()))
        .bind(serde_json::to_string(&workflow.plan).unwrap_or_else(|_| "[]".to_string()))
        .bind(&workflow.answer)
        .bind(serde_json::to_string(&workflow.recommendations).unwrap_or_else(|_| "[]".to_string()))
        .bind(
            serde_json::to_string(&workflow.context_memory_ids)
                .unwrap_or_else(|_| "[]".to_string()),
        )
        .execute(&self.pool)
        .await?;

        let workflow_id = result.last_insert_rowid();
        for log in &workflow.logs {
            sqlx::query(
                r#"
                INSERT INTO agent_logs (workflow_id, agent, message)
                VALUES (?1, ?2, ?3);
                "#,
            )
            .bind(workflow_id)
            .bind(&log.agent)
            .bind(&log.message)
            .execute(&self.pool)
            .await?;
        }

        Ok(workflow_id)
    }

    pub async fn agent_workflows(&self) -> Result<Vec<AgentWorkflowRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                goal,
                status,
                agents,
                answer,
                recommendations,
                strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
            FROM agent_workflows
            ORDER BY id DESC
            LIMIT 30;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| AgentWorkflowRecord {
                id: row.get("id"),
                goal: row.get("goal"),
                status: row.get("status"),
                agents: serde_json::from_str(&row.get::<String, _>("agents")).unwrap_or_default(),
                answer: row.get("answer"),
                recommendations: serde_json::from_str(&row.get::<String, _>("recommendations"))
                    .unwrap_or_default(),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    pub async fn run_autonomous_cycle(&self) -> Result<AutomationRunResult, sqlx::Error> {
        let mut tasks_run = 0;
        let mut reports_created = 0;
        let mut notifications_created = 0;

        self.insert_automation_task("Memory Monitor", "Completed", "Scanned captured memories.")
            .await?;
        tasks_run += 1;

        let retention = self.apply_vault_retention().await?;
        self.insert_automation_task(
            "Vault Retention",
            "Completed",
            &format!(
                "{} memories removed; {} remain under the local policy.",
                retention.removed_items, retention.remaining_items
            ),
        )
        .await?;
        tasks_run += 1;

        let optimized_embeddings = self.optimize_missing_embeddings().await?;
        let vector_details =
            format!("Updated {optimized_embeddings} missing local semantic vectors.");
        self.insert_automation_task("Knowledge Builder", "Completed", &vector_details)
            .await?;
        tasks_run += 1;

        self.rebuild_knowledge_graph().await?;
        self.insert_automation_task(
            "Graph Engine",
            "Completed",
            "Refreshed graph entities and relationships.",
        )
        .await?;
        tasks_run += 1;

        let stats = self.stats().await?;
        let graph = self.knowledge_graph().await?;
        let duplicate_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE copy_count > 1;")
                .fetch_one(&self.pool)
                .await?;
        let missing_embeddings: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM clipboard_items WHERE embedding = '';")
                .fetch_one(&self.pool)
                .await?;

        self.insert_automation_task(
            "Learning Analyzer",
            "Completed",
            &format!(
                "{} memories, {} entities analyzed.",
                stats.total_items,
                graph.nodes.len()
            ),
        )
        .await?;
        tasks_run += 1;

        self.insert_automation_task(
            "Duplicate Cleaner",
            "Completed",
            &format!("{duplicate_count} repeated memories detected."),
        )
        .await?;
        tasks_run += 1;

        let backup_path = self.create_backup_snapshot("automated").await?.path;
        self.insert_automation_task("Backup Manager", "Completed", &backup_path)
            .await?;
        tasks_run += 1;

        let daily_bullets = vec![
            format!("{} total memories captured.", stats.total_items),
            format!(
                "{} connected entities in the knowledge graph.",
                graph.nodes.len()
            ),
            format!("{} graph relationships discovered.", graph.edges.len()),
            format!("{duplicate_count} memories have repeated copy activity."),
        ];
        self.insert_intelligence_report(
            "Daily",
            "Daily Intelligence Summary",
            "Autonomous agents reviewed current memory activity.",
            &daily_bullets,
        )
        .await?;
        reports_created += 1;

        let weekly_bullets = graph
            .clusters
            .iter()
            .take(5)
            .map(|cluster| format!("{}: {} weighted connections.", cluster.name, cluster.count))
            .collect::<Vec<_>>();
        self.insert_intelligence_report(
            "Weekly",
            "Weekly Intelligence Report",
            "Knowledge growth and active topic clusters were analyzed.",
            &weekly_bullets,
        )
        .await?;
        reports_created += 1;

        if duplicate_count > 0 {
            self.insert_notification(
                &format!("{duplicate_count} memories were copied more than once."),
                "Info",
            )
            .await?;
            notifications_created += 1;
        }

        if missing_embeddings > 0 {
            self.insert_notification(
                &format!("{missing_embeddings} memories still need semantic indexing."),
                "Warning",
            )
            .await?;
            notifications_created += 1;
        }

        if let Some(cluster) = graph.clusters.iter().max_by_key(|cluster| cluster.count) {
            self.insert_notification(
                &format!("Your most active knowledge cluster is {}.", cluster.name),
                "Info",
            )
            .await?;
            notifications_created += 1;
        }

        Ok(AutomationRunResult {
            tasks_run,
            reports_created,
            notifications_created,
            backup_path,
        })
    }

    pub async fn knowledge_health(&self) -> Result<KnowledgeHealth, sqlx::Error> {
        let stats = self.stats().await?;
        let connected_entities: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM graph_entities;")
            .fetch_one(&self.pool)
            .await?;
        let graph_relationships: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM graph_relationships;")
                .fetch_one(&self.pool)
                .await?;
        let active_projects: i64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT cluster) FROM graph_entities;")
                .fetch_one(&self.pool)
                .await?;
        let ai_activity: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM intelligence_reports;")
            .fetch_one(&self.pool)
            .await?;
        let background_tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM automation_tasks;")
            .fetch_one(&self.pool)
            .await?;
        let unread_notifications: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM smart_notifications WHERE is_read = 0;")
                .fetch_one(&self.pool)
                .await?;
        let storage_bytes = fs::metadata(&self.database_file)
            .ok()
            .map(|metadata| metadata.len() as i64)
            .unwrap_or(0);
        let storage_health = if storage_bytes < 50_000_000 {
            "Healthy"
        } else {
            "Review"
        }
        .to_string();

        Ok(KnowledgeHealth {
            total_memories: stats.total_items,
            connected_entities,
            graph_relationships,
            active_projects,
            ai_activity,
            background_tasks,
            unread_notifications,
            storage_bytes,
            storage_health,
            productivity_score: autonomous::productivity_score(
                stats.total_items,
                connected_entities,
                graph_relationships,
                background_tasks,
            ),
        })
    }

    pub async fn automation_tasks(&self) -> Result<Vec<AutomationTask>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, service, status, details, strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
            FROM automation_tasks
            ORDER BY id DESC
            LIMIT 20;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| AutomationTask {
                id: row.get("id"),
                service: row.get("service"),
                status: row.get("status"),
                details: row.get("details"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    pub async fn smart_notifications(&self) -> Result<Vec<SmartNotification>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, message, severity, is_read, strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
            FROM smart_notifications
            ORDER BY id DESC
            LIMIT 20;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| SmartNotification {
                id: row.get("id"),
                message: row.get("message"),
                severity: row.get("severity"),
                is_read: row.get::<i64, _>("is_read") == 1,
                created_at: row.get("created_at"),
            })
            .collect())
    }

    pub async fn intelligence_reports(&self) -> Result<Vec<IntelligenceReport>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, report_type, title, summary, bullets, strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
            FROM intelligence_reports
            ORDER BY id DESC
            LIMIT 20;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| IntelligenceReport {
                id: row.get("id"),
                report_type: row.get("report_type"),
                title: row.get("title"),
                summary: row.get("summary"),
                bullets: serde_json::from_str(&row.get::<String, _>("bullets")).unwrap_or_default(),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    pub async fn run_universal_sync_cycle(&self) -> Result<UniversalSyncResult, sqlx::Error> {
        let policy = self.team_sharing_policy().await?;
        let devices_checked: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_devices;")
            .fetch_one(&self.pool)
            .await?;
        let integrations_checked: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM integration_connectors;")
                .fetch_one(&self.pool)
                .await?;

        self.insert_audit_log(
            "system",
            "Platform readiness check",
            "No remote sync, external integrations, plugins, or public API are configured",
            "Info",
        )
        .await?;
        self.insert_automation_task(
            "Platform Readiness",
            if policy.enabled {
                "Policy ready"
            } else {
                "Local-only"
            },
            if policy.enabled {
                "Team sharing policy is configured locally. Remote synchronization is still disabled in this release."
            } else {
                "Remote sync and external integrations are not configured in this release."
            },
        )
        .await?;

        Ok(UniversalSyncResult {
            devices_checked,
            integrations_checked,
            events_recorded: 2,
            status: if policy.enabled {
                format!("Local sharing policy checked in {} mode", policy.mode)
            } else {
                "Local-only platform check complete".to_string()
            },
        })
    }

    pub async fn platform_summary(&self) -> Result<PlatformSummary, sqlx::Error> {
        let device_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_devices;")
            .fetch_one(&self.pool)
            .await?;
        let integration_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM integration_connectors;")
                .fetch_one(&self.pool)
                .await?;
        let active_plugins: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry WHERE status = 'Enabled';")
                .fetch_one(&self.pool)
                .await?;
        let api_clients: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM api_clients WHERE status = 'Active';")
                .fetch_one(&self.pool)
                .await?;
        let audit_events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs;")
            .fetch_one(&self.pool)
            .await?;
        let policy = self.team_sharing_policy().await?;
        Ok(PlatformSummary {
            sync_mode: if policy.enabled {
                policy.mode
            } else {
                "Local-first".to_string()
            },
            sync_status: if policy.enabled {
                "Policy Ready".to_string()
            } else {
                "Local Only".to_string()
            },
            device_count,
            integration_count,
            active_plugins,
            api_clients,
            audit_events,
            encryption_status: "Encryption roadmap".to_string(),
            retention_policy: "User controlled".to_string(),
            performance_score: platform::performance_score(
                device_count,
                integration_count,
                active_plugins,
                api_clients,
            ),
        })
    }

    pub async fn team_sharing_policy(&self) -> Result<TeamSharingPolicy, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                enabled,
                mode,
                allow_workspace_handoffs,
                allow_runbook_exports,
                allow_imported_references,
                require_device_approval,
                require_recipient_trust,
                retention_days,
                strftime('%Y-%m-%d %H:%M', updated_at, 'localtime') AS updated_at
            FROM team_sharing_policy
            WHERE id = 1;
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_team_sharing_policy).unwrap_or_default())
    }

    pub async fn update_team_sharing_policy(
        &self,
        policy: &TeamSharingPolicy,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO team_sharing_policy (
                id,
                enabled,
                mode,
                allow_workspace_handoffs,
                allow_runbook_exports,
                allow_imported_references,
                require_device_approval,
                require_recipient_trust,
                retention_days,
                updated_at
            )
            VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                enabled = excluded.enabled,
                mode = excluded.mode,
                allow_workspace_handoffs = excluded.allow_workspace_handoffs,
                allow_runbook_exports = excluded.allow_runbook_exports,
                allow_imported_references = excluded.allow_imported_references,
                require_device_approval = excluded.require_device_approval,
                require_recipient_trust = excluded.require_recipient_trust,
                retention_days = excluded.retention_days,
                updated_at = CURRENT_TIMESTAMP;
            "#,
        )
        .bind(policy.enabled)
        .bind(&policy.mode)
        .bind(policy.allow_workspace_handoffs)
        .bind(policy.allow_runbook_exports)
        .bind(policy.allow_imported_references)
        .bind(policy.require_device_approval)
        .bind(policy.require_recipient_trust)
        .bind(policy.retention_days)
        .execute(&self.pool)
        .await?;

        self.insert_audit_log(
            "system",
            "team_sharing.policy.updated",
            "Local team sharing policy updated",
            "Info",
        )
        .await?;
        self.prune_team_sharing_audit_logs().await
    }

    pub async fn team_sharing_readiness(&self) -> Result<TeamSharingReadiness, sqlx::Error> {
        let policy = self.team_sharing_policy().await?;
        let approved_devices: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sync_devices WHERE status = 'Approved';")
                .fetch_one(&self.pool)
                .await?;
        let trusted_recipients: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM handoff_recipient_trust WHERE is_active = 1;")
                .fetch_one(&self.pool)
                .await?;
        let trusted_signers: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM handoff_signer_trust WHERE is_active = 1;")
                .fetch_one(&self.pool)
                .await?;
        let checked_at: String = sqlx::query_scalar(
            "SELECT strftime('%Y-%m-%d %H:%M', CURRENT_TIMESTAMP, 'localtime');",
        )
        .fetch_one(&self.pool)
        .await?;
        let allowed_scopes = platform::sharing_allowed_scopes(&policy);
        let blockers = platform::sharing_blockers(
            &policy,
            approved_devices,
            trusted_recipients,
            &allowed_scopes,
        );
        let ready = blockers.is_empty();

        Ok(TeamSharingReadiness {
            ready,
            status: if ready {
                "Ready".to_string()
            } else if policy.enabled {
                "Blocked".to_string()
            } else {
                "Disabled".to_string()
            },
            mode: policy.mode,
            approved_devices,
            trusted_recipients,
            trusted_signers,
            allowed_scopes,
            blockers,
            checked_at,
        })
    }

    pub async fn team_sharing_sync_dry_run(&self) -> Result<TeamSharingSyncDryRun, sqlx::Error> {
        let policy = self.team_sharing_policy().await?;
        let readiness = self.team_sharing_readiness().await?;
        let workspace_handoffs: i64 = if policy.allow_workspace_handoffs {
            sqlx::query_scalar("SELECT COUNT(*) FROM workspace_handoff_exports;")
                .fetch_one(&self.pool)
                .await?
        } else {
            0
        };
        let workspace_handoff_bytes: i64 = if policy.allow_workspace_handoffs {
            sqlx::query_scalar(
                "SELECT COALESCE(SUM(package_bytes), 0) FROM workspace_handoff_exports;",
            )
            .fetch_one(&self.pool)
            .await?
        } else {
            0
        };
        let runbooks: i64 = if policy.allow_runbook_exports {
            sqlx::query_scalar("SELECT COUNT(*) FROM manual_runbooks;")
                .fetch_one(&self.pool)
                .await?
        } else {
            0
        };
        let runbook_bytes: i64 = if policy.allow_runbook_exports {
            sqlx::query_scalar(
                "SELECT COALESCE(SUM(length(title) + length(details)), 0) FROM manual_runbooks;",
            )
            .fetch_one(&self.pool)
            .await?
        } else {
            0
        };
        let imported_references: i64 = if policy.allow_imported_references {
            sqlx::query_scalar("SELECT COUNT(*) FROM workspace_handoff_imports;")
                .fetch_one(&self.pool)
                .await?
        } else {
            0
        };
        let generated_at: String = sqlx::query_scalar(
            "SELECT strftime('%Y-%m-%d %H:%M', CURRENT_TIMESTAMP, 'localtime');",
        )
        .fetch_one(&self.pool)
        .await?;
        let estimated_records = workspace_handoffs + runbooks + imported_references;
        let estimated_bytes = workspace_handoff_bytes + runbook_bytes;

        self.insert_audit_log(
            "system",
            "team_sharing.sync_dry_run",
            "Local sync dry-run manifest generated",
            "Info",
        )
        .await?;
        self.prune_team_sharing_audit_logs().await?;

        Ok(TeamSharingSyncDryRun {
            ready: readiness.ready,
            status: if readiness.ready {
                "Ready".to_string()
            } else {
                "Blocked".to_string()
            },
            mode: readiness.mode,
            eligible_devices: readiness.approved_devices,
            eligible_scopes: readiness.allowed_scopes,
            estimated_records,
            estimated_bytes,
            blockers: readiness.blockers,
            generated_at,
        })
    }

    pub async fn sync_devices(&self) -> Result<Vec<SyncDevice>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, device_name, platform, sync_mode, status, strftime('%Y-%m-%d %H:%M', last_seen_at, 'localtime') AS last_seen_at
            FROM sync_devices
            ORDER BY id;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| SyncDevice {
                id: row.get("id"),
                device_name: row.get("device_name"),
                platform: row.get("platform"),
                sync_mode: row.get("sync_mode"),
                status: row.get("status"),
                last_seen_at: row.get("last_seen_at"),
            })
            .collect())
    }

    pub async fn register_team_sharing_device(
        &self,
        request: &TeamSharingDeviceRequest,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO sync_devices (device_name, platform, sync_mode, status, last_seen_at)
            VALUES (?1, ?2, ?3, 'Pending approval', CURRENT_TIMESTAMP)
            ON CONFLICT(device_name) DO UPDATE SET
                platform = excluded.platform,
                sync_mode = excluded.sync_mode,
                status = CASE
                    WHEN sync_devices.status = 'Approved' THEN sync_devices.status
                    ELSE 'Pending approval'
                END,
                last_seen_at = CURRENT_TIMESTAMP;
            "#,
        )
        .bind(request.device_name.trim())
        .bind(request.platform.trim())
        .bind(request.sync_mode.trim())
        .execute(&self.pool)
        .await?;

        self.insert_audit_log(
            "system",
            "team_sharing.device.registered",
            "Local sharing device record registered",
            "Info",
        )
        .await?;
        self.prune_team_sharing_audit_logs().await
    }

    pub async fn approve_team_sharing_device(
        &self,
        request: &TeamSharingDeviceStatusRequest,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE sync_devices SET status = 'Approved', last_seen_at = CURRENT_TIMESTAMP WHERE id = ?1;",
        )
        .bind(request.device_id)
        .execute(&self.pool)
        .await?;

        self.insert_audit_log(
            "system",
            "team_sharing.device.approved",
            "Local sharing device approved",
            "Info",
        )
        .await?;
        self.prune_team_sharing_audit_logs().await
    }

    pub async fn revoke_team_sharing_device(
        &self,
        request: &TeamSharingDeviceStatusRequest,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE sync_devices SET status = 'Revoked', last_seen_at = CURRENT_TIMESTAMP WHERE id = ?1;",
        )
        .bind(request.device_id)
        .execute(&self.pool)
        .await?;

        self.insert_audit_log(
            "system",
            "team_sharing.device.revoked",
            "Local sharing device revoked",
            "Warning",
        )
        .await?;
        self.prune_team_sharing_audit_logs().await
    }

    pub async fn integration_connectors(&self) -> Result<Vec<IntegrationConnector>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, category, status, capabilities, strftime('%Y-%m-%d %H:%M', last_activity_at, 'localtime') AS last_activity_at
            FROM integration_connectors
            ORDER BY category, name;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| IntegrationConnector {
                id: row.get("id"),
                name: row.get("name"),
                category: row.get("category"),
                status: row.get("status"),
                capabilities: serde_json::from_str(&row.get::<String, _>("capabilities"))
                    .unwrap_or_default(),
                last_activity_at: row.get("last_activity_at"),
            })
            .collect())
    }

    pub async fn plugin_records(&self) -> Result<Vec<PluginRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, version, status, permissions, strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
            FROM plugin_registry
            ORDER BY id;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| PluginRecord {
                id: row.get("id"),
                name: row.get("name"),
                version: row.get("version"),
                status: row.get("status"),
                permissions: row.get("permissions"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    pub async fn api_clients(&self) -> Result<Vec<ApiClient>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, scope, status, strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
            FROM api_clients
            ORDER BY id;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ApiClient {
                id: row.get("id"),
                name: row.get("name"),
                scope: row.get("scope"),
                status: row.get("status"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    pub async fn audit_logs(&self) -> Result<Vec<AuditLog>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, actor, action, resource, severity, strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
            FROM audit_logs
            ORDER BY id DESC
            LIMIT 30;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| AuditLog {
                id: row.get("id"),
                actor: row.get("actor"),
                action: row.get("action"),
                resource: row.get("resource"),
                severity: row.get("severity"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    pub async fn runbook_audit_logs(&self) -> Result<Vec<AuditLog>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, actor, action, resource, severity,
                   strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
            FROM audit_logs
            WHERE action GLOB 'runbook.*'
            ORDER BY id DESC
            LIMIT 12;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_audit_log).collect())
    }

    pub async fn team_sharing_audit_logs(&self) -> Result<Vec<AuditLog>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, actor, action, resource, severity,
                   strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
            FROM audit_logs
            WHERE action GLOB 'team_sharing.*'
            ORDER BY id DESC
            LIMIT 20;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_audit_log).collect())
    }

    pub async fn team_sharing_manifest_ledger_audit_logs(
        &self,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, actor, action, resource, severity,
                   strftime('%Y-%m-%d %H:%M', created_at, 'localtime') AS created_at
            FROM audit_logs
            WHERE action GLOB 'team_sharing.manifest*'
            ORDER BY id DESC
            LIMIT 200;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_audit_log).collect())
    }

    pub async fn record_team_sharing_report_export(&self, path: &str) -> Result<(), sqlx::Error> {
        self.insert_audit_log("system", "team_sharing.report.exported", path, "Info")
            .await?;
        self.prune_team_sharing_audit_logs().await
    }

    pub async fn record_team_sharing_manifest_export(&self, path: &str) -> Result<(), sqlx::Error> {
        self.insert_audit_log("system", "team_sharing.manifest.exported", path, "Info")
            .await?;
        self.prune_team_sharing_audit_logs().await
    }

    pub async fn record_team_sharing_manifest_inspection(
        &self,
        status: &str,
        resource: &str,
    ) -> Result<(), sqlx::Error> {
        self.insert_audit_log(
            "system",
            "team_sharing.manifest.inspected",
            resource,
            if status == "Verified" {
                "Info"
            } else {
                "Warning"
            },
        )
        .await?;
        self.prune_team_sharing_audit_logs().await
    }

    pub async fn record_team_sharing_manifest_ledger_export(
        &self,
        path: &str,
    ) -> Result<(), sqlx::Error> {
        self.insert_audit_log(
            "system",
            "team_sharing.manifest_ledger.exported",
            path,
            "Info",
        )
        .await?;
        self.prune_team_sharing_audit_logs().await
    }

    pub async fn record_team_sharing_filtered_manifest_ledger_export(
        &self,
        path: &str,
        filter: &str,
        matching_events: usize,
    ) -> Result<(), sqlx::Error> {
        let resource = format!("{path} - filter={filter} - matching_events={matching_events}");
        self.insert_audit_log(
            "system",
            "team_sharing.manifest_ledger.exported_filtered",
            &resource,
            "Info",
        )
        .await?;
        self.prune_team_sharing_audit_logs().await
    }

    async fn prune_team_sharing_audit_logs(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM audit_logs
            WHERE action GLOB 'team_sharing.*'
              AND id NOT IN (
                  SELECT id
                  FROM audit_logs
                  WHERE action GLOB 'team_sharing.*'
                  ORDER BY id DESC
                  LIMIT 200
              );
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn run_cognitive_release_check(&self) -> Result<CognitiveReleaseResult, sqlx::Error> {
        let modules_verified: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cognitive_modules WHERE status = 'Operational';",
        )
        .fetch_one(&self.pool)
        .await?;
        let controls_verified: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM enterprise_controls WHERE status = 'Ready';")
                .fetch_one(&self.pool)
                .await?;
        let use_cases_verified: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cognitive_use_cases WHERE status = 'Supported';",
        )
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            r#"
            UPDATE cognitive_modules
            SET updated_at = CURRENT_TIMESTAMP
            WHERE status = 'Operational';
            "#,
        )
        .execute(&self.pool)
        .await?;

        self.insert_audit_log(
            "system",
            "Release readiness check",
            "Local foundation reviewed; enterprise release requirements remain",
            "Info",
        )
        .await?;
        self.insert_automation_task(
            "Release Readiness",
            "In progress",
            "Verified the local foundation. Sync, encryption, RBAC, plugins, and public APIs remain roadmap work.",
        )
        .await?;

        Ok(CognitiveReleaseResult {
            modules_verified,
            controls_verified,
            use_cases_verified,
            status: "Local foundation check complete; enterprise release is not yet ready"
                .to_string(),
        })
    }

    pub async fn cognitive_overview(&self) -> Result<CognitiveOverview, sqlx::Error> {
        let module_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cognitive_modules;")
            .fetch_one(&self.pool)
            .await?;
        let enterprise_controls: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM enterprise_controls;")
                .fetch_one(&self.pool)
                .await?;
        let use_case_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cognitive_use_cases;")
            .fetch_one(&self.pool)
            .await?;
        let platform = self.platform_summary().await?;
        let memory_score = pcos::memory_score(
            module_count,
            enterprise_controls,
            use_case_count,
            platform.performance_score,
        );
        let readiness_status = "Local foundation in progress";

        Ok(CognitiveOverview {
            release: "CYMOS local foundation".to_string(),
            tagline: "Remember Everything. Understand Everything. Accomplish Anything.".to_string(),
            readiness_status: readiness_status.to_string(),
            privacy_mode: "Local-first, user-owned".to_string(),
            memory_score,
            module_count,
            enterprise_controls,
            use_case_count,
        })
    }

    pub async fn cognitive_modules(&self) -> Result<Vec<CognitiveModule>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, layer, status, capabilities, strftime('%Y-%m-%d %H:%M', updated_at, 'localtime') AS updated_at
            FROM cognitive_modules
            ORDER BY id;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| CognitiveModule {
                id: row.get("id"),
                name: row.get("name"),
                layer: row.get("layer"),
                status: row.get("status"),
                capabilities: serde_json::from_str(&row.get::<String, _>("capabilities"))
                    .unwrap_or_default(),
                updated_at: row.get("updated_at"),
            })
            .collect())
    }

    pub async fn enterprise_controls(&self) -> Result<Vec<EnterpriseControl>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, status, scope, strftime('%Y-%m-%d %H:%M', updated_at, 'localtime') AS updated_at
            FROM enterprise_controls
            ORDER BY id;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| EnterpriseControl {
                id: row.get("id"),
                name: row.get("name"),
                status: row.get("status"),
                scope: row.get("scope"),
                updated_at: row.get("updated_at"),
            })
            .collect())
    }

    pub async fn cognitive_use_cases(&self) -> Result<Vec<CognitiveUseCase>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, audience, workflow, status
            FROM cognitive_use_cases
            ORDER BY id;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| CognitiveUseCase {
                id: row.get("id"),
                audience: row.get("audience"),
                workflow: row.get("workflow"),
                status: row.get("status"),
            })
            .collect())
    }

    async fn optimize_missing_embeddings(&self) -> Result<i64, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, content, content_type, language, ai_summary, category, keywords
            FROM clipboard_items
            WHERE embedding = '' OR semantic_text = ''
            LIMIT 100;
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut updated = 0;

        for row in rows {
            let id: i64 = row.get("id");
            let content: String = row.get("content");
            let content_type: String = row.get("content_type");
            let language: Option<String> = row.get("language");
            let summary: String = row.get("ai_summary");
            let category: String = row.get("category");
            let keyword_csv: String = row.get("keywords");
            let keywords = keyword_csv
                .split(',')
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            let semantic_text = semantic::semantic_text(
                &content,
                &content_type,
                language.as_deref(),
                &summary,
                &category,
                &keywords,
                &[],
            );
            let embedding = semantic::local_embedding(&semantic_text);

            sqlx::query(
                r#"
                UPDATE clipboard_items
                SET semantic_text = ?1,
                    embedding = ?2,
                    embedding_source = 'Local',
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = ?3;
                "#,
            )
            .bind(semantic_text)
            .bind(semantic::serialize_embedding(&embedding))
            .bind(id)
            .execute(&self.pool)
            .await?;

            updated += 1;
        }

        Ok(updated)
    }

    async fn insert_automation_task(
        &self,
        service: &str,
        status: &str,
        details: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO automation_tasks (service, status, details)
            VALUES (?1, ?2, ?3);
            "#,
        )
        .bind(service)
        .bind(status)
        .bind(details)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn insert_notification(&self, message: &str, severity: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO smart_notifications (message, severity)
            VALUES (?1, ?2);
            "#,
        )
        .bind(message)
        .bind(severity)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn insert_audit_log(
        &self,
        actor: &str,
        action: &str,
        resource: &str,
        severity: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO audit_logs (actor, action, resource, severity)
            VALUES (?1, ?2, ?3, ?4);
            "#,
        )
        .bind(actor)
        .bind(action)
        .bind(resource)
        .bind(severity)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn insert_intelligence_report(
        &self,
        report_type: &str,
        title: &str,
        summary: &str,
        bullets: &[String],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO intelligence_reports (report_type, title, summary, bullets)
            VALUES (?1, ?2, ?3, ?4);
            "#,
        )
        .bind(report_type)
        .bind(title)
        .bind(summary)
        .bind(serde_json::to_string(bullets).unwrap_or_else(|_| "[]".to_string()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_backup_snapshot(&self, kind: &str) -> Result<DatabaseBackup, sqlx::Error> {
        let (integrity_status, foreign_key_issues) = self.check_database_integrity().await?;
        if integrity_status != "Healthy" || foreign_key_issues > 0 {
            return Err(sqlx::Error::Protocol(
                "Database integrity verification failed; backup was not created.".to_string(),
            ));
        }

        let backup_dir = self.data_dir.join("backups");
        fs::create_dir_all(&backup_dir).map_err(sqlx::Error::Io)?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let destination = backup_dir.join(format!(
            "cymos-{kind}-backup-{timestamp}-{}.db",
            std::process::id()
        ));
        let destination_sql = destination.to_string_lossy().replace('\'', "''");

        // VACUUM INTO takes a consistent SQLite snapshot, unlike copying a live WAL database.
        sqlx::query(&format!("VACUUM INTO '{destination_sql}';"))
            .execute(&self.pool)
            .await?;
        let verified = verify_backup_file(&destination).await?;
        if !verified {
            return Err(sqlx::Error::Protocol(
                "Backup integrity verification failed.".to_string(),
            ));
        }
        if kind == "automated" {
            prune_automated_backups(&backup_dir).map_err(sqlx::Error::Io)?;
        }

        Ok(DatabaseBackup {
            path: destination.to_string_lossy().to_string(),
            verified,
            backup_count: self.backup_files()?.len() as i64,
        })
    }

    async fn check_database_integrity(&self) -> Result<(String, i64), sqlx::Error> {
        let integrity_rows = sqlx::query_scalar::<_, String>("PRAGMA integrity_check;")
            .fetch_all(&self.pool)
            .await?;
        let integrity_status = if integrity_rows.len() == 1
            && integrity_rows
                .first()
                .is_some_and(|result| result.eq_ignore_ascii_case("ok"))
        {
            "Healthy".to_string()
        } else {
            "Needs attention".to_string()
        };
        let foreign_key_issues = sqlx::query("PRAGMA foreign_key_check;")
            .fetch_all(&self.pool)
            .await?
            .len() as i64;
        Ok((integrity_status, foreign_key_issues))
    }

    fn backup_files(&self) -> Result<Vec<PathBuf>, sqlx::Error> {
        let backup_dir = self.data_dir.join("backups");
        if !backup_dir.exists() {
            return Ok(Vec::new());
        }

        let mut backups = fs::read_dir(&backup_dir)
            .map_err(sqlx::Error::Io)?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_type()
                    .map(|kind| kind.is_file())
                    .unwrap_or(false)
                    && entry.file_name().to_string_lossy().ends_with(".db")
                    && entry.file_name().to_string_lossy().starts_with("cymos-")
            })
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        backups.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
        Ok(backups)
    }

    async fn record_duplicate_by_hash(&self, content_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE clipboard_items
            SET copy_count = copy_count + 1,
                last_copied_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP
            WHERE content_hash = ?1;
            "#,
        )
        .bind(content_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_tags(&self, clipboard_id: i64, tags: Vec<String>) -> Result<(), sqlx::Error> {
        for tag in tags {
            let clean = tag.trim();
            if clean.is_empty() {
                continue;
            }

            sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES (?1);")
                .bind(clean)
                .execute(&self.pool)
                .await?;

            let tag_id: i64 = sqlx::query_scalar("SELECT id FROM tags WHERE name = ?1;")
                .bind(clean)
                .fetch_one(&self.pool)
                .await?;

            sqlx::query(
                "INSERT OR IGNORE INTO clipboard_tags (clipboard_id, tag_id) VALUES (?1, ?2);",
            )
            .bind(clipboard_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS clipboard_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                content_type TEXT NOT NULL,
                source_application TEXT NOT NULL DEFAULT 'Unknown',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                content_hash TEXT NOT NULL DEFAULT '',
                character_count INTEGER NOT NULL DEFAULT 0,
                word_count INTEGER NOT NULL DEFAULT 0,
                file_size INTEGER,
                image_width INTEGER,
                image_height INTEGER,
                language TEXT,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                collection_id INTEGER,
                ai_summary TEXT NOT NULL DEFAULT '',
                category TEXT NOT NULL DEFAULT 'Uncategorized',
                keywords TEXT NOT NULL DEFAULT '',
                reading_time_minutes INTEGER NOT NULL DEFAULT 1,
                copy_count INTEGER NOT NULL DEFAULT 1,
                last_copied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                semantic_text TEXT NOT NULL DEFAULT '',
                embedding TEXT NOT NULL DEFAULT '',
                embedding_source TEXT NOT NULL DEFAULT 'Local',
                operational_context TEXT NOT NULL DEFAULT '{}'
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        for (name, definition) in [
            ("source_application", "TEXT NOT NULL DEFAULT 'Unknown'"),
            ("updated_at", "TIMESTAMP DEFAULT CURRENT_TIMESTAMP"),
            ("content_hash", "TEXT NOT NULL DEFAULT ''"),
            ("character_count", "INTEGER NOT NULL DEFAULT 0"),
            ("word_count", "INTEGER NOT NULL DEFAULT 0"),
            ("file_size", "INTEGER"),
            ("image_width", "INTEGER"),
            ("image_height", "INTEGER"),
            ("language", "TEXT"),
            ("is_favorite", "INTEGER NOT NULL DEFAULT 0"),
            ("collection_id", "INTEGER"),
            ("ai_summary", "TEXT NOT NULL DEFAULT ''"),
            ("category", "TEXT NOT NULL DEFAULT 'Uncategorized'"),
            ("keywords", "TEXT NOT NULL DEFAULT ''"),
            ("reading_time_minutes", "INTEGER NOT NULL DEFAULT 1"),
            ("copy_count", "INTEGER NOT NULL DEFAULT 1"),
            ("last_copied_at", "TEXT NOT NULL DEFAULT ''"),
            ("semantic_text", "TEXT NOT NULL DEFAULT ''"),
            ("embedding", "TEXT NOT NULL DEFAULT ''"),
            ("embedding_source", "TEXT NOT NULL DEFAULT 'Local'"),
            ("operational_context", "TEXT NOT NULL DEFAULT '{}'"),
        ] {
            self.add_column_if_missing("clipboard_items", name, definition)
                .await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS collections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                color TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS clipboard_tags (
                clipboard_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (clipboard_id, tag_id)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS graph_entities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                entity_type TEXT NOT NULL,
                weight INTEGER NOT NULL DEFAULT 1,
                cluster TEXT NOT NULL DEFAULT 'General Knowledge',
                first_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                last_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS memory_entities (
                clipboard_id INTEGER NOT NULL,
                entity_id INTEGER NOT NULL,
                PRIMARY KEY (clipboard_id, entity_id)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS graph_relationships (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_entity_id INTEGER NOT NULL,
                target_entity_id INTEGER NOT NULL,
                relationship TEXT NOT NULL,
                weight INTEGER NOT NULL DEFAULT 1,
                last_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(source_entity_id, target_entity_id, relationship)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_workflows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                goal TEXT NOT NULL,
                status TEXT NOT NULL,
                agents TEXT NOT NULL,
                plan TEXT NOT NULL,
                answer TEXT NOT NULL,
                recommendations TEXT NOT NULL,
                context_memory_ids TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workflow_id INTEGER NOT NULL,
                agent TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS automation_tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service TEXT NOT NULL,
                status TEXT NOT NULL,
                details TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS smart_notifications (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message TEXT NOT NULL,
                severity TEXT NOT NULL,
                is_read INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS intelligence_reports (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                report_type TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                bullets TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sync_devices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_name TEXT NOT NULL UNIQUE,
                platform TEXT NOT NULL,
                sync_mode TEXT NOT NULL,
                status TEXT NOT NULL,
                last_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS team_sharing_policy (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                enabled INTEGER NOT NULL DEFAULT 0,
                mode TEXT NOT NULL DEFAULT 'LocalOnly',
                allow_workspace_handoffs INTEGER NOT NULL DEFAULT 1,
                allow_runbook_exports INTEGER NOT NULL DEFAULT 1,
                allow_imported_references INTEGER NOT NULL DEFAULT 0,
                require_device_approval INTEGER NOT NULL DEFAULT 1,
                require_recipient_trust INTEGER NOT NULL DEFAULT 1,
                retention_days INTEGER NOT NULL DEFAULT 30,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS integration_connectors (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                category TEXT NOT NULL,
                status TEXT NOT NULL,
                capabilities TEXT NOT NULL,
                last_activity_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS plugin_registry (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                version TEXT NOT NULL,
                status TEXT NOT NULL,
                permissions TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_clients (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                scope TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                actor TEXT NOT NULL,
                action TEXT NOT NULL,
                resource TEXT NOT NULL,
                severity TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cognitive_modules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                layer TEXT NOT NULL,
                status TEXT NOT NULL,
                capabilities TEXT NOT NULL,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS enterprise_controls (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL,
                scope TEXT NOT NULL,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cognitive_use_cases (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                audience TEXT NOT NULL,
                workflow TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS insight_trail_settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                enabled INTEGER NOT NULL DEFAULT 1,
                capture_clipboard INTEGER NOT NULL DEFAULT 1,
                capture_terminal_history INTEGER NOT NULL DEFAULT 1,
                capture_copied_images INTEGER NOT NULL DEFAULT 1,
                create_incidents INTEGER NOT NULL DEFAULT 1,
                retention_days INTEGER NOT NULL DEFAULT 30,
                max_storage_mb INTEGER NOT NULL DEFAULT 512,
                excluded_applications TEXT NOT NULL DEFAULT '[]',
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS insight_incidents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                signature TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Open',
                summary TEXT NOT NULL,
                recommended_steps TEXT NOT NULL DEFAULT '[]',
                event_count INTEGER NOT NULL DEFAULT 1,
                first_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                last_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                resolved_at TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS insight_trail_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                title TEXT NOT NULL,
                details TEXT NOT NULL,
                source_application TEXT NOT NULL DEFAULT 'Unknown',
                severity TEXT NOT NULL DEFAULT 'Info',
                memory_id INTEGER,
                screenshot_path TEXT,
                incident_id INTEGER,
                tags TEXT NOT NULL DEFAULT '',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(memory_id) REFERENCES clipboard_items(id) ON DELETE SET NULL,
                FOREIGN KEY(incident_id) REFERENCES insight_incidents(id) ON DELETE SET NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workspace_context (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                name TEXT NOT NULL,
                project TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Active',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cognitive_workspaces (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                project TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Ready',
                is_imported INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        self.add_column_if_missing(
            "cognitive_workspaces",
            "is_imported",
            "INTEGER NOT NULL DEFAULT 0",
        )
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workspace_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Active',
                started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                ended_at TIMESTAMP,
                FOREIGN KEY(workspace_id) REFERENCES cognitive_workspaces(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workspace_handoff_imports (
                workspace_id INTEGER PRIMARY KEY,
                source_workspace TEXT NOT NULL,
                source_project TEXT NOT NULL,
                source_scope TEXT NOT NULL,
                source_recipient TEXT NOT NULL DEFAULT 'Unspecified',
                source_purpose TEXT NOT NULL DEFAULT 'Unspecified',
                source_classification TEXT NOT NULL DEFAULT 'Unspecified',
                source_expires_at_unix INTEGER,
                source_signer_fingerprint TEXT,
                source_generated_at TEXT NOT NULL,
                checksum TEXT NOT NULL UNIQUE,
                imported_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(workspace_id) REFERENCES cognitive_workspaces(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        for (name, definition) in [
            ("source_recipient", "TEXT NOT NULL DEFAULT 'Unspecified'"),
            ("source_purpose", "TEXT NOT NULL DEFAULT 'Unspecified'"),
            (
                "source_classification",
                "TEXT NOT NULL DEFAULT 'Unspecified'",
            ),
            ("source_expires_at_unix", "INTEGER"),
            ("source_signer_fingerprint", "TEXT"),
        ] {
            self.add_column_if_missing("workspace_handoff_imports", name, definition)
                .await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workspace_handoff_exports (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace_id INTEGER NOT NULL,
                session_id INTEGER,
                scope TEXT NOT NULL,
                recipient TEXT NOT NULL DEFAULT 'Local review',
                purpose TEXT NOT NULL DEFAULT 'Operational handoff',
                classification TEXT NOT NULL DEFAULT 'Internal',
                expires_at_unix INTEGER,
                signer_fingerprint TEXT NOT NULL DEFAULT 'Unsigned legacy package',
                package_sha256 TEXT NOT NULL,
                package_bytes INTEGER NOT NULL,
                event_count INTEGER NOT NULL,
                excluded_event_count INTEGER NOT NULL DEFAULT 0,
                incident_count INTEGER NOT NULL,
                resolution_count INTEGER NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(workspace_id) REFERENCES cognitive_workspaces(id) ON DELETE CASCADE,
                FOREIGN KEY(session_id) REFERENCES workspace_sessions(id) ON DELETE SET NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        self.add_column_if_missing(
            "workspace_handoff_exports",
            "excluded_event_count",
            "INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        for (name, definition) in [
            ("recipient", "TEXT NOT NULL DEFAULT 'Local review'"),
            ("purpose", "TEXT NOT NULL DEFAULT 'Operational handoff'"),
            ("classification", "TEXT NOT NULL DEFAULT 'Internal'"),
            ("expires_at_unix", "INTEGER"),
            (
                "signer_fingerprint",
                "TEXT NOT NULL DEFAULT 'Unsigned legacy package'",
            ),
        ] {
            self.add_column_if_missing("workspace_handoff_exports", name, definition)
                .await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS handoff_recipient_trust (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                recipient TEXT NOT NULL COLLATE NOCASE UNIQUE,
                max_classification TEXT NOT NULL DEFAULT 'Internal',
                note TEXT NOT NULL DEFAULT '',
                is_active INTEGER NOT NULL DEFAULT 1,
                export_count INTEGER NOT NULL DEFAULT 0,
                last_used_at TIMESTAMP,
                revoked_at TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        for (name, definition) in [
            ("max_classification", "TEXT NOT NULL DEFAULT 'Internal'"),
            ("note", "TEXT NOT NULL DEFAULT ''"),
            ("is_active", "INTEGER NOT NULL DEFAULT 1"),
            ("export_count", "INTEGER NOT NULL DEFAULT 0"),
            ("last_used_at", "TIMESTAMP"),
            ("revoked_at", "TIMESTAMP"),
        ] {
            self.add_column_if_missing("handoff_recipient_trust", name, definition)
                .await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS handoff_signer_trust (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                signer_fingerprint TEXT NOT NULL UNIQUE,
                label TEXT NOT NULL DEFAULT '',
                is_active INTEGER NOT NULL DEFAULT 1,
                import_count INTEGER NOT NULL DEFAULT 0,
                last_used_at TIMESTAMP,
                revoked_at TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        for (name, definition) in [
            ("label", "TEXT NOT NULL DEFAULT ''"),
            ("is_active", "INTEGER NOT NULL DEFAULT 1"),
            ("import_count", "INTEGER NOT NULL DEFAULT 0"),
            ("last_used_at", "TIMESTAMP"),
            ("revoked_at", "TIMESTAMP"),
        ] {
            self.add_column_if_missing("handoff_signer_trust", name, definition)
                .await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workspace_handoff_inspections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                status TEXT NOT NULL,
                workspace_name TEXT,
                classification TEXT,
                signer_fingerprint TEXT,
                package_sha256 TEXT NOT NULL,
                payload_sha256 TEXT,
                failure_reason TEXT,
                package_bytes INTEGER NOT NULL,
                inspected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        for (name, definition) in [
            ("workspace_name", "TEXT"),
            ("classification", "TEXT"),
            ("signer_fingerprint", "TEXT"),
            ("payload_sha256", "TEXT"),
            ("failure_reason", "TEXT"),
        ] {
            self.add_column_if_missing("workspace_handoff_inspections", name, definition)
                .await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workspace_event_links (
                workspace_id INTEGER NOT NULL,
                session_id INTEGER NOT NULL,
                insight_event_id INTEGER NOT NULL UNIQUE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(workspace_id, insight_event_id),
                FOREIGN KEY(workspace_id) REFERENCES cognitive_workspaces(id) ON DELETE CASCADE,
                FOREIGN KEY(session_id) REFERENCES workspace_sessions(id) ON DELETE CASCADE,
                FOREIGN KEY(insight_event_id) REFERENCES insight_trail_events(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS incident_resolutions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                incident_id INTEGER NOT NULL,
                workspace_id INTEGER NOT NULL,
                session_id INTEGER,
                title TEXT NOT NULL,
                details TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(incident_id) REFERENCES insight_incidents(id) ON DELETE CASCADE,
                FOREIGN KEY(workspace_id) REFERENCES cognitive_workspaces(id) ON DELETE CASCADE,
                FOREIGN KEY(session_id) REFERENCES workspace_sessions(id) ON DELETE SET NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS manual_runbooks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                details TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        self.add_column_if_missing("manual_runbooks", "last_reviewed_revision", "INTEGER")
            .await?;
        self.add_column_if_missing("manual_runbooks", "last_reviewed_at", "TIMESTAMP")
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS manual_runbook_revisions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                runbook_id INTEGER NOT NULL,
                revision INTEGER NOT NULL,
                title TEXT NOT NULL,
                details TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(runbook_id, revision),
                FOREIGN KEY(runbook_id) REFERENCES manual_runbooks(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS manual_runbook_reviews (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                runbook_id INTEGER NOT NULL,
                revision INTEGER NOT NULL,
                note TEXT NOT NULL DEFAULT '',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(runbook_id) REFERENCES manual_runbooks(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO manual_runbook_revisions (runbook_id, revision, title, details, tags)
            SELECT id, 1, title, details, tags
            FROM manual_runbooks
            WHERE NOT EXISTS (
                SELECT 1 FROM manual_runbook_revisions
                WHERE manual_runbook_revisions.runbook_id = manual_runbooks.id
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS privacy_settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                protection_enabled INTEGER NOT NULL DEFAULT 1,
                capture_text INTEGER NOT NULL DEFAULT 1,
                capture_images INTEGER NOT NULL DEFAULT 1,
                block_sensitive_text INTEGER NOT NULL DEFAULT 1,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS vault_retention_settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                retention_days INTEGER NOT NULL DEFAULT 365,
                max_items INTEGER NOT NULL DEFAULT 10000,
                max_storage_mb INTEGER NOT NULL DEFAULT 1024,
                preserve_favorites INTEGER NOT NULL DEFAULT 1,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS privacy_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                reason TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        self.seed_insight_trail().await?;
        self.seed_platform().await?;
        self.seed_pcos().await?;

        sqlx::query(
            "UPDATE clipboard_items SET content_hash = 'legacy-' || id WHERE content_hash = '';",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE clipboard_items SET last_copied_at = created_at WHERE last_copied_at = '';",
        )
        .execute(&self.pool)
        .await?;

        // Legacy releases allowed empty and duplicate hashes. Normalize those records before
        // enforcing idempotent capture with a uniqueness constraint.
        sqlx::query(
            "UPDATE clipboard_items SET content_hash = 'legacy-' || id WHERE content_hash = '';",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            UPDATE clipboard_items AS current
            SET content_hash = current.content_hash || '-legacy-' || current.id
            WHERE EXISTS (
                SELECT 1 FROM clipboard_items AS prior
                WHERE prior.content_hash = current.content_hash
                  AND prior.id < current.id
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_clipboard_hash ON clipboard_items(content_hash);",
        )
        .execute(&self.pool)
        .await?;

        // These indexes keep the interactive local vault responsive as history grows.
        for statement in [
            "CREATE INDEX IF NOT EXISTS idx_clipboard_created_at ON clipboard_items(created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_clipboard_type_created ON clipboard_items(content_type, created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_clipboard_collection ON clipboard_items(collection_id);",
            "CREATE INDEX IF NOT EXISTS idx_clipboard_category ON clipboard_items(category);",
            "CREATE INDEX IF NOT EXISTS idx_clipboard_favorite ON clipboard_items(is_favorite, created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_clipboard_tags_tag ON clipboard_tags(tag_id, clipboard_id);",
            "CREATE INDEX IF NOT EXISTS idx_memory_entities_entity ON memory_entities(entity_id, clipboard_id);",
            "CREATE INDEX IF NOT EXISTS idx_graph_relationships_source ON graph_relationships(source_entity_id, target_entity_id);",
            "CREATE INDEX IF NOT EXISTS idx_automation_created_at ON automation_tasks(created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_audit_created_at ON audit_logs(created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_insight_events_created_at ON insight_trail_events(created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_insight_events_type_created ON insight_trail_events(event_type, created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_insight_events_incident ON insight_trail_events(incident_id, created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_workspace_context_updated ON workspace_context(updated_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_cognitive_workspaces_updated ON cognitive_workspaces(updated_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_workspace_sessions_workspace ON workspace_sessions(workspace_id, started_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_workspace_links_workspace ON workspace_event_links(workspace_id, insight_event_id DESC);",
            "CREATE INDEX IF NOT EXISTS idx_workspace_links_session ON workspace_event_links(session_id, insight_event_id DESC);",
            "CREATE INDEX IF NOT EXISTS idx_workspace_handoff_exports_workspace ON workspace_handoff_exports(workspace_id, id DESC);",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_workspace_single_active_session ON workspace_sessions(status) WHERE status = 'Active';",
            "CREATE INDEX IF NOT EXISTS idx_incident_resolutions_workspace ON incident_resolutions(workspace_id, created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_incident_resolutions_incident ON incident_resolutions(incident_id, created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_privacy_events_created_at ON privacy_events(created_at DESC);",
        ] {
            sqlx::query(statement).execute(&self.pool).await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version TEXT PRIMARY KEY,
                applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-16-hardening');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-17-insight-trail');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-17-core-reliability');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-17-capture-privacy');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-17-cognitive-workspaces');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-17-incident-resolutions');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-18-vault-retention');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-18-handoff-export-audit');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-18-selective-handoff');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-18-handoff-declaration');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-18-handoff-expiry');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-18-handoff-signature');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-18-handoff-recipient-trust');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-18-handoff-recipient-revocation');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-18-handoff-inspection-ledger');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-19-handoff-signer-trust');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES ('2026-07-19-team-sharing-policy');",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn add_column_if_missing(
        &self,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<(), sqlx::Error> {
        let rows = sqlx::query(&format!("PRAGMA table_info({table});"))
            .fetch_all(&self.pool)
            .await?;
        let exists = rows
            .iter()
            .any(|row| row.get::<String, _>("name") == column);

        if !exists {
            sqlx::query(&format!(
                "ALTER TABLE {table} ADD COLUMN {column} {definition};"
            ))
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn seed_collections(&self) -> Result<(), sqlx::Error> {
        for (name, color) in [
            ("AI Research", "#0f766e"),
            ("CYMOS Development", "#7c3aed"),
            ("BSNL Exam", "#b45309"),
            ("Programming", "#2563eb"),
            ("Personal", "#be123c"),
            ("Office", "#52525b"),
        ] {
            self.create_collection(name, color).await?;
        }
        Ok(())
    }

    async fn seed_insight_trail(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO insight_trail_settings (
                id,
                enabled,
                capture_clipboard,
                capture_terminal_history,
                capture_copied_images,
                create_incidents,
                retention_days,
                max_storage_mb,
                excluded_applications
            )
            VALUES (1, 1, 1, 1, 1, 1, ?1, ?2, '[]');
            "#,
        )
        .bind(insight_trail::DEFAULT_RETENTION_DAYS)
        .bind(insight_trail::DEFAULT_MAX_STORAGE_MB)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn seed_workspace_context(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO workspace_context (id, name, project, status)
            VALUES (1, ?1, ?2, 'Active');
            "#,
        )
        .bind(workspace::DEFAULT_WORKSPACE_NAME)
        .bind(workspace::DEFAULT_WORKSPACE_PROJECT)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn seed_cognitive_workspaces(&self) -> Result<(), sqlx::Error> {
        let legacy = sqlx::query("SELECT name, project FROM workspace_context WHERE id = 1;")
            .fetch_one(&self.pool)
            .await?;
        let name: String = legacy.get("name");
        let project: String = legacy.get("project");

        sqlx::query(
            r#"INSERT OR IGNORE INTO cognitive_workspaces (id, name, project, status)
               VALUES (1, ?1, ?2, 'Active');"#,
        )
        .bind(name)
        .bind(project)
        .execute(&self.pool)
        .await?;

        let active_session_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM workspace_sessions WHERE status = 'Active';")
                .fetch_one(&self.pool)
                .await?;
        if active_session_count == 0 {
            sqlx::query(
                r#"INSERT INTO workspace_sessions (workspace_id, title, status)
                   VALUES (1, 'Current capture session', 'Active');"#,
            )
            .execute(&self.pool)
            .await?;
        }

        let session = sqlx::query(
            "SELECT id, workspace_id FROM workspace_sessions WHERE status = 'Active' LIMIT 1;",
        )
        .fetch_optional(&self.pool)
        .await?;
        if let Some(session) = session {
            sqlx::query(
                r#"INSERT OR IGNORE INTO workspace_event_links (workspace_id, session_id, insight_event_id)
                   SELECT ?1, ?2, id FROM insight_trail_events;"#,
            )
            .bind(session.get::<i64, _>("workspace_id"))
            .bind(session.get::<i64, _>("id"))
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn seed_privacy_settings(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO privacy_settings (
                id,
                protection_enabled,
                capture_text,
                capture_images,
                block_sensitive_text
            )
            VALUES (1, 1, 1, 1, 1);
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn seed_vault_retention_settings(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO vault_retention_settings (
                id,
                retention_days,
                max_items,
                max_storage_mb,
                preserve_favorites
            )
            VALUES (1, ?1, ?2, ?3, 1);
            "#,
        )
        .bind(DEFAULT_VAULT_RETENTION_DAYS)
        .bind(DEFAULT_VAULT_MAX_ITEMS)
        .bind(DEFAULT_VAULT_MAX_STORAGE_MB)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn seed_platform(&self) -> Result<(), sqlx::Error> {
        for (device_name, platform, sync_mode, status) in
            [("This Mac", "macOS", "Local-only", "Ready")]
        {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO sync_devices (device_name, platform, sync_mode, status)
                VALUES (?1, ?2, ?3, ?4);
                "#,
            )
            .bind(device_name)
            .bind(platform)
            .bind(sync_mode)
            .bind(status)
            .execute(&self.pool)
            .await?;
        }

        for (name, category, status, capabilities) in [
            (
                "Chrome Extension",
                "Browser",
                "Planned",
                vec!["Web pages", "AI chats", "Bookmarks", "Code snippets"],
            ),
            (
                "Firefox Extension",
                "Browser",
                "Planned",
                vec!["Articles", "Research", "Bookmarks"],
            ),
            (
                "VS Code",
                "Developer",
                "Planned",
                vec!["Commands", "Errors", "Project notes", "Code snippets"],
            ),
            (
                "Terminal",
                "Developer",
                "Planned",
                vec!["Shell commands", "Errors", "Run history"],
            ),
            (
                "Microsoft Office",
                "Productivity",
                "Planned",
                vec!["Word", "Excel", "PowerPoint", "Outlook"],
            ),
            (
                "PDF Readers",
                "Productivity",
                "Planned",
                vec!["PDF notes", "Highlights", "Research extracts"],
            ),
            (
                "GitHub",
                "Developer",
                "Planned",
                vec!["Repositories", "Issues", "Pull requests"],
            ),
        ] {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO integration_connectors (name, category, status, capabilities)
                VALUES (?1, ?2, ?3, ?4);
                "#,
            )
            .bind(name)
            .bind(category)
            .bind(status)
            .bind(serde_json::to_string(&capabilities).unwrap_or_else(|_| "[]".to_string()))
            .execute(&self.pool)
            .await?;
        }

        for (name, version, status, permissions) in [
            (
                "Memory Importer SDK",
                "0.0.9",
                "Planned",
                "memory:write,events:read",
            ),
            (
                "Workflow Automation SDK",
                "0.0.9",
                "Planned",
                "automation:run,memory:read",
            ),
            (
                "Enterprise Connector SDK",
                "0.0.9",
                "Planned",
                "audit:write,security:read",
            ),
        ] {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO plugin_registry (name, version, status, permissions)
                VALUES (?1, ?2, ?3, ?4);
                "#,
            )
            .bind(name)
            .bind(version)
            .bind(status)
            .bind(permissions)
            .execute(&self.pool)
            .await?;
        }

        for (name, scope, status) in [
            (
                "Local REST API",
                "search:read,memory:read,graph:read",
                "Not configured",
            ),
            (
                "Automation API",
                "agents:run,automation:run,analytics:read",
                "Not configured",
            ),
        ] {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO api_clients (name, scope, status)
                VALUES (?1, ?2, ?3);
                "#,
            )
            .bind(name)
            .bind(scope)
            .bind(status)
            .execute(&self.pool)
            .await?;
        }

        sqlx::query(
            r#"
            INSERT INTO audit_logs (actor, action, resource, severity)
            SELECT 'system', 'Platform migration', 'Local platform foundation and integration roadmap', 'Info'
            WHERE NOT EXISTS (
                SELECT 1 FROM audit_logs WHERE action = 'Platform migration'
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Preview builds seeded roadmap capabilities as active services. Keep existing local
        // data, but normalize those display states so the UI reflects deployed reality.
        sqlx::query(
            "DELETE FROM sync_devices WHERE device_name IN ('Windows Workstation', 'Linux Lab');",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("UPDATE integration_connectors SET status = 'Planned';")
            .execute(&self.pool)
            .await?;
        sqlx::query("UPDATE plugin_registry SET status = 'Planned';")
            .execute(&self.pool)
            .await?;
        sqlx::query("UPDATE api_clients SET status = 'Not configured';")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn seed_pcos(&self) -> Result<(), sqlx::Error> {
        for (name, layer, status, capabilities) in [
            (
                "Memory Engine",
                "Capture",
                "Operational",
                vec!["Clipboard", "Images", "Files", "HTML", "Code", "URLs"],
            ),
            (
                "AI Intelligence Engine",
                "Understanding",
                "Operational",
                vec!["Summaries", "Categorization", "Keywords", "Topic detection"],
            ),
            (
                "Semantic Memory Engine",
                "Retrieval",
                "Operational",
                vec![
                    "Embeddings",
                    "Hybrid search",
                    "Similar memories",
                    "Context retrieval",
                ],
            ),
            (
                "Knowledge Graph Engine",
                "Connection",
                "Operational",
                vec![
                    "Entities",
                    "Relationships",
                    "Topic clusters",
                    "Graph explorer",
                ],
            ),
            (
                "Memory Assistant",
                "Reasoning",
                "Operational",
                vec!["Natural questions", "Grounded answers", "Source retrieval"],
            ),
            (
                "Agent Intelligence",
                "Collaboration",
                "Operational",
                vec![
                    "Memory Agent",
                    "Research Agent",
                    "Planning Agent",
                    "Coding Agent",
                ],
            ),
            (
                "Autonomous Intelligence",
                "Automation",
                "Operational",
                vec![
                    "Daily summaries",
                    "Weekly reports",
                    "Backups",
                    "Health monitoring",
                ],
            ),
            (
                "Universal Platform",
                "Platform",
                "Operational",
                vec!["Sync readiness", "Integrations", "Plugin SDK", "Public API"],
            ),
        ] {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO cognitive_modules (name, layer, status, capabilities)
                VALUES (?1, ?2, ?3, ?4);
                "#,
            )
            .bind(name)
            .bind(layer)
            .bind(status)
            .bind(serde_json::to_string(&capabilities).unwrap_or_else(|_| "[]".to_string()))
            .execute(&self.pool)
            .await?;
        }
        sqlx::query(
            "UPDATE cognitive_modules SET status = 'Planned' WHERE name = 'Universal Platform';",
        )
        .execute(&self.pool)
        .await?;

        for (name, status, scope) in [
            (
                "End-to-End Encryption",
                "Ready",
                "Secure local vault and future sync",
            ),
            (
                "Offline-First Mode",
                "Ready",
                "Runs without mandatory cloud services",
            ),
            (
                "Self-Hosted Deployment",
                "Ready",
                "Private user or organization deployment",
            ),
            (
                "Team Workspaces",
                "Ready",
                "Enterprise collaboration foundation",
            ),
            (
                "Secure Synchronization",
                "Ready",
                "Local-only, self-hosted, encrypted cloud modes",
            ),
            ("Audit Logs", "Ready", "Security and platform events"),
            ("RBAC", "Ready", "Role-based access model foundation"),
            (
                "Zero-Trust Security",
                "Ready",
                "Least privilege platform posture",
            ),
            (
                "Enterprise APIs",
                "Ready",
                "Search, graph, agents, analytics, automation",
            ),
        ] {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO enterprise_controls (name, status, scope)
                VALUES (?1, ?2, ?3);
                "#,
            )
            .bind(name)
            .bind(status)
            .bind(scope)
            .execute(&self.pool)
            .await?;
        }
        sqlx::query(
            r#"
            UPDATE enterprise_controls
            SET status = CASE name
                WHEN 'Offline-First Mode' THEN 'Implemented'
                WHEN 'Audit Logs' THEN 'Limited'
                ELSE 'Roadmap'
            END;
            "#,
        )
        .execute(&self.pool)
        .await?;

        for (audience, workflow, status) in [
            (
                "Software Engineers",
                "Software development memory",
                "Supported",
            ),
            (
                "AI Engineers",
                "Prompt and model research library",
                "Supported",
            ),
            ("Students", "Exam preparation and revision", "Supported"),
            ("Researchers", "Research organization", "Supported"),
            ("Writers", "Long-term idea and note retrieval", "Supported"),
            (
                "Security Professionals",
                "Incident investigation memory",
                "Supported",
            ),
            (
                "Network Engineers",
                "Technical command and topology memory",
                "Supported",
            ),
            ("Enterprises", "Team knowledge base foundation", "Supported"),
        ] {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO cognitive_use_cases (audience, workflow, status)
                VALUES (?1, ?2, ?3);
                "#,
            )
            .bind(audience)
            .bind(workflow)
            .bind(status)
            .execute(&self.pool)
            .await?;
        }

        sqlx::query(
            r#"
            INSERT INTO audit_logs (actor, action, resource, severity)
            SELECT 'system', 'PCOS migration', 'v1.0 Personal Cognitive Operating System', 'Info'
            WHERE NOT EXISTS (
                SELECT 1 FROM audit_logs WHERE action = 'PCOS migration'
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

async fn connect_sqlite_pool(options: SqliteConnectOptions) -> Result<SqlitePool, sqlx::Error> {
    retry_transient(
        "sqlite_connect",
        SQLITE_CONNECT_ATTEMPTS,
        Duration::from_millis(100),
        || {
            let options = options.clone();
            async move {
                SqlitePoolOptions::new()
                    .max_connections(5)
                    .min_connections(1)
                    .acquire_timeout(Duration::from_secs(10))
                    .connect_with(options)
                    .await
            }
        },
        is_transient_sqlite_error,
    )
    .await
}

async fn retry_transient<T, E, Operation, Attempt, IsTransient>(
    operation_name: &str,
    max_attempts: u8,
    base_delay: Duration,
    mut operation: Operation,
    is_transient: IsTransient,
) -> Result<T, E>
where
    Operation: FnMut() -> Attempt,
    Attempt: Future<Output = Result<T, E>>,
    IsTransient: Fn(&E) -> bool,
{
    assert!(max_attempts > 0, "Retry attempts must be positive");
    for attempt in 1..=max_attempts {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) if attempt < max_attempts && is_transient(&error) => {
                let delay = base_delay.saturating_mul(2_u32.pow((attempt - 1).into()));
                eprintln!(
                    "{{\"event\":\"cymos.transient_retry\",\"operation\":\"{operation_name}\",\"attempt\":{attempt},\"delay_ms\":{}}}",
                    delay.as_millis()
                );
                sleep(delay).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("A positive retry count must return or fail")
}

fn is_transient_sqlite_error(error: &sqlx::Error) -> bool {
    let message = error.to_string().to_lowercase();
    message.contains("database is locked") || message.contains("database is busy")
}

fn push_filter(builder: &mut QueryBuilder<'_, Sqlite>, needs_where: &mut bool, sql: &str) {
    if *needs_where {
        builder.push(" WHERE ");
        *needs_where = false;
    } else {
        builder.push(" AND ");
    }
    builder.push(sql);
}

fn row_to_item(
    row: sqlx::sqlite::SqliteRow,
    query_embedding: Option<&[f32]>,
    query: &str,
) -> ClipboardItem {
    let tag_csv: String = row.get("tags");
    let embedding = semantic::deserialize_embedding(&row.get::<String, _>("embedding"));
    let semantic_score = rank_score(&row, query_embedding, &embedding, query);
    let rank_reason = rank_reason(&row, query_embedding, query);
    ClipboardItem {
        id: row.get("id"),
        content: row.get("content"),
        content_type: row.get("content_type"),
        source_application: row.get("source_application"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        content_hash: row.get("content_hash"),
        character_count: row.get("character_count"),
        word_count: row.get("word_count"),
        file_size: row.get("file_size"),
        image_width: row.get("image_width"),
        image_height: row.get("image_height"),
        language: row.get("language"),
        is_favorite: row.get::<i64, _>("is_favorite") == 1,
        collection_id: row.get("collection_id"),
        collection_name: row.get("collection_name"),
        collection_color: row.get("collection_color"),
        ai_summary: row.get("ai_summary"),
        category: row.get("category"),
        keywords: row
            .get::<String, _>("keywords")
            .split(',')
            .filter(|keyword| !keyword.is_empty())
            .map(ToString::to_string)
            .collect(),
        reading_time_minutes: row.get("reading_time_minutes"),
        copy_count: row.get("copy_count"),
        last_copied_at: row.get("last_copied_at"),
        semantic_score,
        rank_reason,
        embedding_source: row.get("embedding_source"),
        operational_context: serde_json::from_str(&row.get::<String, _>("operational_context"))
            .unwrap_or_default(),
        tags: tag_csv
            .split(',')
            .filter(|tag| !tag.is_empty())
            .map(ToString::to_string)
            .collect(),
    }
}

fn handoff_text(value: Option<&serde_json::Value>, fallback: &str) -> String {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn handoff_name(value: &str) -> String {
    value.chars().take(120).collect()
}

fn handoff_array(value: Option<&serde_json::Value>) -> Vec<&serde_json::Value> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn handoff_tags(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|tags| {
            tags.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .take(12)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn row_to_insight_event(row: sqlx::sqlite::SqliteRow) -> InsightTrailEvent {
    let tags = row.get::<String, _>("tags");
    InsightTrailEvent {
        id: row.get("id"),
        event_type: row.get("event_type"),
        title: row.get("title"),
        details: row.get("details"),
        source_application: row.get("source_application"),
        severity: row.get("severity"),
        created_at: row.get("created_at"),
        memory_id: row.get("memory_id"),
        screenshot_path: row.get("screenshot_path"),
        incident_id: row.get("incident_id"),
        session_id: row.get("session_id"),
        tags: tags
            .split(',')
            .filter(|tag| !tag.is_empty())
            .map(ToString::to_string)
            .collect(),
    }
}

fn row_to_incident_resolution(row: sqlx::sqlite::SqliteRow) -> IncidentResolution {
    IncidentResolution {
        id: row.get("id"),
        incident_id: row.get("incident_id"),
        workspace_id: row.get("workspace_id"),
        workspace_name: row.get("workspace_name"),
        session_id: row.get("session_id"),
        title: row.get("title"),
        details: row.get("details"),
        created_at: row.get("created_at"),
    }
}

fn row_to_runbook_entry(row: sqlx::sqlite::SqliteRow) -> RunbookEntry {
    let tags: String = row.get("tags");
    RunbookEntry {
        id: row.get("id"),
        incident_id: row.get("incident_id"),
        incident_title: row.get("incident_title"),
        workspace_name: row.get("workspace_name"),
        title: row.get("title"),
        details: row.get("details"),
        tags: tags
            .split(',')
            .filter(|tag| !tag.is_empty())
            .map(ToString::to_string)
            .collect(),
        created_at: row.get("created_at"),
        latest_revision: row.get("latest_revision"),
        last_reviewed_revision: row.get("last_reviewed_revision"),
        last_reviewed_at: row.get("last_reviewed_at"),
        last_review_note: row.get("last_review_note"),
        review_status: row.get("review_status"),
    }
}

fn row_to_audit_log(row: sqlx::sqlite::SqliteRow) -> AuditLog {
    AuditLog {
        id: row.get("id"),
        actor: row.get("actor"),
        action: row.get("action"),
        resource: row.get("resource"),
        severity: row.get("severity"),
        created_at: row.get("created_at"),
    }
}

fn row_to_team_sharing_policy(row: sqlx::sqlite::SqliteRow) -> TeamSharingPolicy {
    TeamSharingPolicy {
        enabled: row.get::<i64, _>("enabled") == 1,
        mode: row.get("mode"),
        allow_workspace_handoffs: row.get::<i64, _>("allow_workspace_handoffs") == 1,
        allow_runbook_exports: row.get::<i64, _>("allow_runbook_exports") == 1,
        allow_imported_references: row.get::<i64, _>("allow_imported_references") == 1,
        require_device_approval: row.get::<i64, _>("require_device_approval") == 1,
        require_recipient_trust: row.get::<i64, _>("require_recipient_trust") == 1,
        retention_days: row.get("retention_days"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_workspace_handoff_export(row: sqlx::sqlite::SqliteRow) -> WorkspaceHandoffExportRecord {
    WorkspaceHandoffExportRecord {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        session_id: row.get("session_id"),
        scope: row.get("scope"),
        recipient: row.get("recipient"),
        purpose: row.get("purpose"),
        classification: row.get("classification"),
        expires_at_unix: row.get("expires_at_unix"),
        signer_fingerprint: row.get("signer_fingerprint"),
        package_sha256: row.get("package_sha256"),
        package_bytes: row.get("package_bytes"),
        event_count: row.get("event_count"),
        excluded_event_count: row.get("excluded_event_count"),
        incident_count: row.get("incident_count"),
        resolution_count: row.get("resolution_count"),
        created_at: row.get("created_at"),
    }
}

fn row_to_workspace_handoff_inspection(
    row: sqlx::sqlite::SqliteRow,
) -> WorkspaceHandoffInspectionRecord {
    WorkspaceHandoffInspectionRecord {
        id: row.get("id"),
        status: row.get("status"),
        workspace_name: row.get("workspace_name"),
        classification: row.get("classification"),
        signer_fingerprint: row.get("signer_fingerprint"),
        package_sha256: row.get("package_sha256"),
        payload_sha256: row.get("payload_sha256"),
        failure_reason: row.get("failure_reason"),
        package_bytes: row.get("package_bytes"),
        inspected_at: row.get("inspected_at"),
    }
}

fn row_to_handoff_recipient_trust(row: sqlx::sqlite::SqliteRow) -> HandoffRecipientTrustRecord {
    HandoffRecipientTrustRecord {
        id: row.get("id"),
        recipient: row.get("recipient"),
        max_classification: row.get("max_classification"),
        note: row.get("note"),
        is_active: row.get::<i64, _>("is_active") == 1,
        export_count: row.get("export_count"),
        last_used_at: row.get("last_used_at"),
        revoked_at: row.get("revoked_at"),
        created_at: row.get("created_at"),
    }
}

fn row_to_handoff_signer_trust(row: sqlx::sqlite::SqliteRow) -> HandoffSignerTrustRecord {
    HandoffSignerTrustRecord {
        id: row.get("id"),
        signer_fingerprint: row.get("signer_fingerprint"),
        label: row.get("label"),
        is_active: row.get::<i64, _>("is_active") == 1,
        import_count: row.get("import_count"),
        last_used_at: row.get("last_used_at"),
        revoked_at: row.get("revoked_at"),
        created_at: row.get("created_at"),
    }
}

fn row_to_runbook_revision(row: sqlx::sqlite::SqliteRow) -> RunbookRevision {
    let tags: String = row.get("tags");
    RunbookRevision {
        id: row.get("id"),
        runbook_id: row.get("runbook_id"),
        revision: row.get("revision"),
        title: row.get("title"),
        details: row.get("details"),
        tags: tags
            .split(',')
            .filter(|tag| !tag.is_empty())
            .map(ToString::to_string)
            .collect(),
        created_at: row.get("created_at"),
    }
}

fn row_to_insight_settings(row: sqlx::sqlite::SqliteRow) -> InsightTrailSettings {
    let excluded = row.get::<String, _>("excluded_applications");
    InsightTrailSettings {
        enabled: row.get::<i64, _>("enabled") == 1,
        capture_clipboard: row.get::<i64, _>("capture_clipboard") == 1,
        capture_terminal_history: row.get::<i64, _>("capture_terminal_history") == 1,
        capture_copied_images: row.get::<i64, _>("capture_copied_images") == 1,
        create_incidents: row.get::<i64, _>("create_incidents") == 1,
        retention_days: row.get("retention_days"),
        max_storage_mb: row.get("max_storage_mb"),
        excluded_applications: serde_json::from_str(&excluded).unwrap_or_default(),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_vault_retention_settings(row: sqlx::sqlite::SqliteRow) -> VaultRetentionSettings {
    VaultRetentionSettings {
        retention_days: row.get("retention_days"),
        max_items: row.get("max_items"),
        max_storage_mb: row.get("max_storage_mb"),
        preserve_favorites: row.get::<i64, _>("preserve_favorites") == 1,
        updated_at: row.get("updated_at"),
    }
}

fn row_to_insight_incident(row: sqlx::sqlite::SqliteRow) -> InsightIncident {
    let steps = row.get::<String, _>("recommended_steps");
    InsightIncident {
        id: row.get("id"),
        title: row.get("title"),
        status: row.get("status"),
        summary: row.get("summary"),
        first_seen_at: row.get("first_seen_at"),
        last_seen_at: row.get("last_seen_at"),
        event_count: row.get("event_count"),
        recommended_steps: serde_json::from_str(&steps).unwrap_or_default(),
    }
}

fn rank_score(
    row: &sqlx::sqlite::SqliteRow,
    query_embedding: Option<&[f32]>,
    embedding: &[f32],
    query: &str,
) -> f32 {
    if query.is_empty() {
        return recency_frequency_score(row);
    }

    let lower_query = query.to_lowercase();
    let content = row.get::<String, _>("content").to_lowercase();
    let summary = row.get::<String, _>("ai_summary").to_lowercase();
    let keywords = row.get::<String, _>("keywords").to_lowercase();
    let category = row.get::<String, _>("category").to_lowercase();

    let keyword_score = [
        content.contains(&lower_query),
        summary.contains(&lower_query),
        keywords.contains(&lower_query),
        category.contains(&lower_query),
    ]
    .iter()
    .filter(|value| **value)
    .count() as f32
        * 0.18;

    let semantic_score = query_embedding
        .map(|query| semantic::cosine_similarity(query, embedding).max(0.0))
        .unwrap_or(0.0)
        * 0.62;

    semantic_score + keyword_score + recency_frequency_score(row)
}

fn recency_frequency_score(row: &sqlx::sqlite::SqliteRow) -> f32 {
    let favorite = if row.get::<i64, _>("is_favorite") == 1 {
        0.08
    } else {
        0.0
    };
    let copies = (row.get::<i64, _>("copy_count") as f32).ln_1p().min(2.0) * 0.05;
    favorite + copies
}

fn rank_reason(
    row: &sqlx::sqlite::SqliteRow,
    query_embedding: Option<&[f32]>,
    query: &str,
) -> String {
    if query.is_empty() {
        return "Recent memory".to_string();
    }

    let lower_query = query.to_lowercase();
    let content = row.get::<String, _>("content").to_lowercase();
    let summary = row.get::<String, _>("ai_summary").to_lowercase();
    let keywords = row.get::<String, _>("keywords").to_lowercase();

    if content.contains(&lower_query) {
        "Keyword match".to_string()
    } else if summary.contains(&lower_query) || keywords.contains(&lower_query) {
        "Metadata match".to_string()
    } else if query_embedding.is_some() {
        "Semantic similarity".to_string()
    } else {
        "Hybrid rank".to_string()
    }
}

fn legacy_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri must live inside the project root")
        .join("database")
}

fn migrate_legacy_local_data(data_dir: &PathBuf, database_file: &PathBuf) -> std::io::Result<()> {
    let legacy_dir = legacy_data_dir();
    let legacy_database = legacy_dir.join("cymos.db");

    if !database_file.exists() && legacy_database.exists() {
        fs::copy(&legacy_database, database_file)?;
        for suffix in ["-wal", "-shm"] {
            let source = PathBuf::from(format!("{}{}", legacy_database.to_string_lossy(), suffix));
            if source.exists() {
                let target =
                    PathBuf::from(format!("{}{}", database_file.to_string_lossy(), suffix));
                fs::copy(source, target)?;
            }
        }
    }

    let legacy_assets = legacy_dir.join("assets");
    let assets_dir = data_dir.join("assets");
    if legacy_assets.is_dir() && !assets_dir.exists() {
        fs::create_dir_all(&assets_dir)?;
        for entry in fs::read_dir(legacy_assets)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::copy(entry.path(), assets_dir.join(entry.file_name()))?;
            }
        }
    }

    Ok(())
}

fn prune_automated_backups(backup_dir: &PathBuf) -> std::io::Result<()> {
    let mut backups = fs::read_dir(backup_dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("cymos-automated-backup-")
        })
        .collect::<Vec<_>>();
    backups.sort_by_key(|entry| entry.file_name());

    let excess = backups.len().saturating_sub(MAX_AUTOMATED_BACKUPS);
    for entry in backups.into_iter().take(excess) {
        fs::remove_file(entry.path())?;
    }
    Ok(())
}

async fn verify_backup_file(path: &PathBuf) -> Result<bool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.to_string_lossy()))?
        .read_only(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(options)
        .await?;
    let integrity_rows = sqlx::query_scalar::<_, String>("PRAGMA integrity_check;")
        .fetch_all(&pool)
        .await?;
    let foreign_key_issues = sqlx::query("PRAGMA foreign_key_check;")
        .fetch_all(&pool)
        .await?
        .len();
    pool.close().await;

    Ok(integrity_rows.len() == 1
        && integrity_rows
            .first()
            .is_some_and(|result| result.eq_ignore_ascii_case("ok"))
        && foreign_key_issues == 0)
}

#[cfg(test)]
mod tests {
    use super::{
        retry_transient, Database, InsightTrailSearchRequest, NewClipboardItem,
        TeamSharingDeviceRequest, TeamSharingDeviceStatusRequest, TeamSharingPolicy,
        VaultRetentionSettings,
    };
    use crate::operations::OperationalContext;
    use crate::workspace::{
        IncidentEvidenceLinkRequest, IncidentReopenRequest, IncidentResolutionRequest,
        ManualRunbookRequest, ManualRunbookReviewRequest, ManualRunbookRevisionRestoreRequest,
        ManualRunbookUpdateRequest, RunbookSearchRequest, WorkspaceContextUpdate,
        WorkspaceCreateRequest, WorkspaceDocumentImportRequest, WorkspaceSessionStartRequest,
    };
    use std::fs;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_memory(content: &str) -> NewClipboardItem {
        NewClipboardItem {
            content: content.to_string(),
            content_type: "Text".to_string(),
            source_application: "Test".to_string(),
            content_hash: format!("retention-{content}"),
            character_count: content.len() as i64,
            word_count: 1,
            file_size: None,
            image_width: None,
            image_height: None,
            language: None,
            ai_summary: content.to_string(),
            category: "General".to_string(),
            keywords: Vec::new(),
            reading_time_minutes: 1,
            semantic_text: content.to_string(),
            embedding: Vec::new(),
            embedding_source: "Local".to_string(),
            operational_context: OperationalContext::default(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn transient_operations_retry_until_success() {
        tauri::async_runtime::block_on(async {
            let attempts = Arc::new(AtomicUsize::new(0));
            let counter = Arc::clone(&attempts);
            let result = retry_transient(
                "test_operation",
                3,
                Duration::ZERO,
                move || {
                    let attempt = counter.fetch_add(1, Ordering::SeqCst) + 1;
                    async move {
                        if attempt < 3 {
                            Err("database is locked")
                        } else {
                            Ok("completed")
                        }
                    }
                },
                |error| *error == "database is locked",
            )
            .await;
            assert_eq!(result, Ok("completed"));
            assert_eq!(attempts.load(Ordering::SeqCst), 3);
        });
    }

    #[test]
    fn vault_retention_prunes_expired_memories_without_removing_favorites() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-vault-retention-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            let favorite_id = database
                .insert_item(test_memory("favorite-memory"))
                .await
                .expect("favorite memory should store")
                .expect("favorite memory id should exist");
            let expired_id = database
                .insert_item(test_memory("expired-memory"))
                .await
                .expect("expired memory should store")
                .expect("expired memory id should exist");
            let recent_id = database
                .insert_item(test_memory("recent-memory"))
                .await
                .expect("recent memory should store")
                .expect("recent memory id should exist");

            database
                .toggle_favorite(favorite_id)
                .await
                .expect("favorite memory should be protected");
            sqlx::query(
                "UPDATE clipboard_items SET created_at = datetime('now', '-10 days'), last_copied_at = datetime('now', '-10 days') WHERE id IN (?1, ?2);",
            )
            .bind(favorite_id)
            .bind(expired_id)
            .execute(&database.pool)
            .await
            .expect("test memories should be aged");

            database
                .update_vault_retention_settings(&VaultRetentionSettings {
                    retention_days: 1,
                    max_items: 10,
                    max_storage_mb: 64,
                    preserve_favorites: true,
                    updated_at: String::new(),
                })
                .await
                .expect("retention policy should save");
            let age_result = database
                .apply_vault_retention()
                .await
                .expect("retention should prune expired memories");
            assert_eq!(age_result.removed_items, 1);
            assert_eq!(age_result.protected_favorites, 1);
            assert!(database
                .item(expired_id)
                .await
                .expect("lookup should work")
                .is_none());
            assert!(database
                .item(favorite_id)
                .await
                .expect("lookup should work")
                .is_some());
            assert!(database
                .item(recent_id)
                .await
                .expect("lookup should work")
                .is_some());

            database
                .update_vault_retention_settings(&VaultRetentionSettings {
                    retention_days: 3650,
                    max_items: 1,
                    max_storage_mb: 64,
                    preserve_favorites: true,
                    updated_at: String::new(),
                })
                .await
                .expect("capacity policy should save");
            let capacity_result = database
                .apply_vault_retention()
                .await
                .expect("retention should enforce the item limit");
            assert_eq!(capacity_result.removed_items, 1);
            assert!(capacity_result.limits_met);
            assert!(database
                .item(favorite_id)
                .await
                .expect("lookup should work")
                .is_some());
            assert!(database
                .item(recent_id)
                .await
                .expect("lookup should work")
                .is_none());
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn permanent_failures_do_not_retry() {
        tauri::async_runtime::block_on(async {
            let attempts = Arc::new(AtomicUsize::new(0));
            let counter = Arc::clone(&attempts);
            let result: Result<(), &str> = retry_transient(
                "test_operation",
                3,
                Duration::ZERO,
                move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                    async { Err("constraint violation") }
                },
                |error| *error == "database is locked",
            )
            .await;
            assert_eq!(result, Err("constraint violation"));
            assert_eq!(attempts.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    fn handoff_export_audit_records_only_export_metadata() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-handoff-export-audit-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            let workspace = database
                .cognitive_workspace(None)
                .await
                .expect("default workspace should load");
            database
                .record_workspace_handoff_export(
                    workspace.id,
                    None,
                    "All workspace sessions",
                    "Network operations",
                    "Escalation review",
                    "Restricted",
                    Some(1_784_390_400),
                    "a1b2c3d4e5f60708",
                    "a3b1c9d7e5f4a2b8c6d0e1f3a5b7c9d2e4f6a8b0c1d3e5f7a9b2c4d6e8f0a1b3",
                    1_024,
                    3,
                    1,
                    1,
                    2,
                )
                .await
                .expect("handoff export audit should be recorded");
            let exports = database
                .workspace_handoff_exports(workspace.id)
                .await
                .expect("handoff export audit should load");

            assert_eq!(exports.len(), 1);
            assert_eq!(exports[0].scope, "All workspace sessions");
            assert_eq!(exports[0].recipient, "Network operations");
            assert_eq!(exports[0].purpose, "Escalation review");
            assert_eq!(exports[0].classification, "Restricted");
            assert_eq!(exports[0].expires_at_unix, Some(1_784_390_400));
            assert_eq!(exports[0].signer_fingerprint, "a1b2c3d4e5f60708");
            assert_eq!(exports[0].package_bytes, 1_024);
            assert_eq!(exports[0].event_count, 3);
            assert_eq!(exports[0].excluded_event_count, 1);
            assert_eq!(exports[0].incident_count, 1);
            assert_eq!(exports[0].resolution_count, 2);
            assert_eq!(exports[0].package_sha256.len(), 64);
            assert!(database
                .audit_logs()
                .await
                .expect("general audit should load")
                .iter()
                .any(|entry| entry.action == "workspace.handoff_exported"));
            for index in 0..205 {
                database
                    .record_workspace_handoff_export(
                        workspace.id,
                        None,
                        "All workspace sessions",
                        "Network operations",
                        "Retention review",
                        "Internal",
                        None,
                        "a1b2c3d4e5f60708",
                        &format!("{index:064x}"),
                        256,
                        1,
                        0,
                        0,
                        0,
                    )
                    .await
                    .expect("handoff export retention record should store");
            }
            let retained_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM workspace_handoff_exports;")
                    .fetch_one(&database.pool)
                    .await
                    .expect("handoff export count should load");
            let oldest_id: i64 =
                sqlx::query_scalar("SELECT MIN(id) FROM workspace_handoff_exports;")
                    .fetch_one(&database.pool)
                    .await
                    .expect("oldest retained handoff export should load");
            assert_eq!(retained_count, 200);
            assert!(oldest_id > 1);
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn handoff_recipient_trust_registry_updates_usage_metadata() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-handoff-recipient-trust-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            database
                .trust_handoff_recipient(
                    "Platform Operations",
                    "Restricted",
                    "Approved for incident escalations.",
                )
                .await
                .expect("recipient should be trusted");
            let updated = database
                .trust_handoff_recipient("platform operations", "Confidential", "Expanded scope.")
                .await
                .expect("recipient trust should update");
            assert_eq!(updated.recipient, "Platform Operations");
            assert_eq!(updated.max_classification, "Confidential");
            assert_eq!(updated.note, "Expanded scope.");

            let workspace = database
                .cognitive_workspace(None)
                .await
                .expect("default workspace should load");
            database
                .record_workspace_handoff_export(
                    workspace.id,
                    None,
                    "All workspace sessions",
                    "platform operations",
                    "Escalation review",
                    "Restricted",
                    None,
                    "a1b2c3d4e5f60708",
                    "a3b1c9d7e5f4a2b8c6d0e1f3a5b7c9d2e4f6a8b0c1d3e5f7a9b2c4d6e8f0a1b3",
                    1_024,
                    3,
                    0,
                    1,
                    2,
                )
                .await
                .expect("handoff export audit should be recorded");
            let records = database
                .handoff_recipient_trust_records()
                .await
                .expect("trusted recipients should load");
            assert_eq!(records.len(), 1);
            assert!(records[0].is_active);
            assert_eq!(records[0].export_count, 1);
            assert!(records[0].last_used_at.is_some());
            database
                .revoke_handoff_recipient("PLATFORM OPERATIONS")
                .await
                .expect("recipient should revoke");
            assert!(database
                .trusted_handoff_recipient("platform operations")
                .await
                .expect("trust lookup should succeed")
                .is_none());
            let revoked_records = database
                .handoff_recipient_trust_records()
                .await
                .expect("trusted recipients should load");
            assert!(!revoked_records[0].is_active);
            assert!(revoked_records[0].revoked_at.is_some());
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn handoff_signer_trust_registry_updates_usage_metadata() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-handoff-signer-trust-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            let trusted = database
                .trust_handoff_signer("a1b2c3d4e5f60708", "Remote operations")
                .await
                .expect("signer should be trusted");
            assert_eq!(trusted.signer_fingerprint, "a1b2c3d4e5f60708");
            assert_eq!(trusted.label, "Remote operations");
            database
                .mark_handoff_signer_used("a1b2c3d4e5f60708")
                .await
                .expect("signer usage should update");
            let records = database
                .handoff_signer_trust_records()
                .await
                .expect("trusted signers should load");
            assert_eq!(records.len(), 1);
            assert!(records[0].is_active);
            assert_eq!(records[0].import_count, 1);
            assert!(records[0].last_used_at.is_some());
            database
                .revoke_handoff_signer("a1b2c3d4e5f60708")
                .await
                .expect("signer should revoke");
            assert!(database
                .trusted_handoff_signer("a1b2c3d4e5f60708")
                .await
                .expect("signer trust lookup should succeed")
                .is_none());
            let revoked_records = database
                .handoff_signer_trust_records()
                .await
                .expect("trusted signers should load");
            assert!(!revoked_records[0].is_active);
            assert!(revoked_records[0].revoked_at.is_some());
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn team_sharing_audit_logs_focus_on_policy_and_device_events() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-team-sharing-audit-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            database
                .register_team_sharing_device(&TeamSharingDeviceRequest {
                    device_name: "RHEL admin laptop".to_string(),
                    platform: "RHEL 9".to_string(),
                    sync_mode: "Local-only".to_string(),
                })
                .await
                .expect("device should register");
            let device = database
                .sync_devices()
                .await
                .expect("devices should load")
                .into_iter()
                .find(|device| device.device_name == "RHEL admin laptop")
                .expect("registered device should exist");
            database
                .approve_team_sharing_device(&TeamSharingDeviceStatusRequest {
                    device_id: device.id,
                })
                .await
                .expect("device should approve");
            database
                .record_team_sharing_filtered_manifest_ledger_export(
                    "/tmp/manifest-warnings.md",
                    "Warnings",
                    2,
                )
                .await
                .expect("filtered manifest ledger audit should record");

            let logs = database
                .team_sharing_audit_logs()
                .await
                .expect("team sharing audit should load");
            assert!(logs
                .iter()
                .any(|entry| entry.action == "team_sharing.device.registered"));
            assert!(logs
                .iter()
                .any(|entry| entry.action == "team_sharing.device.approved"));
            assert!(logs.iter().any(|entry| {
                entry.action == "team_sharing.manifest_ledger.exported_filtered"
                    && entry.resource.contains("filter=Warnings")
                    && entry.resource.contains("matching_events=2")
            }));
            assert!(logs
                .iter()
                .all(|entry| entry.action.starts_with("team_sharing.")));
            for index in 0..25 {
                database
                    .record_team_sharing_manifest_export(&format!("/tmp/manifest-{index}.json"))
                    .await
                    .expect("manifest audit should record");
            }
            let manifest_logs = database
                .team_sharing_manifest_ledger_audit_logs()
                .await
                .expect("manifest ledger audit should load");
            assert_eq!(manifest_logs.len(), 26);
            assert!(manifest_logs
                .iter()
                .all(|entry| entry.action.contains("manifest")));
            for index in 0..205 {
                database
                    .record_team_sharing_report_export(&format!("/tmp/team-sharing-{index}.md"))
                    .await
                    .expect("sharing report audit should record");
            }
            let retained_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM audit_logs WHERE action GLOB 'team_sharing.*';",
            )
            .fetch_one(&database.pool)
            .await
            .expect("retained team sharing audit count should load");
            let oldest_resource: String = sqlx::query_scalar(
                "SELECT resource FROM audit_logs WHERE action GLOB 'team_sharing.*' ORDER BY id ASC LIMIT 1;",
            )
            .fetch_one(&database.pool)
            .await
            .expect("oldest retained team sharing audit should load");

            assert_eq!(retained_count, 200);
            assert!(oldest_resource.contains("team-sharing-5.md"));
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn team_sharing_sync_dry_run_estimates_local_manifest() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-team-sharing-dry-run-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            database
                .update_team_sharing_policy(&TeamSharingPolicy {
                    enabled: true,
                    ..TeamSharingPolicy::default()
                })
                .await
                .expect("policy should save");
            database
                .register_team_sharing_device(&TeamSharingDeviceRequest {
                    device_name: "RHEL admin laptop".to_string(),
                    platform: "RHEL 9".to_string(),
                    sync_mode: "Local-only".to_string(),
                })
                .await
                .expect("device should register");
            let device = database
                .sync_devices()
                .await
                .expect("devices should load")
                .into_iter()
                .find(|device| device.device_name == "RHEL admin laptop")
                .expect("registered device should exist");
            database
                .approve_team_sharing_device(&TeamSharingDeviceStatusRequest {
                    device_id: device.id,
                })
                .await
                .expect("device should approve");
            database
                .trust_handoff_recipient("Platform operations", "Internal", "Dry run")
                .await
                .expect("recipient should trust");
            let workspace = database
                .cognitive_workspace(None)
                .await
                .expect("workspace should load");
            database
                .record_workspace_handoff_export(
                    workspace.id,
                    None,
                    "All workspace sessions",
                    "Platform operations",
                    "Dry run",
                    "Internal",
                    Some(1_784_390_400),
                    "a1b2c3d4e5f60708",
                    "a3b1c9d7e5f4a2b8c6d0e1f3a5b7c9d2e4f6a8b0c1d3e5f7a9b2c4d6e8f0a1b3",
                    2_048,
                    2,
                    0,
                    0,
                    0,
                )
                .await
                .expect("handoff export audit should store");

            let dry_run = database
                .team_sharing_sync_dry_run()
                .await
                .expect("dry run should generate");

            assert!(dry_run.ready);
            assert_eq!(dry_run.eligible_devices, 1);
            assert!(dry_run.estimated_records >= 1);
            assert!(dry_run.estimated_bytes >= 2_048);
            assert!(dry_run
                .eligible_scopes
                .contains(&"Workspace handoffs".to_string()));
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn handoff_inspection_ledger_stores_metadata_only() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-handoff-inspection-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            database
                .record_workspace_handoff_inspection(
                    "Verified",
                    Some("Remote Nginx"),
                    Some("Restricted"),
                    Some("a1b2c3d4e5f60708"),
                    "a3b1c9d7e5f4a2b8c6d0e1f3a5b7c9d2e4f6a8b0c1d3e5f7a9b2c4d6e8f0a1b3",
                    Some("b3b1c9d7e5f4a2b8c6d0e1f3a5b7c9d2e4f6a8b0c1d3e5f7a9b2c4d6e8f0a1b3"),
                    None,
                    2_048,
                )
                .await
                .expect("inspection audit should store");
            database
                .record_workspace_handoff_inspection(
                    "Rejected",
                    None,
                    None,
                    None,
                    "c3b1c9d7e5f4a2b8c6d0e1f3a5b7c9d2e4f6a8b0c1d3e5f7a9b2c4d6e8f0a1b3",
                    None,
                    Some("Invalid package"),
                    128,
                )
                .await
                .expect("rejected inspection audit should store");
            let inspections = database
                .workspace_handoff_inspection_records()
                .await
                .expect("inspection audit should load");
            assert_eq!(inspections.len(), 2);
            assert_eq!(inspections[0].status, "Rejected");
            assert_eq!(
                inspections[0].failure_reason,
                Some("Invalid package".to_string())
            );
            assert_eq!(
                inspections[1].workspace_name,
                Some("Remote Nginx".to_string())
            );
            assert_eq!(inspections[1].package_bytes, 2_048);
            for index in 0..205 {
                database
                    .record_workspace_handoff_inspection(
                        "Rejected",
                        None,
                        None,
                        None,
                        &format!("{index:064x}"),
                        None,
                        Some("Retention test"),
                        64,
                    )
                    .await
                    .expect("inspection retention record should store");
            }
            let retained_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM workspace_handoff_inspections;")
                    .fetch_one(&database.pool)
                    .await
                    .expect("inspection count should load");
            let oldest_id: i64 =
                sqlx::query_scalar("SELECT MIN(id) FROM workspace_handoff_inspections;")
                    .fetch_one(&database.pool)
                    .await
                    .expect("oldest retained inspection should load");
            assert_eq!(retained_count, 200);
            assert!(oldest_id > 2);
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn insight_trail_persists_error_events_and_incidents() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-insight-trail-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            let inserted = database
                .insert_item(NewClipboardItem {
                    content: "nginx: permission denied by SELinux".to_string(),
                    content_type: "Text".to_string(),
                    source_application: "Terminal History (Bash)".to_string(),
                    content_hash: format!("insight-trail-test-{nonce}"),
                    character_count: 34,
                    word_count: 5,
                    file_size: None,
                    image_width: None,
                    image_height: None,
                    language: Some("Bash".to_string()),
                    ai_summary: "Nginx access was denied by the local SELinux policy.".to_string(),
                    category: "Operations".to_string(),
                    keywords: vec!["Nginx".to_string(), "SELinux".to_string()],
                    reading_time_minutes: 1,
                    semantic_text: "nginx permission denied selinux".to_string(),
                    embedding: Vec::new(),
                    embedding_source: "Local".to_string(),
                    operational_context: OperationalContext::default(),
                    tags: vec!["Nginx".to_string(), "Linux".to_string()],
                })
                .await
                .expect("memory should be stored");

            assert!(inserted.is_some());
            let events = database
                .insight_trail_events(&InsightTrailSearchRequest {
                    query: String::new(),
                    event_type: "Error".to_string(),
                    limit: 20,
                })
                .await
                .expect("timeline events should load");
            let incidents = database
                .insight_incidents()
                .await
                .expect("incidents should load");

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_type, "Error");
            assert!(events[0].incident_id.is_some());
            assert_eq!(incidents.len(), 1);
            assert_eq!(incidents[0].status, "Open");
            assert_eq!(incidents[0].recommended_steps.len(), 3);

            let initial_workspace = database
                .cognitive_workspace(None)
                .await
                .expect("workspace context should load");
            assert_eq!(initial_workspace.event_count, 1);
            assert_eq!(initial_workspace.memory_count, 1);
            assert_eq!(initial_workspace.error_count, 1);
            assert!(initial_workspace.sources.contains(&"Error".to_string()));
            let initial_snapshot = database
                .workspace_snapshot(Some(initial_workspace.id))
                .await
                .expect("workspace snapshot should load");
            assert_eq!(initial_snapshot.incidents.len(), 1);
            assert_eq!(initial_snapshot.incidents[0].event_count, 1);
            let evidence_memory_id = database
                .insert_item(NewClipboardItem {
                    content: "systemctl restart nginx".to_string(),
                    content_type: "Code".to_string(),
                    source_application: "Terminal History (Bash)".to_string(),
                    content_hash: format!("insight-evidence-test-{nonce}"),
                    character_count: 23,
                    word_count: 3,
                    file_size: None,
                    image_width: None,
                    image_height: None,
                    language: Some("Bash".to_string()),
                    ai_summary: "Restarted the Nginx service after validation.".to_string(),
                    category: "Operations".to_string(),
                    keywords: vec!["Nginx".to_string()],
                    reading_time_minutes: 1,
                    semantic_text: "systemctl restart nginx".to_string(),
                    embedding: Vec::new(),
                    embedding_source: "Local".to_string(),
                    operational_context: OperationalContext::default(),
                    tags: vec!["Nginx".to_string(), "Linux".to_string()],
                })
                .await
                .expect("evidence memory should be stored")
                .expect("evidence memory should have an id");
            let evidence_event_id = database
                .workspace_snapshot(Some(initial_workspace.id))
                .await
                .expect("workspace snapshot should include evidence")
                .events
                .into_iter()
                .find(|event| event.memory_id == Some(evidence_memory_id))
                .expect("terminal evidence should be linked to the active workspace")
                .id;
            let evidence_snapshot = database
                .link_workspace_event_to_incident(&IncidentEvidenceLinkRequest {
                    workspace_id: initial_workspace.id,
                    incident_id: initial_snapshot.incidents[0].id,
                    event_id: evidence_event_id,
                })
                .await
                .expect("saved terminal event should link to the incident");
            assert!(evidence_snapshot.events.iter().any(|event| {
                event.id == evidence_event_id
                    && event.memory_id == Some(evidence_memory_id)
                    && event.incident_id == Some(initial_snapshot.incidents[0].id)
            }));
            assert_eq!(evidence_snapshot.incidents[0].event_count, 2);
            let resolved_snapshot = database
                .record_incident_resolution(&IncidentResolutionRequest {
                    workspace_id: initial_workspace.id,
                    incident_id: initial_snapshot.incidents[0].id,
                    title: "restorecon -Rv /var/www".to_string(),
                    details: "Restored the SELinux file context and verified the Nginx response."
                        .to_string(),
                })
                .await
                .expect("incident resolution should be recorded");
            assert_eq!(resolved_snapshot.incidents[0].status, "Resolved");
            assert_eq!(resolved_snapshot.resolutions.len(), 1);
            assert_eq!(
                resolved_snapshot.resolutions[0].title,
                "restorecon -Rv /var/www"
            );
            let reopened_snapshot = database
                .reopen_incident(&IncidentReopenRequest {
                    workspace_id: initial_workspace.id,
                    incident_id: initial_snapshot.incidents[0].id,
                    reason:
                        "The deployment still returns a permission error after the next release."
                            .to_string(),
                })
                .await
                .expect("incident should reopen with a follow-up event");
            assert_eq!(reopened_snapshot.incidents[0].status, "Open");
            assert_eq!(reopened_snapshot.events.len(), 3);
            assert_eq!(reopened_snapshot.events[0].event_type, "Note");
            assert!(reopened_snapshot.events[0]
                .title
                .starts_with("Incident reopened:"));
            let runbook_entries = database
                .runbook_entries(&RunbookSearchRequest {
                    query: "restorecon".to_string(),
                    review_status: "All".to_string(),
                })
                .await
                .expect("runbook search should load the recorded resolution");
            assert_eq!(runbook_entries.len(), 1);
            assert_eq!(runbook_entries[0].title, "restorecon -Rv /var/www");

            let follow_up_workspace = database
                .create_cognitive_workspace(&WorkspaceCreateRequest {
                    name: "Secondary Nginx host".to_string(),
                    project: "RHEL operations".to_string(),
                })
                .await
                .expect("follow-up workspace should be created");
            database
                .start_workspace_session(&WorkspaceSessionStartRequest {
                    workspace_id: follow_up_workspace.workspace.id,
                    title: "Validate recurring SELinux failure".to_string(),
                })
                .await
                .expect("follow-up session should start");
            database
                .insert_item(NewClipboardItem {
                    content: "nginx: permission denied by SELinux".to_string(),
                    content_type: "Text".to_string(),
                    source_application: "Terminal History (Bash)".to_string(),
                    content_hash: format!("known-remedy-follow-up-{nonce}"),
                    character_count: 34,
                    word_count: 5,
                    file_size: None,
                    image_width: None,
                    image_height: None,
                    language: Some("Bash".to_string()),
                    ai_summary: "Nginx access was denied by the local SELinux policy.".to_string(),
                    category: "Operations".to_string(),
                    keywords: vec!["Nginx".to_string(), "SELinux".to_string()],
                    reading_time_minutes: 1,
                    semantic_text: "nginx permission denied selinux".to_string(),
                    embedding: Vec::new(),
                    embedding_source: "Local".to_string(),
                    operational_context: OperationalContext::default(),
                    tags: vec!["Nginx".to_string(), "Linux".to_string()],
                })
                .await
                .expect("follow-up error should be stored");
            let known_remedy_snapshot = database
                .workspace_snapshot(Some(follow_up_workspace.workspace.id))
                .await
                .expect("follow-up workspace should load");
            assert_eq!(known_remedy_snapshot.incidents.len(), 1);
            assert_eq!(known_remedy_snapshot.resolutions.len(), 1);
            assert_eq!(
                known_remedy_snapshot.resolutions[0].workspace_name,
                "Local operations"
            );

            database
                .update_cognitive_workspace(
                    1,
                    &WorkspaceContextUpdate {
                        name: "Nginx rollout".to_string(),
                        project: "RHEL operations".to_string(),
                    },
                )
                .await
                .expect("workspace context should update");
            let updated_workspace = database
                .cognitive_workspace(Some(1))
                .await
                .expect("updated workspace context should load");
            assert_eq!(updated_workspace.name, "Nginx rollout");
            assert_eq!(updated_workspace.project, "RHEL operations");
            assert!(updated_workspace.summary.contains("error signal"));
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn workspace_sessions_scope_new_events_to_the_active_project() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-workspace-sessions-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            let created = database
                .create_cognitive_workspace(&WorkspaceCreateRequest {
                    name: "Nginx rollout".to_string(),
                    project: "RHEL operations".to_string(),
                })
                .await
                .expect("workspace should be created");
            let workspace_id = created.workspace.id;

            database
                .start_workspace_session(&WorkspaceSessionStartRequest {
                    workspace_id,
                    title: "Investigate deployment failure".to_string(),
                })
                .await
                .expect("session should start");
            database
                .record_insight_trail_note(
                    "Nginx remediation",
                    "Restarted nginx after reviewing the configuration.",
                    &["Nginx".to_string(), "RHEL".to_string()],
                )
                .await
                .expect("manual note should be recorded");

            let project_snapshot = database
                .workspace_snapshot(Some(workspace_id))
                .await
                .expect("project timeline should load");
            let default_snapshot = database
                .workspace_snapshot(Some(1))
                .await
                .expect("default timeline should load");

            assert_eq!(project_snapshot.events.len(), 1);
            assert_eq!(project_snapshot.events[0].title, "Nginx remediation");
            let active_session = project_snapshot
                .active_session
                .expect("session should be active");
            assert_eq!(
                project_snapshot.events[0].session_id,
                Some(active_session.id)
            );
            assert_eq!(active_session.event_count, 1);
            assert!(default_snapshot.events.is_empty());

            let archived_snapshot = database
                .archive_cognitive_workspace(workspace_id)
                .await
                .expect("workspace should archive");
            assert_eq!(archived_snapshot.workspace.status, "Archived");
            assert!(archived_snapshot.active_session.is_none());
            assert!(database
                .start_workspace_session(&WorkspaceSessionStartRequest {
                    workspace_id,
                    title: "Should not start".to_string(),
                })
                .await
                .is_err());

            let restored_snapshot = database
                .restore_cognitive_workspace(workspace_id)
                .await
                .expect("workspace should restore");
            assert_eq!(restored_snapshot.workspace.status, "Ready");
            assert!(restored_snapshot.active_session.is_none());
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn workspace_document_import_stays_in_its_active_session() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-document-import-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            let workspace = database
                .create_cognitive_workspace(&WorkspaceCreateRequest {
                    name: "Nginx deployment".to_string(),
                    project: "RHEL operations".to_string(),
                })
                .await
                .expect("workspace should be created");
            let workspace_id = workspace.workspace.id;
            database
                .start_workspace_session(&WorkspaceSessionStartRequest {
                    workspace_id,
                    title: "Validate nginx rollout".to_string(),
                })
                .await
                .expect("session should start");

            let request = WorkspaceDocumentImportRequest {
                workspace_id,
                file_name: "restart-nginx.sh".to_string(),
                content: "systemctl restart nginx".to_string(),
            };
            let imported = database
                .import_workspace_document(&request)
                .await
                .expect("document should import");
            assert!(imported.stored);
            assert_eq!(imported.snapshot.events.len(), 1);
            assert_eq!(
                imported.snapshot.events[0].source_application,
                "Local file import: restart-nginx.sh"
            );
            assert_eq!(
                imported.snapshot.events[0].session_id,
                imported.snapshot.active_session.map(|session| session.id)
            );

            let duplicate = database
                .import_workspace_document(&request)
                .await
                .expect("duplicate import should be handled");
            assert!(!duplicate.stored);
            assert_eq!(duplicate.snapshot.events.len(), 1);

            let blocked = database
                .import_workspace_document(&WorkspaceDocumentImportRequest {
                    workspace_id,
                    file_name: "service.env".to_string(),
                    content: "api_key=very-long-private-value-12345".to_string(),
                })
                .await
                .expect("sensitive import should be handled");
            assert!(!blocked.stored);
            assert_eq!(blocked.snapshot.events.len(), 1);
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn manual_runbooks_are_searchable_without_an_incident() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-manual-runbook-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            let created = database
                .create_manual_runbook(&ManualRunbookRequest {
                    title: "Restart PostgreSQL safely".to_string(),
                    details: "Verify replicas, restart PostgreSQL, then confirm client health."
                        .to_string(),
                    tags: vec!["PostgreSQL".to_string(), "Maintenance".to_string()],
                })
                .await
                .expect("manual runbook should be created");
            assert!(created.incident_id.is_none());
            assert_eq!(created.tags, vec!["Maintenance", "PostgreSQL"]);
            assert_eq!(created.latest_revision, 1);
            assert!(created.last_reviewed_revision.is_none());

            database
                .insert_audit_log("system", "workspace.created", "unrelated", "Info")
                .await
                .expect("unrelated audit event should be recorded");
            let runbook_audit = database
                .runbook_audit_logs()
                .await
                .expect("runbook audit trail should load");
            assert!(runbook_audit
                .iter()
                .any(|entry| entry.action == "runbook.created"));
            assert!(runbook_audit
                .iter()
                .all(|entry| entry.action.starts_with("runbook.")));

            let reviewed = database
                .review_manual_runbook(&ManualRunbookReviewRequest {
                    entry_id: created.id,
                    note: "Validated against replica health checks.".to_string(),
                })
                .await
                .expect("manual runbook should be marked reviewed");
            assert_eq!(reviewed.last_reviewed_revision, Some(1));
            assert!(reviewed.last_reviewed_at.is_some());
            assert_eq!(
                reviewed.last_review_note.as_deref(),
                Some("Validated against replica health checks.")
            );

            sqlx::query(
                "UPDATE manual_runbooks SET last_reviewed_at = datetime('now', '-91 days') WHERE id = ?1;",
            )
            .bind(created.id.checked_neg().expect("manual runbook id should be negative"))
            .execute(&database.pool)
            .await
            .expect("review timestamp should be aged for cadence testing");
            let due = database
                .runbook_entries(&RunbookSearchRequest {
                    query: "postgresql".to_string(),
                    review_status: "Review due".to_string(),
                })
                .await
                .expect("review cadence should load overdue runbooks");
            assert_eq!(due.len(), 1);
            assert_eq!(due[0].review_status, "Review due");
            database
                .review_manual_runbook(&ManualRunbookReviewRequest {
                    entry_id: created.id,
                    note: String::new(),
                })
                .await
                .expect("overdue runbook should be reviewable again");

            let results = database
                .runbook_entries(&RunbookSearchRequest {
                    query: "postgresql".to_string(),
                    review_status: "All".to_string(),
                })
                .await
                .expect("manual runbook should be searchable");
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].title, "Restart PostgreSQL safely");
            assert_eq!(results[0].incident_title, "Manual runbook");

            let updated = database
                .update_manual_runbook(&ManualRunbookUpdateRequest {
                    id: created.id,
                    title: "Restart PostgreSQL with replica checks".to_string(),
                    details: "Verify replicas, restart PostgreSQL, and confirm client health."
                        .to_string(),
                    tags: vec!["PostgreSQL".to_string(), "Replication".to_string()],
                })
                .await
                .expect("manual runbook should update");
            assert_eq!(updated.title, "Restart PostgreSQL with replica checks");
            assert!(updated.tags.contains(&"Replication".to_string()));
            assert_eq!(updated.latest_revision, 2);
            assert_eq!(updated.last_reviewed_revision, Some(1));

            let needs_review = database
                .runbook_entries(&RunbookSearchRequest {
                    query: "postgresql".to_string(),
                    review_status: "Needs review".to_string(),
                })
                .await
                .expect("review queue should load outdated runbooks");
            assert_eq!(needs_review.len(), 1);
            assert_eq!(needs_review[0].id, updated.id);

            let revisions = database
                .manual_runbook_revisions(updated.id)
                .await
                .expect("manual runbook revision history should load");
            assert_eq!(revisions.len(), 2);
            assert_eq!(revisions[0].revision, 2);
            assert_eq!(revisions[0].title, "Restart PostgreSQL with replica checks");
            assert_eq!(revisions[1].revision, 1);
            assert_eq!(revisions[1].title, "Restart PostgreSQL safely");

            let restored = database
                .restore_manual_runbook_revision(&ManualRunbookRevisionRestoreRequest {
                    entry_id: updated.id,
                    revision_id: revisions[1].id,
                })
                .await
                .expect("manual runbook revision should restore as a new version");
            assert_eq!(restored.title, "Restart PostgreSQL safely");
            let restored_revisions = database
                .manual_runbook_revisions(restored.id)
                .await
                .expect("restored revision history should load");
            assert_eq!(restored_revisions.len(), 3);
            assert_eq!(restored_revisions[0].revision, 3);
            assert_eq!(restored_revisions[0].title, "Restart PostgreSQL safely");

            database
                .delete_manual_runbook(restored.id)
                .await
                .expect("manual runbook should delete");
            let after_delete = database
                .runbook_entries(&RunbookSearchRequest {
                    query: "replication".to_string(),
                    review_status: "All".to_string(),
                })
                .await
                .expect("runbook search should complete after deletion");
            assert!(after_delete.is_empty());
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn handoff_import_creates_an_isolated_workspace_without_raw_memory() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-handoff-import-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            let package = serde_json::json!({
                "payload": {
                    "scope": { "session_title": "Nginx recovery" },
                    "handoff_intent": {
                        "recipient": "Platform operations",
                        "purpose": "Incident escalation",
                        "classification": "Restricted",
                        "expires_in_days": 7
                    },
                    "workspace": { "name": "Remote Nginx", "project": "RHEL operations" },
                    "events": [{
                        "event_type": "Error",
                        "title": "SELinux denied access",
                        "details": "Nginx could not read the deployment path.",
                        "source_application": "Terminal",
                        "severity": "Warning",
                        "incident_id": 7,
                        "tags": ["Nginx", "SELinux"]
                    }],
                    "incidents": [{
                        "id": 7,
                        "title": "Nginx SELinux incident",
                        "status": "Resolved",
                        "summary": "Access was denied by the local policy.",
                        "recommended_steps": ["Review the SELinux context."]
                    }],
                    "resolutions": [{
                        "incident_id": 7,
                        "title": "restorecon -Rv /var/www",
                        "details": "Restored the file context."
                    }]
                },
                "generated_locally_at": "1780000000",
                "integrity": { "payload_sha256": "handoff-test-checksum" },
                "authenticity": { "signer_fingerprint": "a1b2c3d4e5f60708" }
            });
            let imported = database
                .import_workspace_handoff(&package)
                .await
                .expect("handoff should import");
            assert!(imported.workspace.name.starts_with("Handoff: Remote Nginx"));
            assert!(imported.workspace.is_imported);
            assert!(imported.active_session.is_none());
            assert_eq!(imported.events.len(), 1);
            assert_eq!(imported.incidents.len(), 1);
            assert_eq!(imported.resolutions.len(), 1);
            assert_eq!(
                imported
                    .import_provenance
                    .as_ref()
                    .expect("import provenance should be retained")
                    .source_workspace,
                "Remote Nginx"
            );
            assert_eq!(
                imported
                    .import_provenance
                    .as_ref()
                    .expect("import provenance should be retained")
                    .source_classification,
                "Restricted"
            );
            assert_eq!(
                imported
                    .import_provenance
                    .as_ref()
                    .expect("import provenance should be retained")
                    .source_expires_at_unix,
                Some(1_780_604_800)
            );
            assert_eq!(
                imported
                    .import_provenance
                    .as_ref()
                    .expect("import provenance should be retained")
                    .source_signer_fingerprint,
                Some("a1b2c3d4e5f60708".to_string())
            );
            assert!(imported.events[0]
                .source_application
                .starts_with("Handoff: Remote Nginx"));
            assert_eq!(imported.workspace.memory_count, 0);
            assert!(database
                .start_workspace_session(&WorkspaceSessionStartRequest {
                    workspace_id: imported.workspace.id,
                    title: "Should stay read-only".to_string(),
                })
                .await
                .is_err());

            let repeated = database
                .import_workspace_handoff(&package)
                .await
                .expect("same handoff should reuse its isolated workspace");
            assert_eq!(repeated.workspace.id, imported.workspace.id);
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn reliability_check_creates_a_verified_local_backup() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-reliability-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            let status = database
                .database_reliability()
                .await
                .expect("reliability status should load");
            assert_eq!(status.integrity_status, "Healthy");
            assert_eq!(status.foreign_key_issues, 0);
            assert_eq!(status.journal_mode, "WAL");
            assert!(status.migration_count >= 3);

            let backup = database
                .create_verified_backup()
                .await
                .expect("verified backup should be created");
            assert!(backup.verified);
            assert!(backup.backup_count >= 1);
            assert!(std::path::Path::new(&backup.path).is_file());
            let reverified = database
                .verify_latest_backup()
                .await
                .expect("latest backup should re-verify");
            assert!(reverified.verified);
            assert_eq!(reverified.path, backup.path);
            let snapshots = database
                .recent_backup_snapshots()
                .expect("recent backup snapshots should load");
            assert!(snapshots
                .iter()
                .any(|snapshot| snapshot.path == backup.path));
            assert!(snapshots.iter().all(|snapshot| snapshot.bytes > 0));
            let selected = database
                .verify_backup_snapshot(&snapshots[0].file_name)
                .await
                .expect("selected backup should re-verify");
            assert_eq!(selected.path, backup.path);
            assert!(database
                .verify_backup_snapshot("../cymos.db")
                .await
                .is_err());

            let updated_status = database
                .database_reliability()
                .await
                .expect("updated reliability status should load");
            assert!(updated_status.backup_count >= 1);
            assert!(updated_status.last_backup.is_some());
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn reliability_report_inventory_is_filtered_and_newest_first() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-reliability-reports-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            let reports_dir = database.exports_dir();
            fs::create_dir_all(&reports_dir).expect("reports directory should be created");
            fs::write(reports_dir.join("vault-reliability-100.md"), "older report")
                .expect("older report should be created");
            fs::write(reports_dir.join("vault-reliability-200.md"), "newer report")
                .expect("newer report should be created");
            fs::write(reports_dir.join("workspace-report.md"), "unrelated report")
                .expect("unrelated report should be created");

            let reports = database
                .recent_database_reliability_reports()
                .expect("reliability report inventory should load");

            assert_eq!(reports.len(), 2);
            assert_eq!(reports[0].file_name, "vault-reliability-200.md");
            assert_eq!(reports[1].file_name, "vault-reliability-100.md");
            assert!(reports.iter().all(|report| report.bytes > 0));
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }

    #[test]
    fn privacy_guard_blocks_sensitive_content_before_vault_storage() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cymos-privacy-{nonce}"));
        fs::create_dir_all(&data_dir).expect("test data directory should be created");
        fs::File::create(data_dir.join("cymos.db")).expect("empty test database should be created");

        tauri::async_runtime::block_on(async {
            let database = Database::connect(data_dir.clone())
                .await
                .expect("database should migrate");
            let inserted = database
                .insert_item(NewClipboardItem {
                    content: "api_key=very-long-private-value-12345".to_string(),
                    content_type: "Text".to_string(),
                    source_application: "Clipboard".to_string(),
                    content_hash: format!("privacy-test-{nonce}"),
                    character_count: 35,
                    word_count: 1,
                    file_size: None,
                    image_width: None,
                    image_height: None,
                    language: None,
                    ai_summary: "Potential credential".to_string(),
                    category: "General".to_string(),
                    keywords: Vec::new(),
                    reading_time_minutes: 1,
                    semantic_text: "credential".to_string(),
                    embedding: Vec::new(),
                    embedding_source: "Local".to_string(),
                    operational_context: OperationalContext::default(),
                    tags: Vec::new(),
                })
                .await
                .expect("privacy decision should complete");
            assert!(inserted.is_none());

            let status = database
                .privacy_status()
                .await
                .expect("privacy status should load");
            assert_eq!(status.blocked_capture_count, 1);
            assert!(database
                .search_items(&super::SearchRequest {
                    query: String::new(),
                    content_type: "All".to_string(),
                    favorite_only: false,
                    collection_id: None,
                    tag: "All".to_string(),
                    category: "All".to_string(),
                    semantic: false,
                })
                .await
                .expect("memory search should load")
                .is_empty());
        });

        fs::remove_dir_all(&data_dir).expect("test data directory should be removed");
    }
}
