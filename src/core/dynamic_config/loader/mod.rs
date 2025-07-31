//! Configuration loading and saving functionality

pub mod file_loader;
pub mod env_loader;

pub use file_loader::FileLoader;
pub use env_loader::EnvLoader;

use super::types::DynamicConfig;
use crate::core::errors::ConfigError;
use anyhow::Result;
use tracing;
use async_trait::async_trait;
use std::path::Path;

/// Trait for configuration loaders
#[async_trait]
pub trait ConfigLoader {
    /// Load configuration from the source
    async fn load(&self) -> Result<DynamicConfig, ConfigError>;
    
    /// Save configuration to the source
    async fn save(&self, config: &DynamicConfig) -> Result<(), ConfigError>;
    
    /// Check if the configuration source exists
    async fn exists(&self) -> bool;
    
    /// Get the loader type name for debugging
    fn loader_type(&self) -> &'static str;
}

/// Combined loader that can load from multiple sources with priority
pub struct CombinedLoader {
    loaders: Vec<Box<dyn ConfigLoader + Send + Sync>>,
}

impl CombinedLoader {
    /// Create a new combined loader
    pub fn new() -> Self {
        Self {
            loaders: Vec::new(),
        }
    }
    
    /// Add a loader with priority (first added = highest priority)
    pub fn add_loader(mut self, loader: Box<dyn ConfigLoader + Send + Sync>) -> Self {
        self.loaders.push(loader);
        self
    }
    
    /// Load configuration from the first available source
    pub async fn load(&self) -> Result<DynamicConfig, ConfigError> {
        for loader in &self.loaders {
            if loader.exists().await {
                match loader.load().await {
                    Ok(config) => return Ok(config),
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load config from {}: {}",
                            loader.loader_type(),
                            e
                        );
                        continue;
                    }
                }
            }
        }
        
        // If no loader worked, return default config
        Ok(DynamicConfig::default())
    }
    
    /// Save configuration to the primary loader (first one)
    pub async fn save(&self, config: &DynamicConfig) -> Result<(), ConfigError> {
        if let Some(primary_loader) = self.loaders.first() {
            primary_loader.save(config).await
        } else {
            Err(ConfigError::ValidationFailed {
                reason: "No loaders configured".to_string(),
            })
        }
    }
}

impl Default for CombinedLoader {
    fn default() -> Self {
        Self::new()
    }
}