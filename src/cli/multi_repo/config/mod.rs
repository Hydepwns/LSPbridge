pub mod types;
pub mod loaders;
pub mod validators;
pub mod manager;

// Re-export main types and functionality
pub use types::{
    MultiRepoCliConfig, OutputFormat, SystemLimits, WorkspaceConfig, AnalysisConfig,
    TeamConfig, DiscoveryConfig, SyncMode, Priority, ValidationRules, ValidationConstraint
};
pub use loaders::{ConfigLoader, ConfigLoaderFactory, ConfigUtils};
pub use validators::{ConfigValidator, PathValidator};
pub use manager::MultiRepoConfigManager;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_config_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = MultiRepoConfigManager::new(Some(config_path));
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = MultiRepoConfigManager::new(Some(config_path.clone())).unwrap();
        
        // Save default configuration
        manager.save_configuration().await.unwrap();
        assert!(config_path.exists());

        // Load configuration
        let loaded_config = manager.load_configuration().await.unwrap();
        assert!(matches!(loaded_config.default_output_format, OutputFormat::Table));
    }

    #[test]
    fn test_default_configuration() {
        let config = MultiRepoCliConfig::default();
        
        assert!(matches!(config.default_output_format, OutputFormat::Table));
        assert!(config.auto_detect_monorepos);
        assert_eq!(config.limits.max_repositories, 1000);
        assert_eq!(config.analysis.min_impact_threshold, 0.3);
        assert!(config.aliases.contains_key("ls"));
    }

    #[test]
    fn test_configuration_validation() {
        let manager = MultiRepoConfigManager::new(None).unwrap();
        
        // Default configuration should be valid
        assert!(manager.validate_configuration().is_ok());
        
        // Test invalid configuration
        let mut invalid_manager = MultiRepoConfigManager::new(None).unwrap();
        invalid_manager.config_mut().limits.max_repositories = 0;
        assert!(invalid_manager.validate_configuration().is_err());
    }

    #[test]
    fn test_path_validation() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Valid repository path
        let validated = PathValidator::validate_repository_path(repo_path).unwrap();
        assert_eq!(validated, repo_path);

        // Invalid path (doesn't exist)
        let invalid_path = temp_dir.path().join("nonexistent");
        assert!(PathValidator::validate_repository_path(&invalid_path).is_err());
    }

    #[test]
    fn test_config_utils() {
        // Test default paths
        assert!(ConfigUtils::default_config_path().is_ok());
        assert!(ConfigUtils::default_workspace_path().is_ok());

        // Test config merging
        let base = MultiRepoCliConfig::default();
        let mut overlay = MultiRepoCliConfig::default();
        overlay.auto_detect_monorepos = false;

        let merged = ConfigUtils::merge_configs(base, overlay);
        assert!(!merged.auto_detect_monorepos);
    }

    #[test]
    fn test_setting_update() {
        let mut manager = MultiRepoConfigManager::new(None).unwrap();
        
        // Update a setting
        manager.update_setting("auto_detect_monorepos", false).unwrap();
        assert!(!manager.config().auto_detect_monorepos);

        // Try to update invalid setting
        assert!(manager.update_setting("invalid_key", "value").is_err());
    }

    #[test]
    fn test_config_validator() {
        let validator = ConfigValidator::new();
        let config = MultiRepoCliConfig::default();
        
        // Valid configuration
        assert!(validator.validate(&config).is_ok());
        
        // Test individual setting validation
        let value = serde_json::Value::Number(serde_json::Number::from(500));
        assert!(validator.validate_setting("max_repositories", &value).is_ok());
        
        let invalid_value = serde_json::Value::Number(serde_json::Number::from(0));
        assert!(validator.validate_setting("max_repositories", &invalid_value).is_err());
    }
}