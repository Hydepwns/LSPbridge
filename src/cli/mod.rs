use crate::core::*;
use crate::core::traits::ExportService as ExportServiceTrait;
use crate::capture::{MemoryCache, CaptureService};
use crate::privacy::PrivacyFilter;
use crate::format::FormatConverter;
use crate::export::ExportService;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::io::{self, Read};
use tokio::fs;

#[derive(Parser)]
#[command(name = "lsp-bridge")]
#[command(about = "Universal bridge for exporting IDE diagnostics to AI assistants")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Export current diagnostics
    Export {
        /// Export format
        #[arg(short, long, value_enum, default_value = "json")]
        format: OutputFormat,
        
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Include only errors
        #[arg(long)]
        errors_only: bool,
        
        /// Include only warnings and errors
        #[arg(long)]
        warnings_and_errors: bool,
        
        /// File patterns to include (comma-separated)
        #[arg(long)]
        files: Option<String>,
        
        /// File patterns to exclude (comma-separated)
        #[arg(long)]
        exclude: Option<String>,
        
        /// Maximum number of diagnostics
        #[arg(long)]
        max_results: Option<usize>,
        
        /// Include code context around diagnostics
        #[arg(long)]
        include_context: bool,
        
        /// Number of context lines
        #[arg(long, default_value = "3")]
        context_lines: usize,
        
        /// Privacy policy
        #[arg(long, value_enum, default_value = "default")]
        privacy: PrivacyLevel,
    },
    
    /// Watch for diagnostic changes and export continuously
    Watch {
        /// Export format
        #[arg(short, long, value_enum, default_value = "json")]
        format: OutputFormat,
        
        /// Polling interval in milliseconds
        #[arg(long, default_value = "1000")]
        interval: u64,
        
        /// Include only errors
        #[arg(long)]
        errors_only: bool,
        
        /// Privacy policy
        #[arg(long, value_enum, default_value = "default")]
        privacy: PrivacyLevel,
    },
    
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Initialize default configuration
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

#[derive(clap::ValueEnum, Clone)]
pub enum OutputFormat {
    Json,
    Markdown,
    Claude,
}

#[derive(clap::ValueEnum, Clone)]
pub enum PrivacyLevel {
    Default,
    Strict,
    Permissive,
}

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("lsp_bridge={}", log_level))
        .init();

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
            privacy 
        } => {
            export_diagnostics(ExportArgs {
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
            }).await?;
        }
        
        Commands::Watch { 
            format, 
            interval, 
            errors_only, 
            privacy 
        } => {
            watch_diagnostics(WatchArgs {
                format,
                interval,
                errors_only,
                privacy,
            }).await?;
        }
        
        Commands::Config { action } => {
            manage_config(action).await?;
        }
    }
    
    Ok(())
}

struct ExportArgs {
    format: OutputFormat,
    output: Option<PathBuf>,
    errors_only: bool,
    warnings_and_errors: bool,
    files: Option<String>,
    exclude: Option<String>,
    max_results: Option<usize>,
    include_context: bool,
    context_lines: usize,
    privacy: PrivacyLevel,
}

struct WatchArgs {
    format: OutputFormat,
    interval: u64,
    errors_only: bool,
    privacy: PrivacyLevel,
}

async fn export_diagnostics(args: ExportArgs) -> Result<()> {
    // Setup services
    let privacy_filter = PrivacyFilter::new(get_privacy_policy(&args.privacy));
    let format_converter = FormatConverter::new();
    let cache = MemoryCache::with_defaults();
    let mut capture_service = CaptureService::new(cache, privacy_filter, format_converter);
    let export_service = ExportService::new();

    // Create filter from options
    let filter = create_diagnostic_filter(&args)?;
    
    // Create export config
    let export_config = create_export_config(&args)?;

    // Try to read diagnostics from standard input or find from IDE
    let raw_diagnostics = if atty::is(atty::Stream::Stdin) {
        // Not piped, try to find diagnostics from running IDE
        find_ide_diagnostics().await?
    } else {
        // Read from stdin
        let input = read_stdin().await?;
        RawDiagnostics {
            source: "stdin".to_string(),
            data: serde_json::from_str(&input)?,
            timestamp: chrono::Utc::now(),
            workspace: None,
        }
    };

    // Process diagnostics
    capture_service.process_diagnostics(raw_diagnostics).await?;
    let snapshot = capture_service.get_current_snapshot().await?
        .ok_or_else(|| anyhow!("No diagnostics found"))?;

    // Apply additional filtering if specified
    let filtered_snapshot = if filter != DiagnosticFilter::default() {
        // For simplicity, we'll filter the snapshot's diagnostics directly
        // In a real implementation, you'd use the cache's filtering capabilities
        let mut filtered_diagnostics = snapshot.diagnostics.clone();
        
        if let Some(severities) = &filter.severities {
            let severity_set: std::collections::HashSet<_> = severities.iter().collect();
            filtered_diagnostics.retain(|d| severity_set.contains(&d.severity));
        }
        
        if let Some(max_results) = filter.max_results {
            filtered_diagnostics.truncate(max_results);
        }
        
        DiagnosticSnapshot {
            diagnostics: filtered_diagnostics,
            ..snapshot
        }
    } else {
        snapshot
    };

    // Export
    let output_content = match args.format {
        OutputFormat::Markdown => export_service.export_to_markdown(&filtered_snapshot, &export_config)?,
        OutputFormat::Claude => export_service.export_to_claude_optimized(&filtered_snapshot, &export_config)?,
        OutputFormat::Json => export_service.export_to_json(&filtered_snapshot, &export_config)?,
    };

    // Write output
    if let Some(output_path) = args.output {
        fs::write(&output_path, &output_content).await?;
        eprintln!("Diagnostics exported to {}", output_path.display());
    } else {
        print!("{}", output_content);
    }

    Ok(())
}

async fn watch_diagnostics(args: WatchArgs) -> Result<()> {
    eprintln!("Starting diagnostic watch mode...");
    
    let privacy_filter = PrivacyFilter::new(get_privacy_policy(&args.privacy));
    let format_converter = FormatConverter::new();
    let cache = MemoryCache::with_defaults();
    let mut capture_service = CaptureService::new(cache, privacy_filter, format_converter);
    let export_service = ExportService::new();

    let mut last_output = String::new();
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(args.interval));
    
    loop {
        interval.tick().await;
        
        match watch_iteration(&mut capture_service, &export_service, &args).await {
            Ok(Some(output)) => {
                if output != last_output {
                    println!("{}", output);
                    last_output = output;
                }
            }
            Ok(None) => {
                // No change, continue
            }
            Err(e) => {
                eprintln!("Watch iteration failed: {}", e);
            }
        }
    }
}

async fn watch_iteration(
    capture_service: &mut CaptureService<MemoryCache, PrivacyFilter, FormatConverter>,
    export_service: &ExportService,
    args: &WatchArgs,
) -> Result<Option<String>> {
    let raw_diagnostics = find_ide_diagnostics().await?;
    capture_service.process_diagnostics(raw_diagnostics).await?;
    
    let snapshot = match capture_service.get_current_snapshot().await? {
        Some(s) => s,
        None => return Ok(None),
    };

    let filter = if args.errors_only {
        DiagnosticFilter {
            severities: Some(vec![DiagnosticSeverity::Error]),
            ..Default::default()
        }
    } else {
        DiagnosticFilter::default()
    };

    // Apply filtering
    let mut filtered_diagnostics = snapshot.diagnostics.clone();
    if let Some(severities) = &filter.severities {
        let severity_set: std::collections::HashSet<_> = severities.iter().collect();
        filtered_diagnostics.retain(|d| severity_set.contains(&d.severity));
    }

    let filtered_snapshot = DiagnosticSnapshot {
        diagnostics: filtered_diagnostics,
        ..snapshot
    };

    let export_config = ExportConfig {
        format: match args.format {
            OutputFormat::Json => ExportFormat::Json,
            OutputFormat::Markdown => ExportFormat::Markdown,
            OutputFormat::Claude => ExportFormat::ClaudeOptimized,
        },
        ..Default::default()
    };

    let output = match args.format {
        OutputFormat::Markdown => export_service.export_to_markdown(&filtered_snapshot, &export_config)?,
        OutputFormat::Claude => export_service.export_to_claude_optimized(&filtered_snapshot, &export_config)?,
        OutputFormat::Json => export_service.export_to_json(&filtered_snapshot, &export_config)?,
    };

    Ok(Some(output))
}

async fn manage_config(action: ConfigAction) -> Result<()> {
    let config_path = std::env::current_dir()?.join(".lsp-bridge.toml");

    match action {
        ConfigAction::Init => {
            let default_config = BridgeConfig::default();
            let toml_content = toml::to_string_pretty(&default_config)?;
            fs::write(&config_path, toml_content).await?;
            println!("Configuration initialized at {}", config_path.display());
        }
        
        ConfigAction::Show => {
            match fs::read_to_string(&config_path).await {
                Ok(content) => println!("{}", content),
                Err(_) => println!("No configuration file found. Use 'config init' to create one."),
            }
        }
        
        ConfigAction::Set { key: _, value: _ } => {
            println!("Set configuration not implemented yet");
        }
    }

    Ok(())
}

// Helper functions
fn create_diagnostic_filter(args: &ExportArgs) -> Result<DiagnosticFilter> {
    let mut filter = DiagnosticFilter::default();

    if args.errors_only {
        filter.severities = Some(vec![DiagnosticSeverity::Error]);
    } else if args.warnings_and_errors {
        filter.severities = Some(vec![DiagnosticSeverity::Error, DiagnosticSeverity::Warning]);
    }

    if let Some(files) = &args.files {
        filter.file_patterns = Some(files.split(',').map(|s| s.trim().to_string()).collect());
    }

    if let Some(exclude) = &args.exclude {
        filter.exclude_patterns = Some(exclude.split(',').map(|s| s.trim().to_string()).collect());
    }

    filter.max_results = args.max_results;

    Ok(filter)
}

fn create_export_config(args: &ExportArgs) -> Result<ExportConfig> {
    Ok(ExportConfig {
        format: match args.format {
            OutputFormat::Json => ExportFormat::Json,
            OutputFormat::Markdown => ExportFormat::Markdown,
            OutputFormat::Claude => ExportFormat::ClaudeOptimized,
        },
        include_context: args.include_context,
        context_lines: args.context_lines,
        include_summary: true,
        group_by_file: false,
        sort_by: SortBy::Severity,
    })
}

fn get_privacy_policy(level: &PrivacyLevel) -> PrivacyPolicy {
    match level {
        PrivacyLevel::Strict => PrivacyPolicy::strict(),
        PrivacyLevel::Permissive => PrivacyPolicy::permissive(),
        PrivacyLevel::Default => PrivacyPolicy::default(),
    }
}

async fn find_ide_diagnostics() -> Result<RawDiagnostics> {
    // This is a placeholder - in a real implementation, this would:
    // 1. Look for VS Code diagnostics via extension API
    // 2. Look for Zed diagnostics via its API
    // 3. Look for generic LSP server outputs
    // 4. Check for diagnostic files in common locations
    
    // For now, return empty diagnostics
    Ok(RawDiagnostics {
        source: "auto-detected".to_string(),
        data: serde_json::json!({ "diagnostics": [] }),
        timestamp: chrono::Utc::now(),
        workspace: None,
    })
}

async fn read_stdin() -> Result<String> {
    let mut buffer = String::new();
    let mut stdin = io::stdin();
    stdin.read_to_string(&mut buffer)?;
    Ok(buffer)
}

// Add atty to dependencies for checking if stdin is a tty
// In Cargo.toml, add: atty = "0.2"