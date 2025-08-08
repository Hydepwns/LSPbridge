//! Repository registry for tracking related repositories

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Information about a registered repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryInfo {
    /// Unique identifier for the repository
    pub id: String,

    /// Display name for the repository
    pub name: String,

    /// Local path to the repository
    pub path: PathBuf,

    /// Remote URL (if available)
    pub remote_url: Option<String>,

    /// Primary language of the repository
    pub primary_language: Option<String>,

    /// Build system detected
    pub build_system: Option<String>,

    /// Whether this is part of a monorepo
    pub is_monorepo_member: bool,

    /// Parent monorepo ID (if applicable)
    pub monorepo_id: Option<String>,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Whether the repository is active
    pub active: bool,

    /// Last time diagnostics were collected
    pub last_diagnostic_run: Option<DateTime<Utc>>,

    /// Repository metadata
    pub metadata: serde_json::Value,
}

/// Relationship between repositories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryRelation {
    /// Source repository ID
    pub source_id: String,

    /// Target repository ID
    pub target_id: String,

    /// Type of relationship
    pub relation_type: RelationType,

    /// Additional relationship data
    pub data: serde_json::Value,
}

/// Types of relationships between repositories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationType {
    /// Shared type definitions
    SharedTypes,

    /// Direct dependency
    Dependency,

    /// Dev dependency
    DevDependency,

    /// Monorepo sibling
    MonorepoSibling,

    /// API provider/consumer
    ApiRelation,

    /// Custom relation
    Custom(String),
}

/// Repository registry for managing multiple repositories
pub struct RepositoryRegistry {
    conn: Arc<Mutex<Connection>>,
}

impl RepositoryRegistry {
    /// Load existing registry or create a new one
    pub async fn load_or_create(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create registry directory")?;
        }

        let conn = Connection::open(path).context("Failed to open registry database")?;

        // Initialize schema
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS repositories (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                remote_url TEXT,
                primary_language TEXT,
                build_system TEXT,
                is_monorepo_member BOOLEAN DEFAULT 0,
                monorepo_id TEXT,
                tags TEXT,
                active BOOLEAN DEFAULT 1,
                last_diagnostic_run INTEGER,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS repository_relations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                data TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL,
                FOREIGN KEY (source_id) REFERENCES repositories(id),
                FOREIGN KEY (target_id) REFERENCES repositories(id),
                UNIQUE(source_id, target_id, relation_type)
            );
            
            CREATE INDEX IF NOT EXISTS idx_repos_active ON repositories(active);
            CREATE INDEX IF NOT EXISTS idx_repos_monorepo ON repositories(monorepo_id);
            CREATE INDEX IF NOT EXISTS idx_relations_source ON repository_relations(source_id);
            CREATE INDEX IF NOT EXISTS idx_relations_target ON repository_relations(target_id);
            "#,
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Register a new repository
    pub async fn register(&self, info: RepositoryInfo) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now().timestamp();

        conn.execute(
            r#"
            INSERT OR REPLACE INTO repositories 
            (id, name, path, remote_url, primary_language, build_system,
             is_monorepo_member, monorepo_id, tags, active, last_diagnostic_run,
             metadata, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            "#,
            params![
                info.id,
                info.name,
                info.path.to_string_lossy(),
                info.remote_url,
                info.primary_language,
                info.build_system,
                info.is_monorepo_member,
                info.monorepo_id,
                serde_json::to_string(&info.tags)?,
                info.active,
                info.last_diagnostic_run.map(|dt| dt.timestamp()),
                serde_json::to_string(&info.metadata)?,
                now,
                now,
            ],
        )?;

        Ok(())
    }

    /// Get repository by ID
    pub async fn get(&self, id: &str) -> Result<Option<RepositoryInfo>> {
        let conn = self.conn.lock().await;

        let result = conn
            .query_row(
                "SELECT * FROM repositories WHERE id = ?1",
                params![id],
                |row| {
                    Ok(RepositoryInfo {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        path: PathBuf::from(row.get::<_, String>(2)?),
                        remote_url: row.get(3)?,
                        primary_language: row.get(4)?,
                        build_system: row.get(5)?,
                        is_monorepo_member: row.get(6)?,
                        monorepo_id: row.get(7)?,
                        tags: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
                        active: row.get(9)?,
                        last_diagnostic_run: row
                            .get::<_, Option<i64>>(10)?
                            .map(|ts| DateTime::from_timestamp(ts, 0).unwrap()),
                        metadata: serde_json::from_str(&row.get::<_, String>(11)?)
                            .unwrap_or_default(),
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// List all active repositories
    pub async fn list_active(&self) -> Result<Vec<RepositoryInfo>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare("SELECT * FROM repositories WHERE active = 1 ORDER BY name")?;

        let repos = stmt
            .query_map([], |row| {
                Ok(RepositoryInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: PathBuf::from(row.get::<_, String>(2)?),
                    remote_url: row.get(3)?,
                    primary_language: row.get(4)?,
                    build_system: row.get(5)?,
                    is_monorepo_member: row.get(6)?,
                    monorepo_id: row.get(7)?,
                    tags: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
                    active: row.get(9)?,
                    last_diagnostic_run: row
                        .get::<_, Option<i64>>(10)?
                        .map(|ts| DateTime::from_timestamp(ts, 0).unwrap()),
                    metadata: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(repos)
    }

    /// List all repositories (including inactive)
    pub async fn list_all(&self) -> Result<Vec<RepositoryInfo>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare("SELECT * FROM repositories ORDER BY name")?;

        let repos = stmt
            .query_map([], |row| {
                Ok(RepositoryInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: PathBuf::from(row.get::<_, String>(2)?),
                    remote_url: row.get(3)?,
                    primary_language: row.get(4)?,
                    build_system: row.get(5)?,
                    is_monorepo_member: row.get(6)?,
                    monorepo_id: row.get(7)?,
                    tags: row
                        .get::<_, Option<String>>(8)?
                        .map(|s| s.split(',').map(String::from).collect())
                        .unwrap_or_default(),
                    active: row.get(9)?,
                    last_diagnostic_run: row.get(10)?,
                    metadata: serde_json::from_str(&row.get::<_, String>(11)?).ok().unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(repos)
    }

    /// Add a relationship between repositories
    pub async fn add_relation(&self, relation: RepositoryRelation) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now().timestamp();

        let relation_type = match &relation.relation_type {
            RelationType::SharedTypes => "shared_types",
            RelationType::Dependency => "dependency",
            RelationType::DevDependency => "dev_dependency",
            RelationType::MonorepoSibling => "monorepo_sibling",
            RelationType::ApiRelation => "api_relation",
            RelationType::Custom(name) => name,
        };

        conn.execute(
            r#"
            INSERT OR REPLACE INTO repository_relations
            (source_id, target_id, relation_type, data, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                relation.source_id,
                relation.target_id,
                relation_type,
                serde_json::to_string(&relation.data)?,
                now,
            ],
        )?;

        Ok(())
    }

    /// Get relationships for a repository
    pub async fn get_relations(&self, repo_id: &str) -> Result<Vec<RepositoryRelation>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            r#"
            SELECT source_id, target_id, relation_type, data 
            FROM repository_relations 
            WHERE source_id = ?1 OR target_id = ?1
            "#,
        )?;

        let relations = stmt
            .query_map(params![repo_id], |row| {
                let relation_type_str: String = row.get(2)?;
                let relation_type = match relation_type_str.as_str() {
                    "shared_types" => RelationType::SharedTypes,
                    "dependency" => RelationType::Dependency,
                    "dev_dependency" => RelationType::DevDependency,
                    "monorepo_sibling" => RelationType::MonorepoSibling,
                    "api_relation" => RelationType::ApiRelation,
                    other => RelationType::Custom(other.to_string()),
                };

                Ok(RepositoryRelation {
                    source_id: row.get(0)?,
                    target_id: row.get(1)?,
                    relation_type,
                    data: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(relations)
    }

    /// Find repositories by tag
    pub async fn find_by_tag(&self, tag: &str) -> Result<Vec<RepositoryInfo>> {
        let conn = self.conn.lock().await;

        let mut stmt =
            conn.prepare("SELECT * FROM repositories WHERE tags LIKE ?1 AND active = 1")?;

        let pattern = format!("%\"{tag}\"% ");
        let repos = stmt
            .query_map(params![pattern], |row| {
                Ok(RepositoryInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: PathBuf::from(row.get::<_, String>(2)?),
                    remote_url: row.get(3)?,
                    primary_language: row.get(4)?,
                    build_system: row.get(5)?,
                    is_monorepo_member: row.get(6)?,
                    monorepo_id: row.get(7)?,
                    tags: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
                    active: row.get(9)?,
                    last_diagnostic_run: row
                        .get::<_, Option<i64>>(10)?
                        .map(|ts| DateTime::from_timestamp(ts, 0).unwrap()),
                    metadata: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(repos)
    }

    /// Update last diagnostic run timestamp
    pub async fn update_diagnostic_timestamp(&self, repo_id: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        conn.execute(
            "UPDATE repositories SET last_diagnostic_run = ?1, updated_at = ?2 WHERE id = ?3",
            params![now.timestamp(), now.timestamp(), repo_id],
        )?;

        Ok(())
    }
}
