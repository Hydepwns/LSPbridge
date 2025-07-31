use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::core::security_config::PrivacyLevel;
use crate::history::HistoryAction;
use crate::ai_training::AITrainingAction;
use crate::quick_fix::QuickFixAction;
use crate::config::ConfigAction;

/// Main CLI structure for LSPbridge - a universal bridge for exporting IDE diagnostics.
/// 
/// LSPbridge captures diagnostics from Language Server Protocol (LSP) servers
/// and provides various export, analysis, and processing capabilities.
/// 
/// # Examples
/// 
/// ```bash
/// # Export current diagnostics to JSON
/// lspbridge export --format json --output diagnostics.json
/// 
/// # Start interactive query session
/// lspbridge query --interactive
/// 
/// # Generate AI training data
/// lspbridge ai-training export training_data.jsonl
/// ```
#[derive(Parser)]
#[command(name = "lspbridge")]
#[command(about = "Universal bridge for exporting IDE diagnostics to AI assistants")]
#[command(version)]
pub struct Cli {
    /// The command to execute
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging for debugging
    #[arg(short, long)]
    pub verbose: bool,
}

/// Available CLI commands for LSPbridge.
/// 
/// Each command provides specific functionality for working with diagnostic data:
/// - [`Export`] - One-time export of current diagnostics
/// - [`Watch`] - Continuous monitoring and export of diagnostics 
/// - [`Query`] - Interactive or scripted querying of diagnostic data
/// - [`History`] - Analysis of historical diagnostic trends
/// - [`AITraining`] - AI/ML training data generation
/// - [`QuickFix`] - Automated code fix generation and application
/// - [`Config`] - Configuration management
/// - [`MultiRepo`] - Cross-repository analysis
#[derive(Subcommand)]
pub enum Commands {
    /// Export current diagnostics
    Export {
        /// Export format
        #[arg(short, long, value_enum, default_value = "json")]
        format: OutputFormat,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Include only errors
        #[arg(long)]
        errors_only: bool,

        /// Include only warnings and errors
        #[arg(long)]
        warnings_and_errors: bool,

        /// File patterns to include (comma-separated)
        #[arg(long)]
        files: Option<String>,

        /// File patterns to exclude (comma-separated)
        #[arg(long)]
        exclude: Option<String>,

        /// Maximum number of diagnostics
        #[arg(long)]
        max_results: Option<usize>,

        /// Include code context around diagnostics
        #[arg(long)]
        include_context: bool,

        /// Number of context lines
        #[arg(long, default_value = "3")]
        context_lines: usize,

        /// Privacy level for data sanitization
        #[arg(long, value_enum, default_value = "balanced")]
        privacy: PrivacyLevel,
    },

    /// Watch for diagnostic changes
    Watch {
        /// Export format
        #[arg(short, long, value_enum, default_value = "json")]
        format: OutputFormat,

        /// Check interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,

        /// Include only errors
        #[arg(long)]
        errors_only: bool,

        /// Privacy level for data sanitization
        #[arg(long, value_enum, default_value = "balanced")]
        privacy: PrivacyLevel,
    },

    /// Query diagnostic history
    Query {
        /// Query string (SQL-like syntax)
        #[arg(short, long)]
        query: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: QueryOutputFormat,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,
    },

    /// Manage diagnostic history
    History {
        /// History action to perform
        #[command(subcommand)]
        action: HistoryAction,
    },

    /// Generate AI training data
    #[command(name = "ai-training")]
    AITraining {
        /// AI training action to perform
        #[command(subcommand)]
        action: AITrainingAction,
    },

    /// Apply quick fixes
    #[command(name = "quick-fix")]
    QuickFix {
        /// Quick fix action to perform
        #[command(subcommand)]
        action: QuickFixAction,
    },

    /// Manage configuration
    Config {
        /// Configuration action to perform
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Multi-repository operations
    #[command(name = "multi-repo")]
    MultiRepo {
        /// Multi-repo command to execute
        #[command(subcommand)]
        command: crate::cli::multi_repo::MultiRepoCommand,
    },
}

/// Output formats for export commands
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// JSON format
    Json,
    /// Markdown format
    Markdown,
    /// Claude-optimized XML format
    Claude,
}

/// Output formats for query commands
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum QueryOutputFormat {
    /// Table format
    Table,
    /// JSON format
    Json,
    /// CSV format
    Csv,
}

// Argument structures for command handlers
pub struct ExportArgs {
    pub format: OutputFormat,
    pub output: Option<PathBuf>,
    pub errors_only: bool,
    pub warnings_and_errors: bool,
    pub files: Option<String>,
    pub exclude: Option<String>,
    pub max_results: Option<usize>,
    pub include_context: bool,
    pub context_lines: usize,
    pub privacy: PrivacyLevel,
}

pub struct WatchArgs {
    pub format: OutputFormat,
    pub interval: u64,
    pub errors_only: bool,
    pub privacy: PrivacyLevel,
}

pub struct QueryArgs {
    pub query: Option<String>,
    pub format: QueryOutputFormat,
    pub output: Option<PathBuf>,
    pub interactive: bool,
}