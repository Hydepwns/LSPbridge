use crate::cli::multi_repo::config::types::{MultiRepoCliConfig, ValidationRules};
use crate::security::validate_path;
use anyhow::{Context, Result};

/// Configuration validator
pub struct ConfigValidator {
    validation_rules: ValidationRules,
}

impl ConfigValidator {
    pub fn new() -> Self {
        Self {
            validation_rules: ValidationRules::default(),
        }
    }

    pub fn with_rules(validation_rules: ValidationRules) -> Self {
        Self { validation_rules }
    }

    /// Validate a configuration
    pub fn validate(&self, config: &MultiRepoCliConfig) -> Result<()> {
        // Validate limits
        if config.limits.max_repositories == 0 {
            return Err(anyhow::anyhow!("max_repositories must be greater than 0"));
        }

        if config.limits.max_analysis_depth == 0 {
            return Err(anyhow::anyhow!("max_analysis_depth must be greater than 0"));
        }

        if config.limits.max_file_size_mb > 1000 {
            return Err(anyhow::anyhow!("max_file_size_mb cannot exceed 1000MB"));
        }

        // Validate workspace configuration
        if let Some(workspace_root) = &config.workspace.default_root {
            validate_path(workspace_root)
                .context("Invalid workspace root path")?;
        }

        // Validate analysis configuration
        if config.analysis.min_impact_threshold < 0.0 || config.analysis.min_impact_threshold > 1.0 {
            return Err(anyhow::anyhow!("min_impact_threshold must be between 0.0 and 1.0"));
        }

        // Validate team configuration
        if config.team.max_assignments_per_member == 0 {
            return Err(anyhow::anyhow!("max_assignments_per_member must be greater than 0"));
        }

        Ok(())
    }

    /// Validate a specific configuration key-value pair
    pub fn validate_setting(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        if let Some(constraint) = self.validation_rules.constraints.get(key) {
            match constraint {
                crate::cli::multi_repo::config::types::ValidationConstraint::Range { min, max } => {
                    if let Some(num) = value.as_u64() {
                        let num = num as usize;
                        if num < *min || num > *max {
                            return Err(anyhow::anyhow!(
                                "Value for {} must be between {} and {}, got {}",
                                key, min, max, num
                            ));
                        }
                    } else {
                        return Err(anyhow::anyhow!("Value for {} must be a number", key));
                    }
                }
                crate::cli::multi_repo::config::types::ValidationConstraint::FloatRange { min, max } => {
                    if let Some(num) = value.as_f64() {
                        let num = num as f32;
                        if num < *min || num > *max {
                            return Err(anyhow::anyhow!(
                                "Value for {} must be between {} and {}, got {}",
                                key, min, max, num
                            ));
                        }
                    } else {
                        return Err(anyhow::anyhow!("Value for {} must be a number", key));
                    }
                }
                crate::cli::multi_repo::config::types::ValidationConstraint::StringLength { min, max } => {
                    if let Some(s) = value.as_str() {
                        if s.len() < *min || s.len() > *max {
                            return Err(anyhow::anyhow!(
                                "String length for {} must be between {} and {}, got {}",
                                key, min, max, s.len()
                            ));
                        }
                    } else {
                        return Err(anyhow::anyhow!("Value for {} must be a string", key));
                    }
                }
                crate::cli::multi_repo::config::types::ValidationConstraint::PathExists => {
                    if let Some(path_str) = value.as_str() {
                        let path = std::path::Path::new(path_str);
                        if !path.exists() {
                            return Err(anyhow::anyhow!("Path {} does not exist", path_str));
                        }
                    } else {
                        return Err(anyhow::anyhow!("Value for {} must be a path string", key));
                    }
                }
                crate::cli::multi_repo::config::types::ValidationConstraint::OneOf(options) => {
                    if let Some(s) = value.as_str() {
                        if !options.contains(&s.to_string()) {
                            return Err(anyhow::anyhow!(
                                "Value for {} must be one of {:?}, got {}",
                                key, options, s
                            ));
                        }
                    } else {
                        return Err(anyhow::anyhow!("Value for {} must be a string", key));
                    }
                }
            }
        }

        Ok(())
    }

    /// Get validation rules
    pub fn rules(&self) -> &ValidationRules {
        &self.validation_rules
    }
}