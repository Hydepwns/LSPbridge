//! Optimization strategy for the processor

use crate::core::{ErrorRecoverySystem, PersistentCache};
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

/// Strategy for system optimization
pub struct OptimizationStrategy {
    persistent_cache: Option<Arc<PersistentCache>>,
    error_recovery: Arc<ErrorRecoverySystem>,
    last_optimization: RwLock<Instant>,
}

impl OptimizationStrategy {
    /// Create a new optimization strategy
    pub fn new(
        persistent_cache: Option<Arc<PersistentCache>>,
        error_recovery: Arc<ErrorRecoverySystem>,
    ) -> Self {
        Self {
            persistent_cache,
            error_recovery,
            last_optimization: RwLock::new(Instant::now()),
        }
    }

    /// Maybe run optimization if enough time has passed
    pub async fn maybe_optimize(&self) -> Result<()> {
        let now = Instant::now();
        let last_optimization = *self.last_optimization.read().await;

        if now.duration_since(last_optimization) >= Duration::from_secs(3600) {
            // 1 hour
            self.optimize().await?;
            *self.last_optimization.write().await = now;
        }

        Ok(())
    }

    /// Run optimization
    pub async fn optimize(&self) -> Result<()> {
        info!("Starting system optimization");
        let start = Instant::now();

        // Optimize persistent cache
        if let Some(persistent_cache) = &self.persistent_cache {
            persistent_cache.optimize().await?;
        }

        // Cleanup error recovery state
        self.error_recovery.cleanup_old_retry_counts().await;

        info!("System optimization completed in {:?}", start.elapsed());
        Ok(())
    }
}