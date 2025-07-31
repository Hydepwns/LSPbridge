//! Memory monitoring and alerting
//!
//! This module provides memory usage monitoring, threshold alerts,
//! and performance tracking for the memory management system.

use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time;
use tracing::{error, info, warn};

use super::types::{CacheStatistics, MemoryHealthStatus, MemoryReport};

/// Memory monitoring events
#[derive(Debug, Clone)]
pub enum MemoryEvent {
    /// Memory usage crossed a threshold
    ThresholdCrossed {
        threshold: MemoryThreshold,
        current_usage: f64,
        timestamp: Instant,
    },
    
    /// Cache performance degraded
    PerformanceDegraded {
        hit_rate: f64,
        eviction_rate: f64,
        timestamp: Instant,
    },
    
    /// Memory optimized
    MemoryOptimized {
        before_bytes: usize,
        after_bytes: usize,
        freed_bytes: usize,
        timestamp: Instant,
    },
    
    /// Periodic health check
    HealthCheck {
        report: MemoryReport,
        timestamp: Instant,
    },
}

/// Memory usage thresholds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryThreshold {
    /// 50% usage
    Normal,
    /// 70% usage
    Warning,
    /// 85% usage
    High,
    /// 95% usage
    Critical,
}

impl MemoryThreshold {
    /// Get the threshold value
    pub fn value(&self) -> f64 {
        match self {
            Self::Normal => 0.5,
            Self::Warning => 0.7,
            Self::High => 0.85,
            Self::Critical => 0.95,
        }
    }

    /// Get threshold from usage value
    pub fn from_usage(usage: f64) -> Option<Self> {
        if usage >= Self::Critical.value() {
            Some(Self::Critical)
        } else if usage >= Self::High.value() {
            Some(Self::High)
        } else if usage >= Self::Warning.value() {
            Some(Self::Warning)
        } else if usage >= Self::Normal.value() {
            Some(Self::Normal)
        } else {
            None
        }
    }
}

/// Memory monitor configuration
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Monitoring interval
    pub check_interval: Duration,
    
    /// Enable threshold alerts
    pub enable_alerts: bool,
    
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    
    /// Minimum hit rate before alerting
    pub min_hit_rate: f64,
    
    /// Maximum eviction rate before alerting
    pub max_eviction_rate: f64,
    
    /// Enable automatic optimization
    pub auto_optimize: bool,
    
    /// Optimization threshold
    pub optimization_threshold: f64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            enable_alerts: true,
            enable_performance_monitoring: true,
            min_hit_rate: 0.6,
            max_eviction_rate: 0.2,
            auto_optimize: true,
            optimization_threshold: 0.8,
        }
    }
}

/// Memory usage monitor
pub struct MemoryMonitor {
    config: MonitorConfig,
    event_sender: mpsc::Sender<MemoryEvent>,
    event_receiver: Arc<RwLock<mpsc::Receiver<MemoryEvent>>>,
    last_threshold: RwLock<Option<MemoryThreshold>>,
    monitoring_task: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl MemoryMonitor {
    /// Create a new memory monitor
    pub fn new(config: MonitorConfig) -> Self {
        let (sender, receiver) = mpsc::channel(100);
        
        Self {
            config,
            event_sender: sender,
            event_receiver: Arc::new(RwLock::new(receiver)),
            last_threshold: RwLock::new(None),
            monitoring_task: RwLock::new(None),
        }
    }

    /// Start monitoring
    pub async fn start<F, Fut>(&self, get_report: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = MemoryReport> + Send,
    {
        let mut task = self.monitoring_task.write().await;
        if task.is_some() {
            return Err(anyhow::anyhow!("Monitoring already started"));
        }

        let config = self.config.clone();
        let event_sender = self.event_sender.clone();
        let last_threshold = Arc::new(self.last_threshold.clone());

        let handle = tokio::spawn(async move {
            let mut interval = time::interval(config.check_interval);
            
            loop {
                interval.tick().await;
                
                let report = get_report().await;
                let timestamp = Instant::now();

                // Send health check event
                if let Err(e) = event_sender.send(MemoryEvent::HealthCheck {
                    report: report.clone(),
                    timestamp,
                }).await {
                    error!("Failed to send health check event: {}", e);
                    break;
                }

                // Check memory thresholds
                if config.enable_alerts {
                    Self::check_thresholds(&report, &event_sender, &last_threshold).await;
                }

                // Check performance
                if config.enable_performance_monitoring {
                    Self::check_performance(&report, &config, &event_sender).await;
                }
            }
        });

        *task = Some(handle);
        Ok(())
    }

    /// Stop monitoring
    pub async fn stop(&self) -> Result<()> {
        let mut task = self.monitoring_task.write().await;
        if let Some(handle) = task.take() {
            handle.abort();
            info!("Memory monitoring stopped");
        }
        Ok(())
    }

    /// Get event receiver
    pub async fn subscribe(&self) -> mpsc::Receiver<MemoryEvent> {
        let (sender, receiver) = mpsc::channel(100);
        let mut current_receiver = self.event_receiver.write().await;
        
        // Forward events to new receiver
        tokio::spawn(async move {
            while let Some(event) = current_receiver.recv().await {
                if sender.send(event).await.is_err() {
                    break;
                }
            }
        });

        receiver
    }

    /// Check memory thresholds
    async fn check_thresholds(
        report: &MemoryReport,
        sender: &mpsc::Sender<MemoryEvent>,
        last_threshold: &Arc<RwLock<Option<MemoryThreshold>>>,
    ) {
        let usage = report.memory_utilization.max(report.entry_utilization);
        let current_threshold = MemoryThreshold::from_usage(usage);

        let mut last = last_threshold.write().await;
        
        // Only alert if threshold changed
        if current_threshold != *last {
            if let Some(threshold) = current_threshold {
                if let Err(e) = sender.send(MemoryEvent::ThresholdCrossed {
                    threshold,
                    current_usage: usage,
                    timestamp: Instant::now(),
                }).await {
                    error!("Failed to send threshold event: {}", e);
                }
                
                match threshold {
                    MemoryThreshold::Critical => {
                        error!("Memory usage critical: {:.1}%", usage * 100.0);
                    }
                    MemoryThreshold::High => {
                        warn!("Memory usage high: {:.1}%", usage * 100.0);
                    }
                    MemoryThreshold::Warning => {
                        warn!("Memory usage warning: {:.1}%", usage * 100.0);
                    }
                    MemoryThreshold::Normal => {
                        info!("Memory usage normal: {:.1}%", usage * 100.0);
                    }
                }
            }
            *last = current_threshold;
        }
    }

    /// Check cache performance
    async fn check_performance(
        report: &MemoryReport,
        config: &MonitorConfig,
        sender: &mpsc::Sender<MemoryEvent>,
    ) {
        let hit_rate = report.hit_rate;
        let eviction_rate = report.eviction_rate;

        if hit_rate < config.min_hit_rate || eviction_rate > config.max_eviction_rate {
            if let Err(e) = sender.send(MemoryEvent::PerformanceDegraded {
                hit_rate,
                eviction_rate,
                timestamp: Instant::now(),
            }).await {
                error!("Failed to send performance event: {}", e);
            }

            warn!(
                "Cache performance degraded - hit_rate: {:.1}%, eviction_rate: {:.1}%",
                hit_rate * 100.0,
                eviction_rate * 100.0
            );
        }
    }

    /// Process memory events
    pub async fn process_events<H>(&self, mut handler: H) -> Result<()>
    where
        H: FnMut(MemoryEvent) -> Result<()>,
    {
        let mut receiver = self.subscribe().await;
        
        while let Some(event) = receiver.recv().await {
            handler(event)?;
        }
        
        Ok(())
    }
}

/// Memory alert handler
pub struct AlertHandler {
    /// Alert callbacks
    callbacks: Vec<Box<dyn Fn(&MemoryEvent) + Send + Sync>>,
}

impl AlertHandler {
    /// Create a new alert handler
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    /// Add an alert callback
    pub fn add_callback<F>(&mut self, callback: F)
    where
        F: Fn(&MemoryEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }

    /// Handle a memory event
    pub fn handle_event(&self, event: &MemoryEvent) {
        for callback in &self.callbacks {
            callback(event);
        }
    }
}

impl Default for AlertHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory optimization helper
pub struct MemoryOptimizer {
    /// Optimization history
    history: RwLock<Vec<OptimizationRecord>>,
    
    /// Maximum history size
    max_history: usize,
}

#[derive(Debug, Clone)]
struct OptimizationRecord {
    timestamp: Instant,
    before_bytes: usize,
    after_bytes: usize,
    freed_bytes: usize,
    duration: Duration,
}

impl MemoryOptimizer {
    /// Create a new memory optimizer
    pub fn new() -> Self {
        Self {
            history: RwLock::new(Vec::new()),
            max_history: 100,
        }
    }

    /// Record an optimization
    pub async fn record_optimization(
        &self,
        before_bytes: usize,
        after_bytes: usize,
        duration: Duration,
    ) {
        let mut history = self.history.write().await;
        
        history.push(OptimizationRecord {
            timestamp: Instant::now(),
            before_bytes,
            after_bytes,
            freed_bytes: before_bytes.saturating_sub(after_bytes),
            duration,
        });

        // Keep history bounded
        if history.len() > self.max_history {
            history.remove(0);
        }
    }

    /// Get optimization statistics
    pub async fn get_statistics(&self) -> OptimizationStatistics {
        let history = self.history.read().await;
        
        if history.is_empty() {
            return OptimizationStatistics::default();
        }

        let total_freed = history.iter().map(|r| r.freed_bytes).sum();
        let avg_freed = total_freed / history.len();
        let avg_duration = history.iter().map(|r| r.duration.as_millis() as u64).sum::<u64>()
            / history.len() as u64;

        OptimizationStatistics {
            total_optimizations: history.len(),
            total_bytes_freed: total_freed,
            average_bytes_freed: avg_freed,
            average_duration_ms: avg_duration,
            last_optimization: history.last().map(|r| r.timestamp),
        }
    }
}

impl Default for MemoryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimization statistics
#[derive(Debug, Clone, Default)]
pub struct OptimizationStatistics {
    pub total_optimizations: usize,
    pub total_bytes_freed: usize,
    pub average_bytes_freed: usize,
    pub average_duration_ms: u64,
    pub last_optimization: Option<Instant>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_threshold() {
        assert_eq!(MemoryThreshold::from_usage(0.3), None);
        assert_eq!(MemoryThreshold::from_usage(0.5), Some(MemoryThreshold::Normal));
        assert_eq!(MemoryThreshold::from_usage(0.7), Some(MemoryThreshold::Warning));
        assert_eq!(MemoryThreshold::from_usage(0.85), Some(MemoryThreshold::High));
        assert_eq!(MemoryThreshold::from_usage(0.95), Some(MemoryThreshold::Critical));
    }

    #[tokio::test]
    async fn test_memory_optimizer() {
        let optimizer = MemoryOptimizer::new();
        
        optimizer.record_optimization(1000, 600, Duration::from_millis(50)).await;
        optimizer.record_optimization(800, 500, Duration::from_millis(40)).await;
        
        let stats = optimizer.get_statistics().await;
        assert_eq!(stats.total_optimizations, 2);
        assert_eq!(stats.total_bytes_freed, 700); // 400 + 300
        assert_eq!(stats.average_bytes_freed, 350);
        assert_eq!(stats.average_duration_ms, 45);
    }

    #[test]
    fn test_alert_handler() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        let mut handler = AlertHandler::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        handler.add_callback(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        let event = MemoryEvent::ThresholdCrossed {
            threshold: MemoryThreshold::Warning,
            current_usage: 0.75,
            timestamp: Instant::now(),
        };

        handler.handle_event(&event);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}