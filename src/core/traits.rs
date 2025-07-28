use crate::core::types::*;
use anyhow::Result;
use async_trait::async_trait;

/// Core trait for diagnostic bridges
#[async_trait]
pub trait DiagnosticsBridge {
    /// Initialize the bridge
    async fn initialize(&mut self) -> Result<()>;

    /// Capture current diagnostics
    async fn capture_diagnostics(&self) -> Result<DiagnosticSnapshot>;

    /// Subscribe to diagnostic updates
    async fn subscribe_to_diagnostics(
        &self,
        callback: Box<dyn Fn(Vec<Diagnostic>) + Send + Sync>,
    ) -> Result<()>;

    /// Filter diagnostics
    async fn filter_diagnostics(&self, filter: DiagnosticFilter) -> Result<Vec<Diagnostic>>;

    /// Check if a file should be excluded
    fn should_exclude_file(&self, file_path: &str) -> bool;

    /// Sanitize a diagnostic for privacy
    fn sanitize_diagnostic(&self, diagnostic: Diagnostic) -> Diagnostic;

    /// Start capturing diagnostics
    async fn start_capture(&mut self) -> Result<()>;

    /// Stop capturing diagnostics
    async fn stop_capture(&mut self) -> Result<()>;

    /// Dispose of resources
    async fn dispose(&mut self) -> Result<()>;
}

/// Trait for diagnostic caching
#[async_trait]
pub trait DiagnosticsCache {
    /// Store a snapshot
    async fn store(&mut self, snapshot: DiagnosticSnapshot) -> Result<()>;

    /// Get diagnostics with optional filtering
    async fn get(&self, filter: Option<DiagnosticFilter>) -> Result<Vec<Diagnostic>>;

    /// Get a specific snapshot by ID
    async fn get_snapshot(&self, id: &uuid::Uuid) -> Result<Option<DiagnosticSnapshot>>;

    /// Get recent snapshots
    async fn get_snapshots(&self, limit: Option<usize>) -> Result<Vec<DiagnosticSnapshot>>;

    /// Clear all cached data
    async fn clear(&mut self) -> Result<()>;

    /// Clean up old entries
    async fn cleanup(&mut self) -> Result<()>;
}

/// Trait for privacy filtering
pub trait PrivacyFilter {
    /// Apply privacy filtering to diagnostics
    fn apply(&self, diagnostics: Vec<Diagnostic>) -> Result<Vec<Diagnostic>>;

    /// Check if a diagnostic should be included
    fn should_include_diagnostic(&self, diagnostic: &Diagnostic) -> bool;

    /// Sanitize a single diagnostic
    fn sanitize_diagnostic(&self, diagnostic: Diagnostic) -> Diagnostic;
}

/// Trait for format conversion
#[async_trait]
pub trait FormatConverter {
    /// Normalize raw diagnostics from any LSP
    async fn normalize(
        &self,
        raw: RawDiagnostics,
    ) -> Result<Vec<Diagnostic>, crate::core::errors::ParseError>;

    /// Convert diagnostics to unified format
    fn convert_to_unified(
        &self,
        diagnostics: serde_json::Value,
        source: &str,
    ) -> Result<Vec<Diagnostic>, crate::core::errors::ParseError>;
}

/// Trait for exporting diagnostics
pub trait ExportService {
    /// Export to JSON format
    fn export_to_json(
        &self,
        snapshot: &DiagnosticSnapshot,
        config: &ExportConfig,
    ) -> Result<String, crate::core::errors::ExportError>;

    /// Export to Markdown format
    fn export_to_markdown(
        &self,
        snapshot: &DiagnosticSnapshot,
        config: &ExportConfig,
    ) -> Result<String, crate::core::errors::ExportError>;

    /// Export to Claude-optimized format
    fn export_to_claude_optimized(
        &self,
        snapshot: &DiagnosticSnapshot,
        config: &ExportConfig,
    ) -> Result<String, crate::core::errors::ExportError>;

    /// Generate a summary of diagnostics
    fn generate_summary(&self, diagnostics: &[Diagnostic]) -> DiagnosticSummary;
}

/// Trait for capturing diagnostics from various sources
#[async_trait]
pub trait DiagnosticsCaptureService {
    /// Process raw diagnostics
    async fn process_diagnostics(&mut self, raw: RawDiagnostics) -> Result<()>;

    /// Subscribe to snapshot updates
    async fn subscribe(
        &mut self,
        callback: Box<dyn Fn(DiagnosticSnapshot) + Send + Sync>,
    ) -> Result<()>;

    /// Get the current snapshot
    async fn get_current_snapshot(&self) -> Result<Option<DiagnosticSnapshot>>;

    /// Get snapshot history
    async fn get_history(&self, limit: Option<usize>) -> Result<Vec<DiagnosticSnapshot>>;
}

/// Configuration for privacy settings
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrivacyPolicy {
    pub exclude_patterns: Vec<String>,
    pub sanitize_strings: bool,
    pub sanitize_comments: bool,
    pub include_only_errors: bool,
    pub max_diagnostics_per_file: usize,
    pub anonymize_file_paths: bool,
    pub encrypt_exports: bool,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self {
            exclude_patterns: vec![
                "**/.env*".to_string(),
                "**/secrets/**".to_string(),
                "**/.git/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/*.log".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
            ],
            sanitize_strings: true,
            sanitize_comments: false,
            include_only_errors: false,
            max_diagnostics_per_file: 50,
            anonymize_file_paths: false,
            encrypt_exports: false,
        }
    }
}

impl PrivacyPolicy {
    pub fn strict() -> Self {
        Self {
            exclude_patterns: vec![
                "**/.env*".to_string(),
                "**/secrets/**".to_string(),
                "**/.git/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/*.log".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
                "**/config/**".to_string(),
                "**/.ssh/**".to_string(),
                "**/credentials/**".to_string(),
            ],
            sanitize_strings: true,
            sanitize_comments: true,
            include_only_errors: true,
            max_diagnostics_per_file: 20,
            anonymize_file_paths: true,
            encrypt_exports: true,
        }
    }

    pub fn permissive() -> Self {
        Self {
            exclude_patterns: vec!["**/.git/**".to_string(), "**/node_modules/**".to_string()],
            sanitize_strings: false,
            sanitize_comments: false,
            include_only_errors: false,
            max_diagnostics_per_file: 100,
            anonymize_file_paths: false,
            encrypt_exports: false,
        }
    }
}

/// Configuration for export settings
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub include_context: bool,
    pub context_lines: usize,
    pub include_summary: bool,
    pub group_by_file: bool,
    pub sort_by: SortBy,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExportFormat {
    Json,
    Markdown,
    ClaudeOptimized,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SortBy {
    Severity,
    File,
    Source,
    Timestamp,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Json,
            include_context: true,
            context_lines: 3,
            include_summary: true,
            group_by_file: false,
            sort_by: SortBy::Severity,
        }
    }
}

/// Overall bridge configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeConfig {
    pub privacy: PrivacyPolicy,
    pub export: ExportConfig,
    pub capture: CaptureConfig,
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CaptureConfig {
    pub real_time: bool,
    pub batch_size: usize,
    pub debounce_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheConfig {
    pub max_snapshots: usize,
    pub max_age_seconds: u64,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            privacy: PrivacyPolicy::default(),
            export: ExportConfig::default(),
            capture: CaptureConfig {
                real_time: true,
                batch_size: 100,
                debounce_ms: 500,
            },
            cache: CacheConfig {
                max_snapshots: 100,
                max_age_seconds: 24 * 60 * 60, // 24 hours
            },
        }
    }
}
