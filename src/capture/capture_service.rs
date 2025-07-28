use crate::core::{
    CaptureMethod, Diagnostic, DiagnosticGroup, DiagnosticGrouper, DiagnosticSnapshot,
    DiagnosticsCache, DiagnosticsCaptureService, EditorInfo, FormatConverter, IncrementalProcessor,
    PrivacyFilter, ProcessingStats, RawDiagnostics, SnapshotMetadata, WorkspaceInfo,
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct CaptureService<C, P, F>
where
    C: DiagnosticsCache + Send + Sync,
    P: PrivacyFilter + Send + Sync,
    F: FormatConverter + Send + Sync,
{
    cache: Arc<RwLock<C>>,
    privacy_filter: Arc<P>,
    format_converter: Arc<F>,
    diagnostic_grouper: Arc<DiagnosticGrouper>,
    incremental_processor: Arc<IncrementalProcessor>,
    current_snapshot: Arc<RwLock<Option<DiagnosticSnapshot>>>,
    current_groups: Arc<RwLock<Option<Vec<DiagnosticGroup>>>>,
    subscribers: Arc<RwLock<Vec<Box<dyn Fn(DiagnosticSnapshot) + Send + Sync>>>>,
    is_capturing: Arc<RwLock<bool>>,
    enable_grouping: Arc<RwLock<bool>>,
    enable_incremental: Arc<RwLock<bool>>,
    last_stats: Arc<RwLock<Option<ProcessingStats>>>,
}

impl<C, P, F> CaptureService<C, P, F>
where
    C: DiagnosticsCache + Send + Sync,
    P: PrivacyFilter + Send + Sync,
    F: FormatConverter + Send + Sync,
{
    pub fn new(cache: C, privacy_filter: P, format_converter: F) -> Self {
        Self {
            cache: Arc::new(RwLock::new(cache)),
            privacy_filter: Arc::new(privacy_filter),
            format_converter: Arc::new(format_converter),
            diagnostic_grouper: Arc::new(DiagnosticGrouper::new()),
            incremental_processor: Arc::new(IncrementalProcessor::new()),
            current_snapshot: Arc::new(RwLock::new(None)),
            current_groups: Arc::new(RwLock::new(None)),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            is_capturing: Arc::new(RwLock::new(false)),
            enable_grouping: Arc::new(RwLock::new(true)),
            enable_incremental: Arc::new(RwLock::new(true)),
            last_stats: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_grouping_enabled(&self, enabled: bool) {
        let mut enable_grouping = self.enable_grouping.write().await;
        *enable_grouping = enabled;
    }

    pub async fn set_incremental_enabled(&self, enabled: bool) {
        let mut enable_incremental = self.enable_incremental.write().await;
        *enable_incremental = enabled;
    }

    pub async fn get_last_processing_stats(&self) -> Option<ProcessingStats> {
        let stats = self.last_stats.read().await;
        stats.clone()
    }

    pub async fn clear_incremental_cache(&self) -> Result<()> {
        self.incremental_processor.clear_cache().await
    }

    pub async fn get_current_groups(&self) -> Option<Vec<DiagnosticGroup>> {
        let groups = self.current_groups.read().await;
        groups.clone()
    }

    pub async fn start_capture(&self) -> Result<()> {
        let mut is_capturing = self.is_capturing.write().await;
        *is_capturing = true;
        tracing::info!("Diagnostic capture started");
        Ok(())
    }

    pub async fn stop_capture(&self) -> Result<()> {
        let mut is_capturing = self.is_capturing.write().await;
        *is_capturing = false;
        tracing::info!("Diagnostic capture stopped");
        Ok(())
    }

    fn create_snapshot(
        &self,
        diagnostics: Vec<Diagnostic>,
        raw: &RawDiagnostics,
    ) -> DiagnosticSnapshot {
        let language_servers: Vec<String> = diagnostics
            .iter()
            .map(|d| d.source.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let total_files = diagnostics
            .iter()
            .map(|d| &d.file)
            .collect::<HashSet<_>>()
            .len();

        let metadata = SnapshotMetadata {
            capture_method: CaptureMethod::Automatic,
            editor_info: EditorInfo {
                name: "unknown".to_string(),
                version: "unknown".to_string(),
            },
            language_servers,
            total_files,
            filtered_count: diagnostics.len(),
        };

        let workspace = raw.workspace.clone().unwrap_or_else(|| WorkspaceInfo {
            name: "unknown".to_string(),
            root_path: std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            language: None,
            version: None,
        });

        DiagnosticSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            workspace,
            diagnostics,
            metadata,
        }
    }

    async fn notify_subscribers(&self, snapshot: &DiagnosticSnapshot) -> Result<()> {
        let subscribers = self.subscribers.read().await;
        for callback in subscribers.iter() {
            callback(snapshot.clone());
        }
        Ok(())
    }
}

#[async_trait]
impl<C, P, F> DiagnosticsCaptureService for CaptureService<C, P, F>
where
    C: DiagnosticsCache + Send + Sync,
    P: PrivacyFilter + Send + Sync,
    F: FormatConverter + Send + Sync,
{
    async fn process_diagnostics(&mut self, raw: RawDiagnostics) -> Result<()> {
        let is_capturing = *self.is_capturing.read().await;
        if !is_capturing {
            return Ok(());
        }

        tracing::debug!("Processing diagnostics from source: {}", raw.source);

        // 1. Normalize format across different LSPs
        let normalized = self.format_converter.normalize(raw.clone()).await?;
        tracing::debug!("Normalized {} diagnostics", normalized.len());

        // 2. Apply privacy filtering
        let filtered = self.privacy_filter.apply(normalized)?;
        tracing::debug!("Filtered to {} diagnostics", filtered.len());

        // 3. Deduplicate diagnostics
        let deduplicated = self.diagnostic_grouper.deduplicate_diagnostics(filtered);
        tracing::debug!("Deduplicated to {} diagnostics", deduplicated.len());

        // 4. Group related diagnostics if enabled
        let groups = if *self.enable_grouping.read().await {
            let diagnostic_groups = self
                .diagnostic_grouper
                .group_diagnostics(deduplicated.clone());
            let summary = self.diagnostic_grouper.summarize_groups(&diagnostic_groups);
            tracing::debug!(
                "Grouped into {} groups (total: {}, primary errors: {}, cascading: {})",
                summary.total_groups,
                summary.total_diagnostics,
                summary.primary_errors,
                summary.cascading_errors
            );
            Some(diagnostic_groups)
        } else {
            None
        };

        // 5. Create snapshot
        let snapshot = self.create_snapshot(deduplicated, &raw);

        // 6. Cache for quick access
        {
            let mut cache = self.cache.write().await;
            cache.store(snapshot.clone()).await?;
        }

        // 7. Update current snapshot and groups
        {
            let mut current = self.current_snapshot.write().await;
            *current = Some(snapshot.clone());
        }

        if let Some(diagnostic_groups) = groups {
            let mut current_groups = self.current_groups.write().await;
            *current_groups = Some(diagnostic_groups);
        }

        // 8. Notify subscribers
        self.notify_subscribers(&snapshot).await?;

        tracing::info!(
            "Processed snapshot {} with {} diagnostics",
            snapshot.id,
            snapshot.diagnostics.len()
        );
        Ok(())
    }

    async fn subscribe(
        &mut self,
        callback: Box<dyn Fn(DiagnosticSnapshot) + Send + Sync>,
    ) -> Result<()> {
        let mut subscribers = self.subscribers.write().await;
        subscribers.push(callback);
        Ok(())
    }

    async fn get_current_snapshot(&self) -> Result<Option<DiagnosticSnapshot>> {
        let current = self.current_snapshot.read().await;
        Ok(current.clone())
    }

    async fn get_history(&self, limit: Option<usize>) -> Result<Vec<DiagnosticSnapshot>> {
        let cache = self.cache.read().await;
        cache.get_snapshots(limit).await
    }
}

impl<C, P, F> Clone for CaptureService<C, P, F>
where
    C: DiagnosticsCache + Send + Sync,
    P: PrivacyFilter + Send + Sync,
    F: FormatConverter + Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
            privacy_filter: Arc::clone(&self.privacy_filter),
            format_converter: Arc::clone(&self.format_converter),
            diagnostic_grouper: Arc::clone(&self.diagnostic_grouper),
            incremental_processor: Arc::clone(&self.incremental_processor),
            current_snapshot: Arc::clone(&self.current_snapshot),
            current_groups: Arc::clone(&self.current_groups),
            subscribers: Arc::clone(&self.subscribers),
            is_capturing: Arc::clone(&self.is_capturing),
            enable_grouping: Arc::clone(&self.enable_grouping),
            enable_incremental: Arc::clone(&self.enable_incremental),
            last_stats: Arc::clone(&self.last_stats),
        }
    }
}
