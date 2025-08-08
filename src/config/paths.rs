//! Platform-aware configuration paths for LSPbridge

use std::path::PathBuf;
use std::env;
use anyhow::{Result, Context};

/// Get the appropriate configuration directory for the current platform
pub fn config_dir() -> Result<PathBuf> {
    if let Ok(custom_dir) = env::var("LSP_BRIDGE_CONFIG_DIR") {
        return Ok(PathBuf::from(custom_dir));
    }
    
    dirs::config_dir()
        .map(|p| p.join("lspbridge"))
        .context("Unable to determine config directory for the current platform")
}

/// Get the appropriate cache directory for the current platform
pub fn cache_dir() -> Result<PathBuf> {
    // Check for environment variable override first
    if let Ok(custom_dir) = env::var("LSP_BRIDGE_CACHE_DIR") {
        return Ok(PathBuf::from(custom_dir));
    }
    
    // Use platform-specific cache directory
    dirs::cache_dir()
        .map(|p| p.join("lspbridge"))
        .context("Unable to determine cache directory for the current platform")
}

/// Get the appropriate data directory for the current platform
pub fn data_dir() -> Result<PathBuf> {
    if let Ok(custom_dir) = env::var("LSP_BRIDGE_DATA_DIR") {
        return Ok(PathBuf::from(custom_dir));
    }
    
    dirs::data_dir()
        .map(|p| p.join("lspbridge"))
        .context("Unable to determine data directory for the current platform")
}

/// Get the log directory
pub fn log_dir() -> Result<PathBuf> {
    if let Ok(custom_dir) = env::var("LSP_BRIDGE_LOG_DIR") {
        return Ok(PathBuf::from(custom_dir));
    }
    
    // Platform-specific log directories
    #[cfg(target_os = "macos")]
    {
        Ok(dirs::home_dir()
            .context("Unable to determine home directory")?
            .join("Library/Logs/lspbridge"))
    }
    
    #[cfg(target_os = "linux")]
    {
        // Try XDG_STATE_HOME first, then fallback to data directory
        Ok(env::var("XDG_STATE_HOME")
            .map(|p| PathBuf::from(p).join("lspbridge/logs"))
            .unwrap_or_else(|_| data_dir().unwrap_or_default().join("logs")))
    }
    
    #[cfg(target_os = "windows")]
    {
        Ok(data_dir()?.join("logs"))
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Ok(data_dir()?.join("logs"))
    }
}

/// Get the temporary directory for LSPbridge
pub fn temp_dir() -> PathBuf {
    if let Ok(custom_dir) = env::var("LSP_BRIDGE_TEMP_DIR") {
        return PathBuf::from(custom_dir);
    }
    
    env::temp_dir().join("lspbridge")
}

/// Platform-specific path configuration
#[derive(Debug, Clone)]
pub struct PlatformPaths {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub temp_dir: PathBuf,
}

impl PlatformPaths {
    /// Create platform-specific paths with defaults
    pub fn new() -> Result<Self> {
        Ok(Self {
            config_dir: config_dir()?,
            cache_dir: cache_dir()?,
            data_dir: data_dir()?,
            log_dir: log_dir()?,
            temp_dir: temp_dir(),
        })
    }
    
    /// Ensure all directories exist
    pub fn ensure_directories(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)
            .with_context(|| format!("Failed to create config directory: {:?}", self.config_dir))?;
        std::fs::create_dir_all(&self.cache_dir)
            .with_context(|| format!("Failed to create cache directory: {:?}", self.cache_dir))?;
        std::fs::create_dir_all(&self.data_dir)
            .with_context(|| format!("Failed to create data directory: {:?}", self.data_dir))?;
        std::fs::create_dir_all(&self.log_dir)
            .with_context(|| format!("Failed to create log directory: {:?}", self.log_dir))?;
        std::fs::create_dir_all(&self.temp_dir)
            .with_context(|| format!("Failed to create temp directory: {:?}", self.temp_dir))?;
        
        Ok(())
    }
}

/// Get default cache configuration with platform-aware paths
pub fn default_cache_config() -> Result<crate::core::persistent_cache::CacheConfig> {
    Ok(crate::core::persistent_cache::CacheConfig {
        cache_dir: cache_dir()?,
        max_size_mb: 100,
        max_entries: 10000,
        ttl: std::time::Duration::from_secs(24 * 60 * 60),
        enable_compression: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_platform_paths() {
        let paths = PlatformPaths::new().unwrap();
        
        // Paths should be absolute
        assert!(paths.config_dir.is_absolute());
        assert!(paths.cache_dir.is_absolute());
        assert!(paths.data_dir.is_absolute());
        assert!(paths.log_dir.is_absolute());
        
        // Paths should contain "lspbridge" (case-insensitive check for compatibility)
        let config_path = paths.config_dir.to_string_lossy().to_lowercase();
        let cache_path = paths.cache_dir.to_string_lossy().to_lowercase();
        let data_path = paths.data_dir.to_string_lossy().to_lowercase();
        
        assert!(config_path.contains("lspbridge") || config_path.contains("lsp_bridge") || config_path.contains("lsp-bridge"));
        assert!(cache_path.contains("lspbridge") || cache_path.contains("lsp_bridge") || cache_path.contains("lsp-bridge"));
        assert!(data_path.contains("lspbridge") || data_path.contains("lsp_bridge") || data_path.contains("lsp-bridge"));
    }
    
    #[test]
    fn test_env_override() {
        env::set_var("LSP_BRIDGE_CACHE_DIR", "/custom/cache");
        let cache = cache_dir().unwrap();
        assert_eq!(cache, PathBuf::from("/custom/cache"));
        env::remove_var("LSP_BRIDGE_CACHE_DIR");
    }
}