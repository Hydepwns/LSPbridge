use anyhow::Result;
use lsp_bridge::{cli, config};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Validate configuration on startup
    let config_path = std::env::var("LSP_BRIDGE_CONFIG").ok();
    if let Err(e) = config::validate_startup_config(config_path) {
        tracing::error!("Configuration validation failed: {}", e);
        // Continue with defaults if validation fails in non-critical areas
    }
    
    // Ensure platform directories exist
    if let Ok(paths) = config::PlatformPaths::new() {
        if let Err(e) = paths.ensure_directories() {
            tracing::warn!("Failed to create some directories: {}", e);
        }
    }
    
    cli::run_cli().await
}
