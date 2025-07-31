use super::super::utils::SerdeUtils;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Common configuration trait that all config structs should implement
pub trait ConfigDefaults {
    /// Create config with sensible defaults
    fn with_defaults() -> Self;

    /// Validate configuration values
    fn validate(&self) -> Result<()> {
        Ok(()) // Default implementation - override for validation
    }

    /// Get configuration file name
    fn config_file_name() -> &'static str;
}

/// Standard configuration wrapper with common functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config<T: ConfigDefaults> {
    #[serde(flatten)]
    pub inner: T,

    /// Path where this config was loaded from (if any)
    #[serde(skip)]
    pub source_path: Option<PathBuf>,
}

impl<T: ConfigDefaults + Serialize + for<'de> Deserialize<'de>> Config<T> {
    /// Create new config with defaults
    pub fn new() -> Self {
        Self {
            inner: T::with_defaults(),
            source_path: None,
        }
    }

    /// Load config from file, falling back to defaults if file doesn't exist
    pub async fn load_or_default(path: &Path) -> Result<Self> {
        if path.exists() {
            Self::load(path).await
        } else {
            Ok(Self::new())
        }
    }

    /// Load config from file
    pub async fn load(path: &Path) -> Result<Self> {
        let inner = SerdeUtils::read_json_file(path).await?;
        Ok(Self {
            inner,
            source_path: Some(path.to_path_buf()),
        })
    }

    /// Save config to file
    pub async fn save(&self, path: &Path) -> Result<()> {
        self.inner.validate()?;
        SerdeUtils::write_json_file(path, &self.inner).await
    }

    /// Save to the path it was loaded from (if any)
    pub async fn save_to_source(&self) -> Result<()> {
        if let Some(ref path) = self.source_path {
            self.save(path).await
        } else {
            anyhow::bail!("No source path available for config")
        }
    }
}

impl<T: ConfigDefaults> Default for Config<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to easily implement ConfigDefaults for simple config structs
#[macro_export]
macro_rules! impl_config_defaults {
    ($struct_name:ident, $file_name:literal) => {
        impl ConfigDefaults for $struct_name {
            fn with_defaults() -> Self {
                Self::default()
            }

            fn config_file_name() -> &'static str {
                $file_name
            }
        }
    };

    ($struct_name:ident, $file_name:literal, validate => $validate_fn:expr) => {
        impl ConfigDefaults for $struct_name {
            fn with_defaults() -> Self {
                Self::default()
            }

            fn validate(&self) -> anyhow::Result<()> {
                $validate_fn(self)
            }

            fn config_file_name() -> &'static str {
                $file_name
            }
        }
    };
}

/// Common configuration field patterns
pub mod common_fields {
    use std::path::PathBuf;

    /// Common cache configuration
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct CacheConfig {
        pub enable_cache: bool,
        pub cache_dir: PathBuf,
        pub max_size_mb: u64,
        pub max_entries: usize,
        pub ttl_hours: u64,
    }

    impl Default for CacheConfig {
        fn default() -> Self {
            Self {
                enable_cache: true,
                cache_dir: std::env::temp_dir().join("lspbridge-cache"),
                max_size_mb: 100,
                max_entries: 10000,
                ttl_hours: 24,
            }
        }
    }

    /// Common performance configuration
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct PerformanceConfig {
        pub max_concurrent_files: usize,
        pub timeout_seconds: u64,
        pub chunk_size: usize,
        pub parallel_processing: bool,
    }

    impl Default for PerformanceConfig {
        fn default() -> Self {
            Self {
                max_concurrent_files: 1000,
                timeout_seconds: 30,
                chunk_size: 100,
                parallel_processing: true,
            }
        }
    }
}
