use super::ConfigLoader;
use crate::cli::multi_repo::config::types::MultiRepoCliConfig;
use anyhow::{Context, Result};
use std::path::Path;
use std::fs;

/// JSON configuration loader
pub struct JsonConfigLoader;

impl Default for JsonConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonConfigLoader {
    pub fn new() -> Self {
        Self
    }
}

impl ConfigLoader for JsonConfigLoader {
    fn load_from_file(&self, path: &Path) -> Result<MultiRepoCliConfig> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration file: {}", path.display()))?;
        
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON configuration: {}", path.display()))
    }
    
    fn save_to_file(&self, config: &MultiRepoCliConfig, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create configuration directory")?;
        }

        let json = serde_json::to_string_pretty(config)
            .context("Failed to serialize configuration")?;
        
        fs::write(path, json)
            .with_context(|| format!("Failed to write configuration file: {}", path.display()))
    }
    
    fn supports_extension(&self, extension: &str) -> bool {
        extension == "json"
    }
}