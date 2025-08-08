use super::loaders::{ConfigLoader, ConfigLoaderFactory};
use super::types::{MultiRepoCliConfig, ValidationRules};
use super::validators::ConfigValidator;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::PathBuf;

/// Multi-repository CLI configuration manager
pub struct MultiRepoConfigManager {
    /// Configuration file path
    config_path: Option<PathBuf>,
    
    /// Loaded configuration
    config: MultiRepoCliConfig,
    
    /// Configuration validator
    validator: ConfigValidator,
    
    /// Configuration loader
    loader: Box<dyn ConfigLoader>,
}

impl MultiRepoConfigManager {
    /// Create a new configuration manager
    pub fn new(config_path: Option<PathBuf>) -> Result<Self> {
        let loader = if let Some(ref path) = config_path {
            ConfigLoaderFactory::create_loader(path)?
        } else {
            ConfigLoaderFactory::create_loader(&PathBuf::from("config.json"))?
        };

        Ok(Self {
            config_path,
            config: MultiRepoCliConfig::default(),
            validator: ConfigValidator::new(),
            loader,
        })
    }

    /// Create a configuration manager with custom validation rules
    pub fn with_validation_rules(
        config_path: Option<PathBuf>,
        validation_rules: ValidationRules,
    ) -> Result<Self> {
        let loader = if let Some(ref path) = config_path {
            ConfigLoaderFactory::create_loader(path)?
        } else {
            ConfigLoaderFactory::create_loader(&PathBuf::from("config.json"))?
        };

        Ok(Self {
            config_path,
            config: MultiRepoCliConfig::default(),
            validator: ConfigValidator::with_rules(validation_rules),
            loader,
        })
    }

    /// Load configuration from file or create default
    pub async fn load_configuration(&mut self) -> Result<&MultiRepoCliConfig> {
        if let Some(path) = &self.config_path {
            if path.exists() {
                self.config = self.loader.load_from_file(path)?;
                
                // Validate loaded configuration
                self.validate_configuration()?;
            } else {
                // Create default configuration file
                self.save_configuration().await?;
            }
        }

        Ok(&self.config)
    }

    /// Save current configuration to file
    pub async fn save_configuration(&self) -> Result<()> {
        if let Some(path) = &self.config_path {
            self.loader.save_to_file(&self.config, path)?;
        }

        Ok(())
    }

    /// Update configuration settings
    pub fn update_setting<T: Serialize>(&mut self, key: &str, value: T) -> Result<()> {
        let value_json = serde_json::to_value(value)
            .context("Failed to serialize configuration value")?;

        // Validate the setting first
        self.validator.validate_setting(key, &value_json)?;

        match key {
            "default_output_format" => {
                self.config.default_output_format = serde_json::from_value(value_json)?;
            }
            "auto_detect_monorepos" => {
                self.config.auto_detect_monorepos = serde_json::from_value(value_json)?;
            }
            "max_repositories" => {
                self.config.limits.max_repositories = serde_json::from_value(value_json)?;
            }
            "max_analysis_depth" => {
                self.config.limits.max_analysis_depth = serde_json::from_value(value_json)?;
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
            }
        }

        self.validate_configuration()?;
        Ok(())
    }

    /// Get configuration setting
    pub fn get_setting(&self, key: &str) -> Result<serde_json::Value> {
        let value = match key {
            "default_output_format" => serde_json::to_value(&self.config.default_output_format)?,
            "auto_detect_monorepos" => serde_json::to_value(self.config.auto_detect_monorepos)?,
            "max_repositories" => serde_json::to_value(self.config.limits.max_repositories)?,
            "max_analysis_depth" => serde_json::to_value(self.config.limits.max_analysis_depth)?,
            _ => {
                return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
            }
        };

        Ok(value)
    }

    /// Validate current configuration
    pub fn validate_configuration(&self) -> Result<()> {
        self.validator.validate(&self.config)
    }

    /// Reset configuration to defaults
    pub fn reset_to_defaults(&mut self) {
        self.config = MultiRepoCliConfig::default();
    }

    /// Get configuration reference
    pub fn config(&self) -> &MultiRepoCliConfig {
        &self.config
    }

    /// Get mutable configuration reference
    pub fn config_mut(&mut self) -> &mut MultiRepoCliConfig {
        &mut self.config
    }
}