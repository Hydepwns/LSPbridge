pub mod paths;
pub mod validation;

pub use paths::{PlatformPaths, config_dir, cache_dir, data_dir, log_dir, temp_dir};
pub use validation::{ConfigValidator, validate_startup_config};

use clap::Subcommand;

/// Configuration actions for LSPbridge
#[derive(Debug, Clone, Subcommand)]
pub enum ConfigAction {
    /// Initialize configuration file
    Init,
    /// Show current configuration
    Show,
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
}