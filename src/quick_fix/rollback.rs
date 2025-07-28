use crate::quick_fix::engine::FileBackup;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

/// Manages rollback state for applied fixes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackState {
    /// Unique session ID
    pub session_id: String,
    /// Timestamp when fixes were applied
    pub timestamp: DateTime<Utc>,
    /// Backups of modified files
    pub backups: Vec<FileBackup>,
    /// Description of what was fixed
    pub description: String,
    /// Whether this state has been rolled back
    pub rolled_back: bool,
}

/// Manages rollback operations
pub struct RollbackManager {
    /// Directory to store rollback states
    state_dir: PathBuf,
    /// Maximum number of rollback states to keep
    max_states: usize,
    /// In-memory cache of recent states
    state_cache: HashMap<String, RollbackState>,
}

impl RollbackManager {
    pub fn new(state_dir: PathBuf) -> Self {
        Self {
            state_dir,
            max_states: 10,
            state_cache: HashMap::new(),
        }
    }

    pub fn with_max_states(mut self, max: usize) -> Self {
        self.max_states = max;
        self
    }

    /// Initialize the rollback manager
    pub async fn init(&mut self) -> Result<()> {
        // Create state directory if it doesn't exist
        fs::create_dir_all(&self.state_dir)
            .await
            .context("Failed to create rollback state directory")?;

        // Load existing states
        self.load_states().await?;

        Ok(())
    }

    /// Save a rollback state
    pub async fn save_state(&mut self, state: RollbackState) -> Result<()> {
        // Add to cache
        self.state_cache
            .insert(state.session_id.clone(), state.clone());

        // Save to disk
        let state_file = self.state_dir.join(format!("{}.json", state.session_id));
        let json = serde_json::to_string_pretty(&state)?;
        fs::write(&state_file, json)
            .await
            .context("Failed to save rollback state")?;

        // Clean up old states if needed
        self.cleanup_old_states().await?;

        Ok(())
    }

    /// Create a rollback state from backups
    pub fn create_state(backups: Vec<FileBackup>, description: String) -> RollbackState {
        RollbackState {
            session_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            backups,
            description,
            rolled_back: false,
        }
    }

    /// Rollback to a previous state
    pub async fn rollback(&mut self, session_id: &str) -> Result<()> {
        let state = self
            .get_state(session_id)
            .await?
            .context("Rollback state not found")?;

        if state.rolled_back {
            return Err(anyhow::anyhow!("This state has already been rolled back"));
        }

        // Restore each file
        for backup in &state.backups {
            self.restore_file(backup).await?;
        }

        // Mark as rolled back
        let mut updated_state = state;
        updated_state.rolled_back = true;
        self.save_state(updated_state).await?;

        Ok(())
    }

    /// Rollback the most recent fixes
    pub async fn rollback_latest(&mut self) -> Result<()> {
        let latest = self
            .get_latest_state()
            .await?
            .context("No rollback states available")?;

        self.rollback(&latest.session_id).await
    }

    /// Get a specific rollback state
    pub async fn get_state(&self, session_id: &str) -> Result<Option<RollbackState>> {
        // Check cache first
        if let Some(state) = self.state_cache.get(session_id) {
            return Ok(Some(state.clone()));
        }

        // Load from disk
        let state_file = self.state_dir.join(format!("{}.json", session_id));
        if !state_file.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&state_file).await?;
        let state: RollbackState = serde_json::from_str(&json)?;

        Ok(Some(state))
    }

    /// Get the most recent rollback state
    pub async fn get_latest_state(&self) -> Result<Option<RollbackState>> {
        let mut latest: Option<RollbackState> = None;

        for state in self.state_cache.values() {
            if !state.rolled_back {
                if let Some(ref current_latest) = latest {
                    if state.timestamp > current_latest.timestamp {
                        latest = Some(state.clone());
                    }
                } else {
                    latest = Some(state.clone());
                }
            }
        }

        Ok(latest)
    }

    /// List all available rollback states
    pub async fn list_states(&self) -> Result<Vec<RollbackState>> {
        let mut states: Vec<RollbackState> = self.state_cache.values().cloned().collect();

        // Sort by timestamp (newest first)
        states.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(states)
    }

    /// Restore a single file from backup
    async fn restore_file(&self, backup: &FileBackup) -> Result<()> {
        fs::write(&backup.file_path, &backup.original_content)
            .await
            .with_context(|| format!("Failed to restore file: {:?}", backup.file_path))?;

        Ok(())
    }

    /// Load existing states from disk
    async fn load_states(&mut self) -> Result<()> {
        let mut entries = fs::read_dir(&self.state_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(json) = fs::read_to_string(&path).await {
                    if let Ok(state) = serde_json::from_str::<RollbackState>(&json) {
                        self.state_cache.insert(state.session_id.clone(), state);
                    }
                }
            }
        }

        Ok(())
    }

    /// Clean up old rollback states
    async fn cleanup_old_states(&mut self) -> Result<()> {
        if self.state_cache.len() <= self.max_states {
            return Ok(());
        }

        // Get states sorted by timestamp
        let mut states: Vec<(String, DateTime<Utc>)> = self
            .state_cache
            .iter()
            .map(|(id, state)| (id.clone(), state.timestamp))
            .collect();

        states.sort_by(|a, b| b.1.cmp(&a.1));

        // Remove oldest states
        for (id, _) in states.iter().skip(self.max_states) {
            self.state_cache.remove(id);
            let state_file = self.state_dir.join(format!("{}.json", id));
            let _ = fs::remove_file(&state_file).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_rollback_manager() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = RollbackManager::new(temp_dir.path().to_path_buf());
        manager.init().await.unwrap();

        // Create a backup
        let backup = FileBackup {
            file_path: PathBuf::from("test.rs"),
            original_content: "original content".to_string(),
            timestamp: Utc::now(),
        };

        let state = RollbackManager::create_state(vec![backup], "Test fix".to_string());

        let session_id = state.session_id.clone();

        // Save state
        manager.save_state(state).await.unwrap();

        // Verify we can retrieve it
        let retrieved = manager.get_state(&session_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().description, "Test fix");

        // List states
        let states = manager.list_states().await.unwrap();
        assert_eq!(states.len(), 1);
    }

    #[tokio::test]
    async fn test_state_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = RollbackManager::new(temp_dir.path().to_path_buf()).with_max_states(2);
        manager.init().await.unwrap();

        // Create multiple states
        for i in 0..5 {
            let backup = FileBackup {
                file_path: PathBuf::from(format!("test{}.rs", i)),
                original_content: format!("content {}", i),
                timestamp: Utc::now(),
            };

            let state = RollbackManager::create_state(vec![backup], format!("Fix {}", i));

            manager.save_state(state).await.unwrap();

            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Should only keep 2 most recent
        let states = manager.list_states().await.unwrap();
        assert_eq!(states.len(), 2);
    }
}
