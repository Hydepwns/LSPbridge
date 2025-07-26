use crate::core::{
    DiagnosticsCaptureService, DiagnosticsCache, PrivacyFilter, FormatConverter,
    RawDiagnostics, DiagnosticSnapshot, Diagnostic, WorkspaceInfo, SnapshotMetadata,
    CaptureMethod, EditorInfo
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
    current_snapshot: Arc<RwLock<Option<DiagnosticSnapshot>>>,
    subscribers: Arc<RwLock<Vec<Box<dyn Fn(DiagnosticSnapshot) + Send + Sync>>>>,
    is_capturing: Arc<RwLock<bool>>,
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
            current_snapshot: Arc::new(RwLock::new(None)),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            is_capturing: Arc::new(RwLock::new(false)),
        }
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

    fn create_snapshot(&self, diagnostics: Vec<Diagnostic>, raw: &RawDiagnostics) -> DiagnosticSnapshot {
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

        // 3. Create snapshot
        let snapshot = self.create_snapshot(filtered, &raw);

        // 4. Cache for quick access
        {
            let mut cache = self.cache.write().await;
            cache.store(snapshot.clone()).await?;
        }

        // 5. Update current snapshot
        {
            let mut current = self.current_snapshot.write().await;
            *current = Some(snapshot.clone());
        }

        // 6. Notify subscribers
        self.notify_subscribers(&snapshot).await?;

        tracing::info!("Processed snapshot {} with {} diagnostics", snapshot.id, snapshot.diagnostics.len());
        Ok(())
    }

    async fn subscribe(&mut self, callback: Box<dyn Fn(DiagnosticSnapshot) + Send + Sync>) -> Result<()> {
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
            current_snapshot: Arc::clone(&self.current_snapshot),
            subscribers: Arc::clone(&self.subscribers),
            is_capturing: Arc::clone(&self.is_capturing),
        }
    }
}