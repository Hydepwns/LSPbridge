//! Configuration validation for LSPbridge

use anyhow::{Context, Result};
use std::path::Path;
use std::fs;

/// Configuration validation errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigValidationError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },
    
    #[error("Invalid configuration format: {reason}")]
    InvalidFormat { reason: String },
    
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Invalid value for field '{field}': {reason}")]
    InvalidValue { field: String, reason: String },
    
    #[error("Directory not accessible: {path} - {reason}")]
    DirectoryNotAccessible { path: String, reason: String },
}

/// Validates the complete application configuration on startup
pub struct ConfigValidator {
    config_path: Option<String>,
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigValidator {
    pub fn new() -> Self {
        Self { config_path: None }
    }
    
    pub fn with_config_path(mut self, path: String) -> Self {
        self.config_path = Some(path);
        self
    }
    
    /// Perform full configuration validation
    pub fn validate(&self) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();
        
        // Check environment
        self.validate_environment(&mut report)?;
        
        // Check directories
        self.validate_directories(&mut report)?;
        
        // Check configuration file if specified
        if let Some(config_path) = &self.config_path {
            self.validate_config_file(config_path, &mut report)?;
        }
        
        // Check runtime dependencies
        self.validate_dependencies(&mut report)?;
        
        Ok(report)
    }
    
    fn validate_environment(&self, report: &mut ValidationReport) -> Result<()> {
        // Check for required environment variables
        let env_vars = [
            ("RUST_LOG", false),  // optional but recommended
            ("LSP_BRIDGE_CONFIG_DIR", false),
            ("LSP_BRIDGE_CACHE_DIR", false),
        ];
        
        for (var, required) in env_vars {
            match std::env::var(var) {
                Ok(value) => {
                    report.add_info(format!("Environment variable {var} = {value}"));
                }
                Err(_) if required => {
                    report.add_error(ConfigValidationError::MissingField {
                        field: var.to_string(),
                    });
                }
                Err(_) => {
                    report.add_warning(format!("Optional environment variable {var} not set"));
                }
            }
        }
        
        Ok(())
    }
    
    fn validate_directories(&self, report: &mut ValidationReport) -> Result<()> {
        use crate::config::PlatformPaths;
        
        let paths = PlatformPaths::new()?;
        
        // Check if directories can be created/accessed
        let dirs = [
            ("Config", &paths.config_dir),
            ("Cache", &paths.cache_dir),
            ("Data", &paths.data_dir),
            ("Log", &paths.log_dir),
            ("Temp", &paths.temp_dir),
        ];
        
        for (name, dir) in dirs {
            if dir.exists() {
                // Check if we can write to it
                let test_file = dir.join(".lspbridge-test");
                match fs::write(&test_file, "test") {
                    Ok(_) => {
                        let _ = fs::remove_file(test_file);
                        report.add_success(format!("{name} directory is writable: {dir:?}"));
                    }
                    Err(e) => {
                        report.add_error(ConfigValidationError::DirectoryNotAccessible {
                            path: dir.display().to_string(),
                            reason: format!("Not writable: {e}"),
                        });
                    }
                }
            } else {
                // Try to create it
                match fs::create_dir_all(dir) {
                    Ok(_) => {
                        report.add_success(format!("{name} directory created: {dir:?}"));
                    }
                    Err(e) => {
                        report.add_error(ConfigValidationError::DirectoryNotAccessible {
                            path: dir.display().to_string(),
                            reason: format!("Cannot create: {e}"),
                        });
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn validate_config_file(&self, path: &str, report: &mut ValidationReport) -> Result<()> {
        let config_path = Path::new(path);
        
        if !config_path.exists() {
            report.add_error(ConfigValidationError::FileNotFound {
                path: path.to_string(),
            });
            return Ok(());
        }
        
        // Read and parse config file
        let content = fs::read_to_string(config_path)
            .context("Failed to read configuration file")?;
        
        // Try to parse as TOML
        match toml::from_str::<toml::Value>(&content) {
            Ok(config) => {
                report.add_success(format!("Configuration file is valid TOML: {path}"));
                
                // Validate specific fields
                self.validate_config_fields(&config, report);
            }
            Err(e) => {
                report.add_error(ConfigValidationError::InvalidFormat {
                    reason: e.to_string(),
                });
            }
        }
        
        Ok(())
    }
    
    fn validate_config_fields(&self, config: &toml::Value, report: &mut ValidationReport) {
        // Check for common configuration fields
        if let Some(table) = config.as_table() {
            // Validate cache configuration
            if let Some(cache) = table.get("cache").and_then(|v| v.as_table()) {
                if let Some(max_size) = cache.get("max_size_mb").and_then(|v| v.as_integer()) {
                    if max_size <= 0 {
                        report.add_error(ConfigValidationError::InvalidValue {
                            field: "cache.max_size_mb".to_string(),
                            reason: "Must be greater than 0".to_string(),
                        });
                    }
                }
            }
            
            // Validate logging configuration
            if let Some(logging) = table.get("logging").and_then(|v| v.as_table()) {
                if let Some(level) = logging.get("level").and_then(|v| v.as_str()) {
                    let valid_levels = ["error", "warn", "info", "debug", "trace"];
                    if !valid_levels.contains(&level) {
                        report.add_error(ConfigValidationError::InvalidValue {
                            field: "logging.level".to_string(),
                            reason: format!("Invalid log level '{level}', must be one of: {valid_levels:?}"),
                        });
                    }
                }
            }
        }
    }
    
    fn validate_dependencies(&self, report: &mut ValidationReport) -> Result<()> {
        // Check for git
        match std::process::Command::new("git").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                report.add_success(format!("Git is available: {}", version.trim()));
            }
            _ => {
                report.add_warning("Git not found - some features may be limited".to_string());
            }
        }
        
        Ok(())
    }
}

/// Validation report containing all findings
#[derive(Debug)]
pub struct ValidationReport {
    pub errors: Vec<ConfigValidationError>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
    pub successes: Vec<String>,
}

impl ValidationReport {
    fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
            successes: Vec::new(),
        }
    }
    
    fn add_error(&mut self, error: ConfigValidationError) {
        self.errors.push(error);
    }
    
    fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    fn add_info(&mut self, info: String) {
        self.info.push(info);
    }
    
    fn add_success(&mut self, success: String) {
        self.successes.push(success);
    }
    
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    pub fn print_summary(&self) {
        println!("Configuration Validation Report");
        println!("==============================");
        
        if !self.successes.is_empty() {
            println!("\n✅ Successes:");
            for success in &self.successes {
                println!("   {success}");
            }
        }
        
        if !self.info.is_empty() {
            println!("\nℹ️  Information:");
            for info in &self.info {
                println!("   {info}");
            }
        }
        
        if !self.warnings.is_empty() {
            println!("\n⚠️  Warnings:");
            for warning in &self.warnings {
                println!("   {warning}");
            }
        }
        
        if !self.errors.is_empty() {
            println!("\n❌ Errors:");
            for error in &self.errors {
                println!("   {error}");
            }
        }
        
        println!("\nSummary: {} errors, {} warnings", self.errors.len(), self.warnings.len());
    }
}

/// Run configuration validation on startup
pub fn validate_startup_config(config_path: Option<String>) -> Result<()> {
    let validator = ConfigValidator::new();
    let validator = if let Some(path) = config_path {
        validator.with_config_path(path)
    } else {
        validator
    };
    
    let report = validator.validate()?;
    
    if std::env::var("LSP_BRIDGE_VERBOSE").is_ok() {
        report.print_summary();
    }
    
    if report.has_errors() {
        return Err(anyhow::anyhow!("Configuration validation failed with {} errors", report.errors.len()));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validation_report() {
        let mut report = ValidationReport::new();
        report.add_success("Test passed".to_string());
        report.add_warning("Test warning".to_string());
        
        assert!(!report.has_errors());
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.successes.len(), 1);
    }
}