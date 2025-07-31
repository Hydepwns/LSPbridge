use anyhow::Result;
use clap::Parser;

// Re-export command modules
pub mod args;
pub mod commands;
pub mod multi_repo;

// Re-export commonly used types
pub use args::{Cli, Commands, OutputFormat, QueryOutputFormat};
pub use multi_repo::{handle_multi_repo_command, MultiRepoCommand};

use commands::{
    ai_training::AITrainingCommand, config::ConfigCommand, export::ExportCommand,
    history::HistoryCommand, query::QueryCommand, quick_fix::QuickFixCommand, watch::WatchCommand,
    Command,
};

/// Main entry point for the CLI application.
/// 
/// This function parses command line arguments and routes them to the appropriate
/// command handler. Each command is implemented as a separate module for better
/// organization and maintainability.
/// 
/// # Examples
/// 
/// ```rust
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     run_cli().await?;
///     Ok(())
/// }
/// ```
pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("lsp_bridge={}", log_level))
        .init();

    // Route to appropriate command handler
    match cli.command {
        Commands::Export {
            format,
            output,
            errors_only,
            warnings_and_errors,
            files,
            exclude,
            max_results,
            include_context,
            context_lines,
            privacy,
        } => {
            let args = args::ExportArgs {
                format,
                output,
                errors_only,
                warnings_and_errors,
                files,
                exclude,
                max_results,
                include_context,
                context_lines,
                privacy,
            };
            ExportCommand::new(args).execute().await
        }

        Commands::Watch {
            format,
            interval,
            errors_only,
            privacy,
        } => {
            let args = args::WatchArgs {
                format,
                interval,
                errors_only,
                privacy,
            };
            WatchCommand::new(args).execute().await
        }

        Commands::Query {
            query,
            format,
            output,
            interactive,
        } => {
            let args = args::QueryArgs {
                query,
                format,
                output,
                interactive,
            };
            QueryCommand::new(args).execute().await
        }

        Commands::History { action } => HistoryCommand::new(action).execute().await,

        Commands::AITraining { action } => AITrainingCommand::new(action).execute().await,

        Commands::QuickFix { action } => QuickFixCommand::new(action).execute().await,

        Commands::Config { action } => ConfigCommand::new(action).execute().await,

        Commands::MultiRepo { command } => handle_multi_repo_command(command, None).await,
    }
}