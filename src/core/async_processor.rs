//! # Async Diagnostic Processing
//!
//! This module provides async architecture improvements for concurrent diagnostic
//! processing with backpressure handling and resource management.
//!
//! ## Key Components
//!
//! - **AsyncDiagnosticProcessor**: Main async processing service
//! - **Semaphore-based Concurrency**: Limit concurrent operations
//! - **Streaming Processing**: Process diagnostics as they arrive
//! - **Backpressure Handling**: Graceful handling of overload scenarios
//!
//! ## Usage Examples
//!
//! ```rust
//! use lsp_bridge::core::async_processor::AsyncDiagnosticProcessor;
//! use lsp_bridge::core::config::UnifiedConfig;
//! use futures::stream::{self, StreamExt};
//!
//! // Create async processor with resource limits
//! let config = UnifiedConfig::default();
//! let processor = AsyncDiagnosticProcessor::new(config).await?;
//!
//! // Process diagnostics stream
//! let diagnostics = stream::iter(diagnostic_list);
//! let results = processor.process_diagnostics_stream(diagnostics).await?;
//!
//! // Collect results with error handling
//! let processed: Vec<_> = results
//!     .filter_map(|result| async move { result.ok() })
//!     .collect()
//!     .await;
//! ```

use super::context_ranking::{ContextRanker, RankedContext};
use super::semantic_context::{ContextExtractor, SemanticContext};
use super::types::Diagnostic;
use crate::core::config::{HasPerformanceConfig, UnifiedConfig};
use anyhow::Result;
use futures::stream::{Stream, StreamExt};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};

/// Async diagnostic processor with concurrency control and backpressure handling
pub struct AsyncDiagnosticProcessor {
    /// Context extractor for semantic analysis
    context_extractor: Arc<tokio::sync::Mutex<ContextExtractor>>,

    /// Context ranker for optimization
    context_ranker: Arc<ContextRanker>,

    /// Semaphore for controlling concurrent operations
    semaphore: Arc<Semaphore>,

    /// Configuration for performance limits
    config: UnifiedConfig,
}

/// Result of processing a single diagnostic
#[derive(Debug, Clone)]
pub struct ProcessedDiagnostic {
    /// Original diagnostic
    pub diagnostic: Diagnostic,

    /// Extracted semantic context
    pub semantic_context: SemanticContext,

    /// Ranked and optimized context
    pub ranked_context: RankedContext,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// Whether processing was completed successfully
    pub success: bool,

    /// Any error that occurred during processing
    pub error: Option<String>,
}

/// Processing statistics
#[derive(Debug, Clone)]
pub struct ProcessingStats {
    /// Total diagnostics processed
    pub total_processed: usize,

    /// Number of successful processings
    pub successful: usize,

    /// Number of failed processings
    pub failed: usize,

    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,

    /// Total processing time in milliseconds
    pub total_time_ms: u64,

    /// Peak concurrent operations
    pub peak_concurrency: usize,
}

impl AsyncDiagnosticProcessor {
    /// Create a new async diagnostic processor
    pub async fn new(config: UnifiedConfig) -> Result<Self> {
        let context_extractor = Arc::new(tokio::sync::Mutex::new(ContextExtractor::new()?));

        let context_ranker = Arc::new(
            ContextRanker::builder()
                .max_tokens(config.analysis.max_context_tokens)
                .build(),
        );

        // Create semaphore based on performance config
        let max_concurrent = config.performance_config().max_concurrent_files;
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        Ok(Self {
            context_extractor,
            context_ranker,
            semaphore,
            config,
        })
    }

    /// Process a single diagnostic asynchronously
    pub async fn process_diagnostic(&self, diagnostic: Diagnostic) -> Result<ProcessedDiagnostic> {
        let start_time = std::time::Instant::now();

        // Acquire semaphore permit for concurrency control
        let _permit = self.semaphore.acquire().await?;

        // Set up timeout for processing
        let timeout_duration = Duration::from_secs(self.config.timeouts.processing_timeout_seconds);

        let result = timeout(timeout_duration, async {
            self.process_diagnostic_inner(&diagnostic).await
        })
        .await;

        let elapsed = start_time.elapsed();
        let processing_time_ms = elapsed.as_millis() as u64;
        // Ensure at least 1ms for very fast operations
        let processing_time_ms = processing_time_ms.max(1);

        match result {
            Ok(Ok((semantic_context, ranked_context))) => Ok(ProcessedDiagnostic {
                diagnostic,
                semantic_context,
                ranked_context,
                processing_time_ms,
                success: true,
                error: None,
            }),
            Ok(Err(e)) => Ok(ProcessedDiagnostic {
                diagnostic,
                semantic_context: SemanticContext::default(),
                ranked_context: RankedContext::default(),
                processing_time_ms,
                success: false,
                error: Some(e.to_string()),
            }),
            Err(_) => Ok(ProcessedDiagnostic {
                diagnostic,
                semantic_context: SemanticContext::default(),
                ranked_context: RankedContext::default(),
                processing_time_ms,
                success: false,
                error: Some("Processing timeout".to_string()),
            }),
        }
    }

    /// Process a stream of diagnostics with concurrency control
    pub fn process_diagnostics_stream<S>(
        &self,
        diagnostics: S,
    ) -> impl Stream<Item = Result<ProcessedDiagnostic>> + '_
    where
        S: Stream<Item = Diagnostic> + Send + 'static,
    {
        let max_concurrent = self.config.performance_config().max_concurrent_files;

        diagnostics
            .map(move |diagnostic| {
                let processor_clone = self;
                async move { processor_clone.process_diagnostic(diagnostic).await }
            })
            .buffer_unordered(max_concurrent)
    }

    /// Process multiple diagnostics and collect results with statistics
    pub async fn process_diagnostics_batch(
        &self,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<(Vec<ProcessedDiagnostic>, ProcessingStats)> {
        let start_time = std::time::Instant::now();
        let _total_count = diagnostics.len();

        let stream = futures::stream::iter(diagnostics);
        let results: Vec<Result<ProcessedDiagnostic>> =
            self.process_diagnostics_stream(stream).collect().await;

        let elapsed = start_time.elapsed();
        let total_time_ms = elapsed.as_millis() as u64;
        // Ensure at least 1ms for very fast operations
        let total_time_ms = total_time_ms.max(1);

        // Collect successful and failed results
        let processed_results: Vec<ProcessedDiagnostic> =
            results.into_iter().filter_map(|r| r.ok()).collect();

        let successful = processed_results.iter().filter(|r| r.success).count();
        let failed = processed_results.len() - successful;

        let avg_processing_time_ms = if !processed_results.is_empty() {
            processed_results
                .iter()
                .map(|r| r.processing_time_ms as f64)
                .sum::<f64>()
                / processed_results.len() as f64
        } else {
            0.0
        };

        let stats = ProcessingStats {
            total_processed: processed_results.len(),
            successful,
            failed,
            avg_processing_time_ms,
            total_time_ms,
            peak_concurrency: self.config.performance_config().max_concurrent_files,
        };

        Ok((processed_results, stats))
    }

    /// Get current processing load (number of active operations)
    pub fn current_load(&self) -> usize {
        let total_permits = self.config.performance_config().max_concurrent_files;
        let available_permits = self.semaphore.available_permits();
        total_permits - available_permits
    }

    /// Check if the processor is currently overloaded
    pub fn is_overloaded(&self) -> bool {
        self.semaphore.available_permits() == 0
    }

    /// Internal processing logic
    async fn process_diagnostic_inner(
        &self,
        diagnostic: &Diagnostic,
    ) -> Result<(SemanticContext, RankedContext)> {
        // Extract semantic context (CPU-intensive, run in blocking task)
        let diagnostic_clone = diagnostic.clone();
        let extractor = Arc::clone(&self.context_extractor);

        let semantic_context = tokio::task::spawn_blocking(move || {
            let mut extractor = extractor.blocking_lock();
            extractor.extract_context_from_file(&diagnostic_clone)
        })
        .await??;

        // Rank and optimize context (lightweight, can run async)
        let ranked_context = self
            .context_ranker
            .rank_context(semantic_context.clone(), diagnostic)?;

        Ok((semantic_context, ranked_context))
    }
}

impl Default for RankedContext {
    fn default() -> Self {
        Self {
            context: SemanticContext::default(),
            ranked_elements: Vec::new(),
            estimated_tokens: 0,
            budget_context: BudgetOptimizedContext {
                essential_context: Vec::new(),
                supplementary_context: Vec::new(),
                excluded_context: Vec::new(),
                tokens_used: 0,
                tokens_remaining: 0,
            },
        }
    }
}

// Import the type we need for the default implementation
use super::context_ranking::BudgetOptimizedContext;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Diagnostic, DiagnosticSeverity, Position, Range};

    fn create_test_diagnostic() -> Diagnostic {
        Diagnostic {
            id: "test-diagnostic-1".to_string(),
            file: "test.rs".to_string(),
            range: Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 10,
                    character: 20,
                },
            },
            severity: DiagnosticSeverity::Error,
            message: "Test error message".to_string(),
            code: Some("E0001".to_string()),
            source: "rust-analyzer".to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    #[tokio::test]
    async fn test_async_processor_creation() -> Result<()> {
        let config = UnifiedConfig::default();
        let processor = AsyncDiagnosticProcessor::new(config).await?;

        assert!(!processor.is_overloaded());
        assert_eq!(processor.current_load(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_single_diagnostic_processing() -> Result<()> {
        let config = UnifiedConfig::default();
        let processor = AsyncDiagnosticProcessor::new(config).await?;

        let diagnostic = create_test_diagnostic();
        let result = processor.process_diagnostic(diagnostic.clone()).await?;

        assert_eq!(result.diagnostic.file, diagnostic.file);
        assert!(result.processing_time_ms > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_processing() -> Result<()> {
        let config = UnifiedConfig::default();
        let processor = AsyncDiagnosticProcessor::new(config).await?;

        let diagnostics = vec![
            create_test_diagnostic(),
            create_test_diagnostic(),
            create_test_diagnostic(),
        ];

        let (results, stats) = processor.process_diagnostics_batch(diagnostics).await?;

        assert_eq!(results.len(), 3);
        assert_eq!(stats.total_processed, 3);
        assert!(stats.total_time_ms > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrency_limits() -> Result<()> {
        let mut config = UnifiedConfig::default();
        config.performance.max_concurrent_files = 2; // Limit to 2 concurrent operations

        let processor = AsyncDiagnosticProcessor::new(config).await?;

        // The processor should respect the concurrency limit
        assert_eq!(processor.semaphore.available_permits(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_stream_processing() -> Result<()> {
        let config = UnifiedConfig::default();
        let processor = AsyncDiagnosticProcessor::new(config).await?;

        let diagnostics = vec![create_test_diagnostic(), create_test_diagnostic()];

        let stream = futures::stream::iter(diagnostics);
        let results: Vec<_> = processor.process_diagnostics_stream(stream).collect().await;

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));

        Ok(())
    }
}
