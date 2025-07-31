pub mod confidence;
pub mod engine;
pub mod rollback;
pub mod verification;

pub use confidence::{ConfidenceScore, ConfidenceThreshold, FixConfidenceScorer};
pub use engine::{FixApplicationEngine, FixEdit, FixResult};
pub use rollback::{RollbackManager, RollbackState};
pub use verification::{FixVerifier, VerificationResult};

use clap::Subcommand;

/// Quick fix actions for automatic code corrections
#[derive(Debug, Clone, Subcommand)]
pub enum QuickFixAction {
    /// Apply available quick fixes
    Apply {
        /// Confidence threshold for auto-applying fixes (0.0-1.0)
        #[arg(short = 't', long, default_value = "0.9")]
        threshold: f64,
        /// Only fix errors (skip warnings)
        #[arg(long)]
        errors_only: bool,
        /// Verify fixes pass tests
        #[arg(long)]
        verify_tests: bool,
        /// Verify fixes pass build
        #[arg(long)]
        verify_build: bool,
        /// Create backups before applying fixes
        #[arg(short, long)]
        backup: bool,
        /// Dry run - show what would be fixed
        #[arg(short, long)]
        dry_run: bool,
        /// File pattern to fix (e.g. "*.rs")
        #[arg(short, long)]
        files: Option<String>,
    },
    /// Rollback previously applied fixes
    Rollback {
        /// Session ID to rollback (latest if not specified)
        #[arg(short, long)]
        session_id: Option<String>,
        /// List available rollback sessions
        #[arg(short, long)]
        list: bool,
    },
    /// Analyze fix confidence scores
    Analyze {
        /// Show detailed confidence factors
        #[arg(short, long)]
        detailed: bool,
        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: crate::cli::OutputFormat,
    },
}
