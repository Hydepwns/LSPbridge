use anyhow::Result;
use async_trait::async_trait;
use tokio::fs;

use crate::cli::commands::Command;
use crate::config::ConfigAction;
use crate::core::BridgeConfig;

pub struct ConfigCommand {
    action: ConfigAction,
}

impl ConfigCommand {
    pub fn new(action: ConfigAction) -> Self {
        Self { action }
    }
}

#[async_trait]
impl Command for ConfigCommand {
    async fn execute(&self) -> Result<()> {
        let config_path = std::env::current_dir()?.join(".lsp-bridge.toml");

        match &self.action {
            ConfigAction::Init => {
                let default_config = BridgeConfig::default();
                let toml_content = toml::to_string_pretty(&default_config)?;
                fs::write(&config_path, toml_content).await?;
                println!("Configuration initialized at {}", config_path.display());
            }

            ConfigAction::Show => match fs::read_to_string(&config_path).await {
                Ok(content) => println!("{}", content),
                Err(_) => println!("No configuration file found. Use 'config init' to create one."),
            },

            ConfigAction::Set { key: _, value: _ } => {
                println!("Set configuration not implemented yet");
            }
        }

        Ok(())
    }
}