use anyhow::Result;
use prometheus::{Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone)]
pub struct ProcessingMetrics {
    pub cache_hit_rate: f64,
    pub avg_processing_time: Duration,
    pub cache_size_bytes: usize,
    pub files_processed: usize,
    pub errors_per_hour: f64,
    pub memory_pressure: f64,
    pub throughput_files_per_second: f64,
    pub parallel_efficiency: f64,
}

impl Default for ProcessingMetrics {
    fn default() -> Self {
        Self {
            cache_hit_rate: 0.0,
            avg_processing_time: Duration::ZERO,
            cache_size_bytes: 0,
            files_processed: 0,
            errors_per_hour: 0.0,
            memory_pressure: 0.0,
            throughput_files_per_second: 0.0,
            parallel_efficiency: 0.0,
        }
    }
}

pub struct MetricsCollector {
    registry: Registry,

    // Counters
    files_processed_total: IntCounter,
    cache_hits_total: IntCounter,
    cache_misses_total: IntCounter,
    errors_total: IntCounter,
    recovery_attempts_total: IntCounter,

    // Gauges
    cache_size_bytes: IntGauge,
    cache_entries: IntGauge,
    active_parallel_tasks: IntGauge,
    memory_usage_bytes: IntGauge,

    // Histograms
    file_processing_duration: Histogram,
    cache_operation_duration: Histogram,
    hash_computation_duration: Histogram,

    // Custom metrics tracking
    processing_history: RwLock<Vec<ProcessingRecord>>,
    start_time: Instant,
}

#[derive(Debug, Clone)]
struct ProcessingRecord {
    timestamp: Instant,
    files_processed: usize,
    cache_hits: usize,
    cache_misses: usize,
    processing_duration: Duration,
    parallel_tasks: usize,
}

impl MetricsCollector {
    pub fn new() -> Result<Self> {
        let registry = Registry::new();

        // Create counters
        let files_processed_total = IntCounter::with_opts(Opts::new(
            "lsp_bridge_files_processed_total",
            "Total number of files processed",
        ))?;

        let cache_hits_total = IntCounter::with_opts(Opts::new(
            "lsp_bridge_cache_hits_total",
            "Total number of cache hits",
        ))?;

        let cache_misses_total = IntCounter::with_opts(Opts::new(
            "lsp_bridge_cache_misses_total",
            "Total number of cache misses",
        ))?;

        let errors_total = IntCounter::with_opts(Opts::new(
            "lsp_bridge_errors_total",
            "Total number of errors encountered",
        ))?;

        let recovery_attempts_total = IntCounter::with_opts(Opts::new(
            "lsp_bridge_recovery_attempts_total",
            "Total number of error recovery attempts",
        ))?;

        // Create gauges
        let cache_size_bytes = IntGauge::with_opts(Opts::new(
            "lsp_bridge_cache_size_bytes",
            "Current cache size in bytes",
        ))?;

        let cache_entries = IntGauge::with_opts(Opts::new(
            "lsp_bridge_cache_entries",
            "Current number of cache entries",
        ))?;

        let active_parallel_tasks = IntGauge::with_opts(Opts::new(
            "lsp_bridge_active_parallel_tasks",
            "Number of currently active parallel tasks",
        ))?;

        let memory_usage_bytes = IntGauge::with_opts(Opts::new(
            "lsp_bridge_memory_usage_bytes",
            "Current memory usage in bytes",
        ))?;

        // Create histograms
        let file_processing_duration = Histogram::with_opts(
            HistogramOpts::new(
                "lsp_bridge_file_processing_duration_seconds",
                "Time spent processing individual files",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
        )?;

        let cache_operation_duration = Histogram::with_opts(
            HistogramOpts::new(
                "lsp_bridge_cache_operation_duration_seconds",
                "Time spent on cache operations",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]),
        )?;

        let hash_computation_duration = Histogram::with_opts(
            HistogramOpts::new(
                "lsp_bridge_hash_computation_duration_seconds",
                "Time spent computing file hashes",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]),
        )?;

        // Register all metrics
        registry.register(Box::new(files_processed_total.clone()))?;
        registry.register(Box::new(cache_hits_total.clone()))?;
        registry.register(Box::new(cache_misses_total.clone()))?;
        registry.register(Box::new(errors_total.clone()))?;
        registry.register(Box::new(recovery_attempts_total.clone()))?;
        registry.register(Box::new(cache_size_bytes.clone()))?;
        registry.register(Box::new(cache_entries.clone()))?;
        registry.register(Box::new(active_parallel_tasks.clone()))?;
        registry.register(Box::new(memory_usage_bytes.clone()))?;
        registry.register(Box::new(file_processing_duration.clone()))?;
        registry.register(Box::new(cache_operation_duration.clone()))?;
        registry.register(Box::new(hash_computation_duration.clone()))?;

        Ok(Self {
            registry,
            files_processed_total,
            cache_hits_total,
            cache_misses_total,
            errors_total,
            recovery_attempts_total,
            cache_size_bytes,
            cache_entries,
            active_parallel_tasks,
            memory_usage_bytes,
            file_processing_duration,
            cache_operation_duration,
            hash_computation_duration,
            processing_history: RwLock::new(Vec::new()),
            start_time: Instant::now(),
        })
    }

    // Counter methods
    pub fn increment_files_processed(&self, count: u64) {
        self.files_processed_total.inc_by(count);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits_total.inc();
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses_total.inc();
    }

    pub fn record_error(&self) {
        self.errors_total.inc();
    }

    pub fn record_recovery_attempt(&self) {
        self.recovery_attempts_total.inc();
    }

    // Gauge methods
    pub fn set_cache_size(&self, size_bytes: i64) {
        self.cache_size_bytes.set(size_bytes);
    }

    pub fn set_cache_entries(&self, count: i64) {
        self.cache_entries.set(count);
    }

    pub fn set_active_parallel_tasks(&self, count: i64) {
        self.active_parallel_tasks.set(count);
    }

    pub fn set_memory_usage(&self, bytes: i64) {
        self.memory_usage_bytes.set(bytes);
    }

    // Histogram methods
    pub fn record_file_processing_time(&self, duration: Duration) {
        self.file_processing_duration
            .observe(duration.as_secs_f64());
    }

    pub fn record_cache_operation_time(&self, duration: Duration) {
        self.cache_operation_duration
            .observe(duration.as_secs_f64());
    }

    pub fn record_hash_computation_time(&self, duration: Duration) {
        self.hash_computation_duration
            .observe(duration.as_secs_f64());
    }

    // Helper methods for common operations
    pub async fn time_operation<F, T>(&self, operation: F) -> (T, Duration)
    where
        F: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = operation.await;
        let duration = start.elapsed();
        (result, duration)
    }

    pub async fn time_file_processing<F, T>(&self, operation: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let (result, duration) = self.time_operation(operation).await;
        self.record_file_processing_time(duration);
        result
    }

    pub async fn time_cache_operation<F, T>(&self, operation: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let (result, duration) = self.time_operation(operation).await;
        self.record_cache_operation_time(duration);
        result
    }

    pub async fn time_hash_computation<F, T>(&self, operation: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let (result, duration) = self.time_operation(operation).await;
        self.record_hash_computation_time(duration);
        result
    }

    // Complex metrics calculation
    pub async fn record_processing_session(
        &self,
        files_processed: usize,
        cache_hits: usize,
        cache_misses: usize,
        duration: Duration,
        parallel_tasks: usize,
    ) {
        let record = ProcessingRecord {
            timestamp: Instant::now(),
            files_processed,
            cache_hits,
            cache_misses,
            processing_duration: duration,
            parallel_tasks,
        };

        let mut history = self.processing_history.write().await;
        history.push(record);

        // Keep only last 1000 records
        if history.len() > 1000 {
            history.drain(0..500);
        }
    }

    pub async fn get_comprehensive_metrics(&self) -> ProcessingMetrics {
        let history = self.processing_history.read().await;

        if history.is_empty() {
            return ProcessingMetrics::default();
        }

        // Calculate metrics from recent history (last hour)
        let recent_cutoff = Instant::now() - Duration::from_secs(3600);
        let recent_records: Vec<_> = history
            .iter()
            .filter(|r| r.timestamp >= recent_cutoff)
            .collect();

        let cache_hit_rate = if recent_records.is_empty() {
            0.0
        } else {
            let total_hits: usize = recent_records.iter().map(|r| r.cache_hits).sum();
            let total_misses: usize = recent_records.iter().map(|r| r.cache_misses).sum();
            let total_requests = total_hits + total_misses;
            if total_requests > 0 {
                total_hits as f64 / total_requests as f64
            } else {
                0.0
            }
        };

        let avg_processing_time = if recent_records.is_empty() {
            Duration::ZERO
        } else {
            let total_duration: Duration =
                recent_records.iter().map(|r| r.processing_duration).sum();
            total_duration / recent_records.len() as u32
        };

        let files_processed: usize = recent_records.iter().map(|r| r.files_processed).sum();

        let throughput_files_per_second = if recent_records.is_empty() {
            0.0
        } else {
            let time_span = recent_records
                .last()
                .unwrap()
                .timestamp
                .duration_since(recent_records.first().unwrap().timestamp);
            if time_span.as_secs() > 0 {
                files_processed as f64 / time_span.as_secs_f64()
            } else {
                0.0
            }
        };

        let parallel_efficiency = if recent_records.is_empty() {
            0.0
        } else {
            let avg_parallel_tasks: f64 = recent_records
                .iter()
                .map(|r| r.parallel_tasks as f64)
                .sum::<f64>()
                / recent_records.len() as f64;

            // Efficiency = actual throughput / (ideal single-threaded throughput * parallel tasks)
            // This is a simplified calculation
            if avg_parallel_tasks > 1.0 {
                throughput_files_per_second / avg_parallel_tasks
            } else {
                1.0
            }
        };

        // Calculate error rate (errors per hour)
        let errors_per_hour = {
            let errors_count = self.errors_total.get() as f64;
            let uptime_hours = self.start_time.elapsed().as_secs_f64() / 3600.0;
            if uptime_hours > 0.0 {
                errors_count / uptime_hours
            } else {
                0.0
            }
        };

        // Memory pressure (simplified)
        let memory_pressure = {
            let current_memory = self.memory_usage_bytes.get() as f64;
            // Assume 1GB as reasonable upper limit for this application
            let max_reasonable_memory = 1024.0 * 1024.0 * 1024.0;
            (current_memory / max_reasonable_memory).min(1.0)
        };

        ProcessingMetrics {
            cache_hit_rate,
            avg_processing_time,
            cache_size_bytes: self.cache_size_bytes.get() as usize,
            files_processed,
            errors_per_hour,
            memory_pressure,
            throughput_files_per_second,
            parallel_efficiency,
        }
    }

    pub fn export_prometheus_metrics(&self) -> String {
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder
            .encode_to_string(&metric_families)
            .unwrap_or_else(|e| format!("Error encoding metrics: {}", e))
    }

    pub async fn get_performance_summary(&self) -> PerformanceSummary {
        let metrics = self.get_comprehensive_metrics().await;
        let uptime = self.start_time.elapsed();

        PerformanceSummary {
            uptime,
            total_files_processed: self.files_processed_total.get(),
            cache_hit_rate: metrics.cache_hit_rate,
            average_processing_time: metrics.avg_processing_time,
            current_throughput: metrics.throughput_files_per_second,
            error_rate: metrics.errors_per_hour,
            memory_efficiency: 1.0 - metrics.memory_pressure,
            parallel_efficiency: metrics.parallel_efficiency,
        }
    }

    pub async fn log_performance_summary(&self) {
        let summary = self.get_performance_summary().await;

        info!(
            "Performance Summary - Uptime: {:.2}h, Files: {}, Cache Hit Rate: {:.1}%, Throughput: {:.1} files/s, Error Rate: {:.2}/h",
            summary.uptime.as_secs_f64() / 3600.0,
            summary.total_files_processed,
            summary.cache_hit_rate * 100.0,
            summary.current_throughput,
            summary.error_rate
        );
    }

    // Health check metrics
    pub async fn health_check(&self) -> HealthStatus {
        let metrics = self.get_comprehensive_metrics().await;

        let mut issues = Vec::new();
        let mut score: f32 = 100.0;

        // Check cache hit rate
        if metrics.cache_hit_rate < 0.8 {
            issues.push("Low cache hit rate".to_string());
            score -= 20.0;
        }

        // Check error rate
        if metrics.errors_per_hour > 10.0 {
            issues.push("High error rate".to_string());
            score -= 30.0;
        }

        // Check memory pressure
        if metrics.memory_pressure > 0.8 {
            issues.push("High memory pressure".to_string());
            score -= 25.0;
        }

        // Check processing time
        if metrics.avg_processing_time > Duration::from_secs(5) {
            issues.push("Slow processing times".to_string());
            score -= 15.0;
        }

        let status = if score >= 90.0 {
            "healthy"
        } else if score >= 70.0 {
            "degraded"
        } else if score >= 50.0 {
            "unhealthy"
        } else {
            "critical"
        };

        HealthStatus {
            status: status.to_string(),
            score: score.max(0.0) as f64,
            issues,
            metrics,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub uptime: Duration,
    pub total_files_processed: u64,
    pub cache_hit_rate: f64,
    pub average_processing_time: Duration,
    pub current_throughput: f64,
    pub error_rate: f64,
    pub memory_efficiency: f64,
    pub parallel_efficiency: f64,
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub status: String,
    pub score: f64,
    pub issues: Vec<String>,
    pub metrics: ProcessingMetrics,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics collector")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_metrics_collection() {
        let metrics = MetricsCollector::new().unwrap();

        // Record some metrics
        metrics.increment_files_processed(10);
        metrics.record_cache_hit();
        metrics.record_cache_miss();
        metrics.record_file_processing_time(Duration::from_millis(100));

        // Test comprehensive metrics
        let comprehensive = metrics.get_comprehensive_metrics().await;
        assert_eq!(comprehensive.files_processed, 0); // No sessions recorded yet

        // Record a processing session
        metrics
            .record_processing_session(10, 8, 2, Duration::from_millis(1000), 4)
            .await;

        let comprehensive = metrics.get_comprehensive_metrics().await;
        assert_eq!(comprehensive.files_processed, 10);
        assert_eq!(comprehensive.cache_hit_rate, 0.8);
    }

    #[tokio::test]
    async fn test_health_check() {
        let metrics = MetricsCollector::new().unwrap();

        // Record good metrics
        metrics
            .record_processing_session(100, 95, 5, Duration::from_millis(500), 4)
            .await;

        let health = metrics.health_check().await;
        assert_eq!(health.status, "healthy");
        assert!(health.score >= 90.0);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = MetricsCollector::new().unwrap();

        metrics.increment_files_processed(5);
        metrics.record_cache_hit();

        let exported = metrics.export_prometheus_metrics();
        assert!(exported.contains("lsp_bridge_files_processed_total"));
        assert!(exported.contains("lsp_bridge_cache_hits_total"));
    }
}
