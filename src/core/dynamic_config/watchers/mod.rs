//! Configuration file watching and auto-reload functionality

pub mod file_watcher;

pub use file_watcher::FileWatcher;

use super::types::ConfigChange;
use crate::core::errors::ConfigError;
use async_trait::async_trait;
use tokio::sync::broadcast;

/// Trait for configuration watchers
#[async_trait]
pub trait ConfigWatcher {
    /// Start watching for configuration changes
    async fn start_watching(&self) -> Result<(), ConfigError>;
    
    /// Stop watching for configuration changes
    async fn stop_watching(&self) -> Result<(), ConfigError>;
    
    /// Get the change notification receiver
    fn get_change_receiver(&self) -> broadcast::Receiver<ConfigChange>;
    
    /// Check if the watcher is currently active
    fn is_watching(&self) -> bool;
    
    /// Get the watcher type name for debugging
    fn watcher_type(&self) -> &'static str;
}

/// Configuration change notifier
#[derive(Clone)]
pub struct ConfigChangeNotifier {
    sender: broadcast::Sender<ConfigChange>,
}

impl ConfigChangeNotifier {
    /// Create a new change notifier
    pub fn new(capacity: usize) -> (Self, broadcast::Receiver<ConfigChange>) {
        let (sender, receiver) = broadcast::channel(capacity);
        (Self { sender }, receiver)
    }
    
    /// Notify about a configuration change
    pub fn notify(&self, change: ConfigChange) -> Result<usize, broadcast::error::SendError<ConfigChange>> {
        self.sender.send(change)
    }
    
    /// Get a new receiver for change notifications
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigChange> {
        self.sender.subscribe()
    }
    
    /// Get the number of active receivers
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}