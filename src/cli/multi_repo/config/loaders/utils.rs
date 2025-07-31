use crate::cli::multi_repo::config::types::MultiRepoCliConfig;
use anyhow::Result;
use std::path::PathBuf;

/// Configuration utilities
pub struct ConfigUtils;

impl ConfigUtils {
    /// Get default configuration file path
    pub fn default_config_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to determine home directory"))?;
        
        Ok(home_dir.join(".lspbridge").join("multi-repo-config.json"))
    }

    /// Get default workspace path
    pub fn default_workspace_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to determine home directory"))?;
        
        Ok(home_dir.join(".lspbridge").join("workspace"))
    }

    /// Merge configurations (overlay on top of base)
    pub fn merge_configs(base: MultiRepoCliConfig, overlay: MultiRepoCliConfig) -> MultiRepoCliConfig {
        MultiRepoCliConfig {
            default_output_format: overlay.default_output_format,
            auto_detect_monorepos: overlay.auto_detect_monorepos,
            limits: overlay.limits,
            workspace: overlay.workspace,
            analysis: overlay.analysis,
            team: overlay.team,
            discovery: overlay.discovery,
            aliases: {
                let mut merged = base.aliases;
                merged.extend(overlay.aliases);
                merged
            },
        }
    }
}