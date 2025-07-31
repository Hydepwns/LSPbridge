use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Multi-repository CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiRepoCliConfig {
    /// Default output format for commands
    pub default_output_format: OutputFormat,
    
    /// Whether to automatically detect monorepos
    pub auto_detect_monorepos: bool,
    
    /// System limits and constraints
    pub limits: SystemLimits,
    
    /// Workspace configuration
    pub workspace: WorkspaceConfig,
    
    /// Analysis configuration
    pub analysis: AnalysisConfig,
    
    /// Team collaboration configuration
    pub team: TeamConfig,
    
    /// Repository discovery configuration
    pub discovery: DiscoveryConfig,
    
    /// Custom command aliases
    pub aliases: HashMap<String, String>,
}

impl Default for MultiRepoCliConfig {
    fn default() -> Self {
        let mut aliases = HashMap::new();
        aliases.insert("ls".to_string(), "list".to_string());
        aliases.insert("reg".to_string(), "register".to_string());
        aliases.insert("analyze".to_string(), "analyze".to_string());

        Self {
            default_output_format: OutputFormat::Table,
            auto_detect_monorepos: true,
            limits: SystemLimits::default(),
            workspace: WorkspaceConfig::default(),
            analysis: AnalysisConfig::default(),
            team: TeamConfig::default(),
            discovery: DiscoveryConfig::default(),
            aliases,
        }
    }
}

/// Output format options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}

/// System limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLimits {
    /// Maximum number of repositories to manage
    pub max_repositories: usize,
    
    /// Maximum depth for analysis operations
    pub max_analysis_depth: usize,
    
    /// Maximum file size to process (in MB)
    pub max_file_size_mb: usize,
    
    /// Maximum number of concurrent operations
    pub max_concurrent_operations: usize,
    
    /// Timeout for repository operations (in seconds)
    pub operation_timeout_seconds: u64,
}

impl Default for SystemLimits {
    fn default() -> Self {
        Self {
            max_repositories: 1000,
            max_analysis_depth: 10,
            max_file_size_mb: 100,
            max_concurrent_operations: 10,
            operation_timeout_seconds: 300, // 5 minutes
        }
    }
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Default workspace root directory
    pub default_root: Option<PathBuf>,
    
    /// Default synchronization mode
    pub default_sync_mode: SyncMode,
    
    /// File patterns to include by default
    pub default_include_patterns: Vec<String>,
    
    /// File patterns to exclude by default
    pub default_exclude_patterns: Vec<String>,
    
    /// Whether to preserve file timestamps
    pub preserve_timestamps: bool,
    
    /// Whether to create workspace index
    pub create_index: bool,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            default_root: None,
            default_sync_mode: SyncMode::Incremental,
            default_include_patterns: vec![
                "*.rs".to_string(),
                "*.ts".to_string(),
                "*.js".to_string(),
                "*.py".to_string(),
                "*.go".to_string(),
                "*.java".to_string(),
                "Cargo.toml".to_string(),
                "package.json".to_string(),
                "*.md".to_string(),
            ],
            default_exclude_patterns: vec![
                "target/**".to_string(),
                "node_modules/**".to_string(),
                ".git/**".to_string(),
                "*.log".to_string(),
            ],
            preserve_timestamps: true,
            create_index: true,
        }
    }
}

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Minimum impact threshold for displaying results
    pub min_impact_threshold: f32,
    
    /// Whether to include inactive repositories in analysis
    pub include_inactive_repos: bool,
    
    /// Language weights for impact calculation
    pub language_weights: HashMap<String, f32>,
    
    /// Whether to cache analysis results
    pub cache_results: bool,
    
    /// Cache duration in hours
    pub cache_duration_hours: u64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        let mut language_weights = HashMap::new();
        language_weights.insert("typescript".to_string(), 1.0);
        language_weights.insert("javascript".to_string(), 1.0);
        language_weights.insert("rust".to_string(), 0.9);
        language_weights.insert("python".to_string(), 0.8);
        language_weights.insert("java".to_string(), 0.7);

        Self {
            min_impact_threshold: 0.3,
            include_inactive_repos: false,
            language_weights,
            cache_results: true,
            cache_duration_hours: 24,
        }
    }
}

/// Team collaboration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    /// Maximum assignments per team member
    pub max_assignments_per_member: usize,
    
    /// Default assignment priority
    pub default_priority: Priority,
    
    /// Whether to send notifications
    pub enable_notifications: bool,
    
    /// Assignment timeout in days
    pub assignment_timeout_days: u32,
    
    /// Whether to track assignment history
    pub track_history: bool,
}

impl Default for TeamConfig {
    fn default() -> Self {
        Self {
            max_assignments_per_member: 20,
            default_priority: Priority::Medium,
            enable_notifications: false,
            assignment_timeout_days: 30,
            track_history: true,
        }
    }
}

/// Repository discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Maximum search depth
    pub max_search_depth: usize,
    
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
    
    /// Custom repository indicators
    pub custom_indicators: Vec<String>,
    
    /// Whether to detect Git submodules
    pub detect_submodules: bool,
    
    /// Minimum repository size (in KB)
    pub min_repo_size_kb: u64,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            max_search_depth: 5,
            follow_symlinks: false,
            custom_indicators: Vec::new(),
            detect_submodules: true,
            min_repo_size_kb: 1, // 1KB minimum
        }
    }
}

/// Synchronization modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMode {
    Full,
    Incremental,
    SymbolicLinks,
}

/// Priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Configuration validation rules
#[derive(Debug, Clone)]
pub struct ValidationRules {
    /// Required configuration keys
    pub required_keys: Vec<String>,
    
    /// Validation constraints
    pub constraints: HashMap<String, ValidationConstraint>,
}

impl Default for ValidationRules {
    fn default() -> Self {
        let mut constraints = HashMap::new();
        
        constraints.insert(
            "max_repositories".to_string(),
            ValidationConstraint::Range { min: 1, max: 10000 }
        );
        
        constraints.insert(
            "max_analysis_depth".to_string(),
            ValidationConstraint::Range { min: 1, max: 50 }
        );
        
        constraints.insert(
            "min_impact_threshold".to_string(),
            ValidationConstraint::FloatRange { min: 0.0, max: 1.0 }
        );

        Self {
            required_keys: vec![
                "default_output_format".to_string(),
                "limits".to_string(),
                "workspace".to_string(),
            ],
            constraints,
        }
    }
}

/// Validation constraint types
#[derive(Debug, Clone)]
pub enum ValidationConstraint {
    Range { min: usize, max: usize },
    FloatRange { min: f32, max: f32 },
    StringLength { min: usize, max: usize },
    PathExists,
    OneOf(Vec<String>),
}