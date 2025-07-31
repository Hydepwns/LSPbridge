pub mod annotation;
pub mod data_structures;
pub mod export;
pub mod synthetic;

pub use annotation::{AnnotationReport, AnnotationTool, FixQuality};
pub use data_structures::{FixConfidence, TrainingDataset, TrainingPair};
pub use export::{ExportFormat, TrainingExporter};
pub use synthetic::{DifficultyLevel, ErrorInjector};

use clap::{Subcommand, ValueEnum};
use std::path::PathBuf;

/// Export formats for AI training data
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum AIExportFormat {
    /// JSON Lines format
    JsonLines,
    /// Apache Parquet format
    Parquet,
    /// HuggingFace dataset format
    HuggingFace,
    /// OpenAI fine-tuning format
    OpenAI,
    /// Custom format
    Custom,
}

/// AI training data actions
#[derive(Debug, Clone, Subcommand)]
pub enum AITrainingAction {
    /// Export diagnostics as training data
    Export {
        /// Output file path
        output: PathBuf,
        /// Export format
        #[arg(short, long, value_enum, default_value = "json-lines")]
        format: AIExportFormat,
        /// Only include high-confidence fixes
        #[arg(long)]
        high_confidence_only: bool,
        /// Maximum tokens per example
        #[arg(long)]
        max_tokens: Option<usize>,
        /// Filter by language
        #[arg(short, long)]
        language: Option<String>,
    },
    /// Generate synthetic training data
    Synthetic {
        /// Input code file
        input: PathBuf,
        /// Output dataset file
        output: PathBuf,
        /// Programming language
        #[arg(short, long)]
        language: String,
        /// Difficulty level
        #[arg(short, long, value_enum)]
        difficulty: Option<DifficultyLevel>,
        /// Number of examples to generate
        #[arg(short, long, default_value = "100")]
        count: usize,
        /// Generate gradient of difficulties
        #[arg(long)]
        gradient: bool,
    },
    /// Annotate training data for quality
    Annotate {
        /// Dataset file to annotate
        dataset: PathBuf,
        /// Annotator name
        #[arg(short, long)]
        annotator: String,
        /// Auto-annotate with quality threshold
        #[arg(long, value_enum)]
        auto_quality: Option<FixQuality>,
        /// Output file for annotated dataset
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Generate annotation report
    Report {
        /// Annotated dataset file
        dataset: PathBuf,
        /// Report format
        #[arg(short, long, value_enum, default_value = "markdown")]
        format: crate::cli::OutputFormat,
    },
}
