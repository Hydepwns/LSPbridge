//! # LSPbridge
//!
//! A comprehensive Language Server Protocol (LSP) bridge for cross-language analysis,
//! diagnostic processing, and intelligent code assistance.
//!
//! LSPbridge provides tools for capturing, analyzing, and processing diagnostics from
//! multiple language servers, enabling cross-repository analysis, quick fixes,
//! privacy-aware filtering, and AI-powered code assistance.
//!
//! ## Core Features
//!
//! - **Multi-language Analysis**: Support for Rust, TypeScript, Python, and more
//! - **Cross-repository Intelligence**: Analyze dependencies and relationships across projects
//! - **Privacy-aware Processing**: Filter sensitive information with configurable privacy levels
//! - **Quick Fix Engine**: Automated code fixes with confidence scoring
//! - **AI Training Data**: Export diagnostics for machine learning model training
//! - **History Tracking**: Track diagnostic changes and trends over time
//! - **Export Capabilities**: Multiple output formats (JSON, CSV, Parquet, etc.)
//!
//! ## Quick Start
//!
//! ```rust
//! use lspbridge::{Cli, run_cli};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     run_cli().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Module Overview
//!
//! - [`analyzers`] - Language-specific diagnostic analyzers
//! - [`capture`] - Diagnostic capture and caching services
//! - [`cli`] - Command-line interface and argument parsing
//! - [`core`] - Core types, utilities, and processing engines
//! - [`export`] - Data export services for various formats
//! - [`query`] - Query API for diagnostic search and filtering
//! - [`quick_fix`] - Automated code fix generation and application
//! - [`multi_repo`] - Cross-repository analysis and collaboration
//! - [`privacy`] - Privacy filtering and workspace isolation
//! - [`ai_training`] - AI training data preparation and export

/// AI training data generation and export functionality
pub mod ai_training;
/// Language-specific diagnostic analyzers
pub mod analyzers;
/// Diagnostic capture and caching services
pub mod capture;
/// Command-line interface and argument parsing
pub mod cli;
/// Configuration management and validation
pub mod config;
/// Core types, utilities, and processing engines
pub mod core;
/// Error types and handling utilities
pub mod error;
/// Common error patterns and recovery strategies
pub mod error_patterns;
/// Data export services for multiple output formats
pub mod export;
/// Format conversion and output formatting utilities
pub mod format;
/// Diagnostic history tracking and analysis
pub mod history;
/// Cross-repository analysis and collaboration tools
pub mod multi_repo;
/// Privacy filtering and sensitive data protection
pub mod privacy;
/// Project structure analysis and build system detection
pub mod project;
/// Query API for diagnostic search and filtering
pub mod query;
/// Automated code fix generation and application
pub mod quick_fix;
/// Security utilities and input validation
pub mod security;

// Re-export core functionality for easy access
pub use core::*;
/// Re-export main error types for convenient error handling
pub use error::{LspBridgeError, LspResult, ErrorContext};
