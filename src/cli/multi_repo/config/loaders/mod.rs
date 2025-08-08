pub mod json_loader;
pub mod utils;

pub use json_loader::JsonConfigLoader;
pub use utils::ConfigUtils;

use anyhow::Result;
use std::path::Path;

use crate::cli::multi_repo::config::types::MultiRepoCliConfig;

/// Trait for configuration loading strategies
pub trait ConfigLoader: Send + Sync {
    /// Load configuration from a file
    fn load_from_file(&self, path: &Path) -> Result<MultiRepoCliConfig>;
    
    /// Save configuration to a file
    fn save_to_file(&self, config: &MultiRepoCliConfig, path: &Path) -> Result<()>;
    
    /// Check if the loader supports the given file extension
    fn supports_extension(&self, extension: &str) -> bool;
}

/// Configuration loader factory
pub struct ConfigLoaderFactory;

impl ConfigLoaderFactory {
    /// Create a loader for the given file path
    pub fn create_loader(path: &Path) -> Result<Box<dyn ConfigLoader>> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("json");
            
        match extension {
            "json" => Ok(Box::new(JsonConfigLoader::new())),
            _ => Err(anyhow::anyhow!("Unsupported configuration format: {}", extension)),
        }
    }
}