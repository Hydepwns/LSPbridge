use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,      // Single file failures
    Medium,   // Multiple file failures, cache corruption
    High,     // System failures, disk issues
    Critical, // Complete processing failure
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    Retry,
    RetryWithBackoff,
    Fallback,
    SkipFile,
    ClearCache,
    FullReprocessing,
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct ErrorEvent {
    pub error: String,
    pub file_path: Option<PathBuf>,
    pub severity: ErrorSeverity,
    pub timestamp: Instant,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct RecoveryStrategy {
    pub max_retries: usize,
    pub backoff_multiplier: f64,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub circuit_breaker_threshold: usize,
    pub circuit_breaker_timeout: Duration,
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_multiplier: 2.0,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing fast
    HalfOpen, // Testing recovery
}

pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_count: AtomicUsize,
    last_failure_time: RwLock<Option<Instant>>,
    success_count: AtomicUsize,
    strategy: RecoveryStrategy,
}

impl CircuitBreaker {
    pub fn new(strategy: RecoveryStrategy) -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicUsize::new(0),
            last_failure_time: RwLock::new(None),
            success_count: AtomicUsize::new(0),
            strategy,
        }
    }

    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display + From<String>,
    {
        let state = *self.state.read().await;

        match state {
            CircuitState::Open => {
                if self.should_attempt_reset().await {
                    *self.state.write().await = CircuitState::HalfOpen;
                    info!("Circuit breaker transitioning to half-open");
                } else {
                    warn!("Circuit breaker is open - failing fast");
                    return Err(E::from("Circuit breaker is open".to_string()));
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests through
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }

        match operation.await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(error) => {
                self.record_failure().await;
                Err(error)
            }
        }
    }

    async fn should_attempt_reset(&self) -> bool {
        if let Some(last_failure) = *self.last_failure_time.read().await {
            last_failure.elapsed() >= self.strategy.circuit_breaker_timeout
        } else {
            false
        }
    }

    async fn record_success(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::HalfOpen => {
                let success_count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if success_count >= 3 {
                    *self.state.write().await = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                    info!("Circuit breaker closed - recovery successful");
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::Open => {
                // Should not happen
            }
        }
    }

    async fn record_failure(&self) {
        let failure_count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_failure_time.write().await = Some(Instant::now());

        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                if failure_count >= self.strategy.circuit_breaker_threshold {
                    *self.state.write().await = CircuitState::Open;
                    warn!("Circuit breaker opened due to {} failures", failure_count);
                }
            }
            CircuitState::HalfOpen => {
                *self.state.write().await = CircuitState::Open;
                self.success_count.store(0, Ordering::SeqCst);
                warn!("Circuit breaker reopened after half-open failure");
            }
            CircuitState::Open => {
                // Already open
            }
        }
    }

    pub async fn is_open(&self) -> bool {
        *self.state.read().await == CircuitState::Open
    }
}

pub struct ErrorRecoverySystem {
    circuit_breaker: Arc<CircuitBreaker>,
    error_history: RwLock<Vec<ErrorEvent>>,
    retry_counts: RwLock<HashMap<PathBuf, usize>>,
    strategy: RecoveryStrategy,
    total_errors: AtomicU64,
    recovery_attempts: AtomicU64,
}

impl ErrorRecoverySystem {
    pub fn new(strategy: RecoveryStrategy) -> Self {
        Self {
            circuit_breaker: Arc::new(CircuitBreaker::new(strategy.clone())),
            error_history: RwLock::new(Vec::new()),
            retry_counts: RwLock::new(HashMap::new()),
            strategy,
            total_errors: AtomicU64::new(0),
            recovery_attempts: AtomicU64::new(0),
        }
    }

    pub async fn handle_error(&self, error_event: ErrorEvent) -> RecoveryAction {
        self.total_errors.fetch_add(1, Ordering::SeqCst);

        // Record error in history
        {
            let mut history = self.error_history.write().await;
            history.push(error_event.clone());

            // Keep only recent errors (last 1000)
            if history.len() > 1000 {
                history.drain(0..500);
            }
        }

        // Determine recovery action based on error severity and context
        let action = self.determine_recovery_action(&error_event).await;

        debug!(
            "Error recovery: {:?} -> {:?} for file {:?}",
            error_event.severity, action, error_event.file_path
        );

        action
    }

    async fn determine_recovery_action(&self, error_event: &ErrorEvent) -> RecoveryAction {
        match error_event.severity {
            ErrorSeverity::Low => {
                if let Some(file_path) = &error_event.file_path {
                    let retry_count = self.get_retry_count(file_path).await;
                    if retry_count < self.strategy.max_retries {
                        self.increment_retry_count(file_path).await;
                        RecoveryAction::RetryWithBackoff
                    } else {
                        warn!("Max retries exceeded for {:?}, skipping", file_path);
                        RecoveryAction::SkipFile
                    }
                } else {
                    RecoveryAction::Retry
                }
            }
            ErrorSeverity::Medium => {
                if self.should_clear_cache(error_event).await {
                    RecoveryAction::ClearCache
                } else {
                    RecoveryAction::Fallback
                }
            }
            ErrorSeverity::High => {
                if self.circuit_breaker.is_open().await {
                    RecoveryAction::FullReprocessing
                } else {
                    RecoveryAction::Fallback
                }
            }
            ErrorSeverity::Critical => RecoveryAction::Shutdown,
        }
    }

    async fn should_clear_cache(&self, error_event: &ErrorEvent) -> bool {
        // Clear cache if we see cache corruption indicators
        error_event.error.contains("deserialize")
            || error_event.error.contains("corrupt")
            || error_event.error.contains("integrity")
    }

    pub async fn execute_with_recovery<F, T, E>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>
            + Send
            + Sync,
        E: std::fmt::Display + Send + Sync + 'static,
        T: Send,
    {
        self.recovery_attempts.fetch_add(1, Ordering::SeqCst);

        let mut delay = self.strategy.initial_delay;

        for attempt in 0..self.strategy.max_retries {
            // Call operation directly to avoid circuit breaker type issues for now
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    let error_event = ErrorEvent {
                        error: error.to_string(),
                        file_path: None,
                        severity: self.classify_error(&error.to_string()),
                        timestamp: Instant::now(),
                        context: HashMap::new(),
                    };

                    let action = self.handle_error(error_event).await;

                    match action {
                        RecoveryAction::Retry => continue,
                        RecoveryAction::RetryWithBackoff => {
                            if attempt < self.strategy.max_retries - 1 {
                                info!("Retrying with backoff delay: {:?}", delay);
                                tokio::time::sleep(delay).await;
                                delay = std::cmp::min(
                                    Duration::from_millis(
                                        (delay.as_millis() as f64
                                            * self.strategy.backoff_multiplier)
                                            as u64,
                                    ),
                                    self.strategy.max_delay,
                                );
                                continue;
                            }
                        }
                        RecoveryAction::Fallback => {
                            return Err(anyhow!(
                                "Fallback recovery not implemented for this operation"
                            ));
                        }
                        _ => {
                            return Err(anyhow!(
                                "Recovery action {:?} requires external handling",
                                action
                            ));
                        }
                    }
                }
            }
        }

        Err(anyhow!("All recovery attempts exhausted"))
    }

    fn classify_error(&self, error_message: &str) -> ErrorSeverity {
        let error_lower = error_message.to_lowercase();

        if error_lower.contains("no space left")
            || error_lower.contains("permission denied")
            || error_lower.contains("disk full")
        {
            ErrorSeverity::Critical
        } else if error_lower.contains("corrupt")
            || error_lower.contains("integrity")
            || error_lower.contains("deserialize")
        {
            ErrorSeverity::High
        } else if error_lower.contains("timeout")
            || error_lower.contains("connection")
            || error_lower.contains("network")
        {
            ErrorSeverity::Medium
        } else {
            ErrorSeverity::Low
        }
    }

    async fn get_retry_count(&self, file_path: &PathBuf) -> usize {
        let retry_counts = self.retry_counts.read().await;
        *retry_counts.get(file_path).unwrap_or(&0)
    }

    async fn increment_retry_count(&self, file_path: &PathBuf) {
        let mut retry_counts = self.retry_counts.write().await;
        *retry_counts.entry(file_path.clone()).or_insert(0) += 1;
    }

    pub async fn reset_retry_count(&self, file_path: &PathBuf) {
        let mut retry_counts = self.retry_counts.write().await;
        retry_counts.remove(file_path);
    }

    pub async fn get_error_statistics(&self) -> ErrorStatistics {
        let history = self.error_history.read().await;
        let recent_errors = history
            .iter()
            .filter(|e| e.timestamp.elapsed() < Duration::from_secs(3600))
            .count();

        let error_rate = if recent_errors > 0 {
            recent_errors as f64 / 3600.0 // errors per second
        } else {
            0.0
        };

        ErrorStatistics {
            total_errors: self.total_errors.load(Ordering::SeqCst),
            recovery_attempts: self.recovery_attempts.load(Ordering::SeqCst),
            recent_error_rate: error_rate,
            circuit_breaker_open: self.circuit_breaker.is_open().await,
            active_retry_files: self.retry_counts.read().await.len(),
        }
    }

    pub async fn cleanup_old_retry_counts(&self) {
        let mut retry_counts = self.retry_counts.write().await;
        // Remove retry counts for files that haven't failed recently
        // This is a simple cleanup - in practice, you might want more sophisticated logic
        if retry_counts.len() > 1000 {
            retry_counts.clear();
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorStatistics {
    pub total_errors: u64,
    pub recovery_attempts: u64,
    pub recent_error_rate: f64,
    pub circuit_breaker_open: bool,
    pub active_retry_files: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_basic() {
        let strategy = RecoveryStrategy {
            circuit_breaker_threshold: 2,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(strategy);

        // First failure
        let result = cb
            .call(async { Err::<(), String>("error".to_string()) })
            .await;
        assert!(result.is_err());
        assert!(!cb.is_open().await);

        // Second failure - should open circuit
        let result = cb
            .call(async { Err::<(), String>("error".to_string()) })
            .await;
        assert!(result.is_err());
        assert!(cb.is_open().await);

        // Third call should fail fast
        let result = cb.call(async { Ok::<(), String>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_error_recovery_system() {
        let strategy = RecoveryStrategy::default();
        let recovery_system = ErrorRecoverySystem::new(strategy);

        let error_event = ErrorEvent {
            error: "test error".to_string(),
            file_path: Some(PathBuf::from("/test/file.rs")),
            severity: ErrorSeverity::Low,
            timestamp: Instant::now(),
            context: HashMap::new(),
        };

        let action = recovery_system.handle_error(error_event).await;
        assert_eq!(action, RecoveryAction::RetryWithBackoff);

        let stats = recovery_system.get_error_statistics().await;
        assert_eq!(stats.total_errors, 1);
    }
}
