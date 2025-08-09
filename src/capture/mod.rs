pub mod capture_service;
pub mod memory_cache;

pub use capture_service::CaptureService;
pub use memory_cache::MemoryCache;

use crate::core::{
    DiagnosticSnapshot, RawDiagnostics, PrivacyPolicy
};
use crate::privacy::privacy_filter::PrivacyFilter;
use crate::format::format_converter::FormatConverter;
use anyhow::Result;

/// Simplified wrapper for diagnostic capture functionality
/// 
/// This provides an easy-to-use interface for tests and basic usage scenarios.
pub struct DiagnosticsCapture {
    service: CaptureService<MemoryCache, PrivacyFilter, FormatConverter>,
}

impl DiagnosticsCapture {
    /// Create a new DiagnosticsCapture instance with default configuration
    pub fn new() -> Self {
        let cache = MemoryCache::new(100, 3600); // 100 snapshots, 1 hour TTL
        let privacy_filter = PrivacyFilter::with_default_policy();
        let format_converter = FormatConverter::new();
        
        Self {
            service: CaptureService::new(cache, privacy_filter, format_converter),
        }
    }
    
    /// Process raw diagnostics and return a snapshot
    pub async fn process_diagnostics(&mut self, raw: RawDiagnostics) -> Result<DiagnosticSnapshot> {
        use crate::core::DiagnosticsCaptureService;
        
        self.service.process_diagnostics(raw).await?;
        
        // Get the current snapshot
        self.service
            .get_current_snapshot()
            .await?
            .ok_or_else(|| anyhow::anyhow!("No snapshot available after processing"))
    }
    
    /// Set privacy policy for filtering diagnostics
    pub fn set_privacy_policy(&mut self, policy: PrivacyPolicy) {
        // Update the privacy filter with the new policy
        let privacy_filter = PrivacyFilter::new(policy);
        
        // Create new service with updated privacy filter
        let cache = MemoryCache::new(100, 3600);
        let format_converter = FormatConverter::new();
        
        self.service = CaptureService::new(cache, privacy_filter, format_converter);
    }
    
    /// Set privacy policy with workspace filtering
    pub fn set_privacy_policy_with_workspace(&mut self, policy: PrivacyPolicy, workspace_root: std::path::PathBuf) {
        // Create privacy filter with workspace filtering
        let privacy_filter = PrivacyFilter::new(policy).with_workspace(workspace_root);
        
        // Create new service with updated privacy filter
        let cache = MemoryCache::new(100, 3600);
        let format_converter = FormatConverter::new();
        
        self.service = CaptureService::new(cache, privacy_filter, format_converter);
    }
    
    /// Get the current privacy policy
    pub fn get_privacy_policy(&self) -> PrivacyPolicy {
        self.service.get_privacy_policy()
    }
    
    /// Create a new DiagnosticsCapture with specific privacy policy
    pub fn with_privacy_policy(policy: PrivacyPolicy) -> Self {
        let cache = MemoryCache::new(100, 3600);
        let privacy_filter = PrivacyFilter::new(policy);
        let format_converter = FormatConverter::new();
        
        Self {
            service: CaptureService::new(cache, privacy_filter, format_converter),
        }
    }
    
    /// Create a new DiagnosticsCapture with strict privacy policy
    pub fn with_strict_privacy() -> Self {
        Self::with_privacy_policy(PrivacyPolicy::strict())
    }
    
    /// Create a new DiagnosticsCapture with permissive privacy policy  
    pub fn with_permissive_privacy() -> Self {
        Self::with_privacy_policy(PrivacyPolicy::permissive())
    }
    
    /// Start capturing diagnostics
    pub async fn start_capture(&self) -> Result<()> {
        self.service.start_capture().await
    }
    
    /// Create a snapshot from diagnostics
    pub fn create_snapshot(&self, diagnostics: Vec<crate::core::Diagnostic>) -> crate::core::DiagnosticSnapshot {
        use crate::core::{DiagnosticSnapshot, SnapshotMetadata, CaptureMethod, EditorInfo, WorkspaceInfo};
        use uuid::Uuid;
        use chrono::Utc;
        use std::collections::HashSet;
        
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
            capture_method: CaptureMethod::Manual,
            editor_info: EditorInfo {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
            },
            language_servers,
            total_files,
            filtered_count: diagnostics.len(),
        };

        let workspace = WorkspaceInfo {
            name: "test_workspace".to_string(),
            root_path: std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            language: None,
            version: None,
        };

        DiagnosticSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            workspace,
            diagnostics,
            metadata,
        }
    }
}

impl Default for DiagnosticsCapture {
    fn default() -> Self {
        Self::new()
    }
}

