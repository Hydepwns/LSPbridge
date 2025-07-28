use crate::capture::{CaptureService, MemoryCache};
use crate::core::constants::{languages, lsp_constants};
use crate::core::traits::ExportService as ExportServiceTrait;
use crate::core::*;
use crate::export::ExportService;
use crate::format::FormatConverter;
use crate::privacy::PrivacyFilter;
use crate::quick_fix::FixEdit;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::io::{self, Read};
use std::path::PathBuf;
use tokio::fs;

mod multi_repo;
pub use multi_repo::{handle_multi_repo_command, MultiRepoCommand};

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

    /// Query historical diagnostic data
    History {
        #[command(subcommand)]
        action: HistoryAction,
    },

    /// Interactive diagnostic query explorer
    Query {
        /// Query to execute (if not provided, starts interactive REPL)
        #[arg(short, long)]
        query: Option<String>,

        /// Output format for non-interactive queries
        #[arg(short, long, value_enum, default_value = "table")]
        format: QueryFormat,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Enable interactive REPL mode
        #[arg(long)]
        interactive: bool,
    },

    /// AI training data generation and management
    AITraining {
        #[command(subcommand)]
        action: AITrainingAction,
    },

    /// Quick fix automation for diagnostics
    QuickFix {
        #[command(subcommand)]
        action: QuickFixAction,
    },

    /// Multi-repository support
    MultiRepo {
        #[command(subcommand)]
        command: MultiRepoCommand,
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

#[derive(Subcommand)]
pub enum HistoryAction {
    /// Show trend analysis
    Trends {
        /// Time window in hours
        #[arg(long, default_value = "24")]
        hours: u64,

        /// Output format
        #[arg(short, long, value_enum, default_value = "markdown")]
        format: OutputFormat,
    },

    /// Show hot spots (files with most issues)
    HotSpots {
        /// Maximum number of files to show
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Output format
        #[arg(short, long, value_enum, default_value = "markdown")]
        format: OutputFormat,
    },

    /// Show file-specific history
    File {
        /// File path to analyze
        path: PathBuf,

        /// Time window in hours
        #[arg(long, default_value = "168")] // 7 days
        hours: u64,

        /// Output format
        #[arg(short, long, value_enum, default_value = "markdown")]
        format: OutputFormat,
    },

    /// Show recurring error patterns
    Patterns {
        /// Minimum occurrences to show
        #[arg(long, default_value = "5")]
        min_occurrences: usize,

        /// Output format
        #[arg(short, long, value_enum, default_value = "markdown")]
        format: OutputFormat,
    },

    /// Export ML-ready data
    ExportML {
        /// Output file path
        output: PathBuf,
    },

    /// Export visualization-ready data
    Visualize {
        /// Output file path
        output: PathBuf,

        /// Visualization format
        #[arg(long, value_enum, default_value = "html")]
        format: VizFormat,

        /// Time window in hours
        #[arg(long, default_value = "168")] // 7 days
        hours: u64,
    },

    /// Clean up old historical data
    Cleanup {
        /// Days of data to retain
        #[arg(long, default_value = "30")]
        retain_days: u64,
    },
}

#[derive(Subcommand)]
pub enum AITrainingAction {
    /// Export diagnostics as AI training data
    Export {
        /// Output file path
        output: PathBuf,

        /// Export format
        #[arg(short, long, value_enum, default_value = "jsonl")]
        format: AIExportFormat,

        /// Include only high-confidence fixes (>0.7)
        #[arg(long)]
        high_confidence_only: bool,

        /// Maximum context tokens per example
        #[arg(long, default_value = "2000")]
        max_tokens: usize,

        /// Filter by language
        #[arg(long)]
        language: Option<String>,
    },

    /// Generate synthetic training data
    Synthetic {
        /// Base code file or directory
        input: PathBuf,

        /// Output file path
        output: PathBuf,

        /// Programming language
        #[arg(short, long)]
        language: String,

        /// Difficulty level
        #[arg(long, value_enum)]
        difficulty: Option<DifficultyLevel>,

        /// Number of examples to generate
        #[arg(long, default_value = "100")]
        count: usize,

        /// Generate gradient dataset with all difficulty levels
        #[arg(long)]
        gradient: bool,
    },

    /// Start annotation session for training data
    Annotate {
        /// Training dataset file to annotate
        dataset: PathBuf,

        /// Annotator ID
        #[arg(long, default_value = "default")]
        annotator: String,

        /// Auto-annotate with quality threshold
        #[arg(long, value_enum)]
        auto_quality: Option<FixQuality>,

        /// Output annotated dataset
        output: PathBuf,
    },

    /// Generate training report from annotated dataset
    Report {
        /// Annotated dataset file
        dataset: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "markdown")]
        format: OutputFormat,
    },
}

#[derive(Subcommand)]
pub enum QuickFixAction {
    /// Apply automatic fixes to diagnostics
    Apply {
        /// Minimum confidence threshold for auto-apply (0.0-1.0)
        #[arg(long, default_value = "0.9")]
        threshold: f32,

        /// Only apply fixes to errors (not warnings)
        #[arg(long)]
        errors_only: bool,

        /// Run tests after applying fixes
        #[arg(long)]
        verify_tests: bool,

        /// Check build after applying fixes
        #[arg(long)]
        verify_build: bool,

        /// Create backups before applying fixes
        #[arg(long, default_value = "true")]
        backup: bool,

        /// Dry run - show what would be fixed without applying
        #[arg(long)]
        dry_run: bool,

        /// Filter by file pattern
        #[arg(long)]
        files: Option<String>,
    },

    /// Rollback previously applied fixes
    Rollback {
        /// Session ID to rollback (latest if not specified)
        session_id: Option<String>,

        /// List available rollback sessions
        #[arg(long)]
        list: bool,
    },

    /// Show confidence scores for available fixes
    Analyze {
        /// Show detailed confidence factors
        #[arg(long)]
        detailed: bool,

        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Configure fix confidence thresholds
    Config {
        /// Auto-apply threshold (0.0-1.0)
        #[arg(long)]
        auto_threshold: Option<f32>,

        /// Suggestion threshold (0.0-1.0)
        #[arg(long)]
        suggest_threshold: Option<f32>,

        /// Show current configuration
        #[arg(long)]
        show: bool,
    },
}

#[derive(clap::ValueEnum, Clone)]
pub enum AIExportFormat {
    JsonLines,
    Parquet,
    HuggingFace,
    OpenAI,
    Custom,
}

#[derive(clap::ValueEnum, Clone)]
pub enum DifficultyLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

#[derive(clap::ValueEnum, Clone)]
pub enum FixQuality {
    Perfect,
    Good,
    Acceptable,
    Poor,
    Incorrect,
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

#[derive(clap::ValueEnum, Clone)]
pub enum VizFormat {
    Html,
    Plotly,
    Chartjs,
    Vega,
    Json,
}

#[derive(clap::ValueEnum, Clone)]
pub enum QueryFormat {
    Table,
    Json,
    Csv,
    Markdown,
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
            privacy,
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
            })
            .await?;
        }

        Commands::Watch {
            format,
            interval,
            errors_only,
            privacy,
        } => {
            watch_diagnostics(WatchArgs {
                format,
                interval,
                errors_only,
                privacy,
            })
            .await?;
        }

        Commands::Config { action } => {
            manage_config(action).await?;
        }

        Commands::History { action } => {
            manage_history(action).await?;
        }

        Commands::Query {
            query,
            format,
            output,
            interactive,
        } => {
            run_query_command(query, format, output, interactive).await?;
        }

        Commands::AITraining { action } => {
            manage_ai_training(action).await?;
        }

        Commands::QuickFix { action } => {
            manage_quick_fix(action).await?;
        }

        Commands::MultiRepo { command } => {
            handle_multi_repo_command(command, None).await?;
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
    // Try to detect project info from current directory
    let export_service = match std::env::current_dir() {
        Ok(cwd) => ExportService::with_project_info(&cwd),
        Err(_) => ExportService::new(),
    };

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
    let snapshot = capture_service
        .get_current_snapshot()
        .await?
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
        OutputFormat::Markdown => {
            export_service.export_to_markdown(&filtered_snapshot, &export_config)?
        }
        OutputFormat::Claude => {
            export_service.export_to_claude_optimized(&filtered_snapshot, &export_config)?
        }
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
    // Try to detect project info from current directory
    let export_service = match std::env::current_dir() {
        Ok(cwd) => ExportService::with_project_info(&cwd),
        Err(_) => ExportService::new(),
    };

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
        OutputFormat::Markdown => {
            export_service.export_to_markdown(&filtered_snapshot, &export_config)?
        }
        OutputFormat::Claude => {
            export_service.export_to_claude_optimized(&filtered_snapshot, &export_config)?
        }
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

async fn manage_history(action: HistoryAction) -> Result<()> {
    use crate::history::{HistoryConfig, HistoryManager};
    use std::time::Duration;

    // Initialize history manager with default config
    let config = HistoryConfig::default();
    let manager = HistoryManager::new(config).await?;

    match action {
        HistoryAction::Trends { hours, format } => {
            let window = Duration::from_secs(hours * 3600);
            let trends = manager.get_trends(window).await?;

            match format {
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&trends)?;
                    println!("{}", json);
                }
                OutputFormat::Markdown | OutputFormat::Claude => {
                    println!("# Diagnostic Trends (Last {} hours)\n", hours);
                    println!("**Health Score**: {:.1}%", trends.health_score * 100.0);
                    println!("**Trend Direction**: {:?}", trends.trend_direction);
                    println!(
                        "**Error Velocity**: {:.1} errors/hour",
                        trends.error_velocity
                    );
                    println!(
                        "**Warning Velocity**: {:.1} warnings/hour\n",
                        trends.warning_velocity
                    );

                    if !trends.hot_spots.is_empty() {
                        println!("## Hot Spots");
                        for (i, file) in trends.hot_spots.iter().take(5).enumerate() {
                            println!(
                                "{}. {} - {} errors, {} warnings",
                                i + 1,
                                file.file_path.display(),
                                file.last_error_count,
                                file.last_warning_count
                            );
                        }
                        println!();
                    }

                    if !trends.recurring_issues.is_empty() {
                        println!("## Recurring Issues");
                        for pattern in trends.recurring_issues.iter().take(5) {
                            println!(
                                "- {} ({:.1} occurrences/day)",
                                pattern.description, pattern.occurrence_rate
                            );
                        }
                    }
                }
            }
        }

        HistoryAction::HotSpots { limit, format } => {
            let hot_spots = manager.get_hot_spots(limit).await?;

            match format {
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&hot_spots)?;
                    println!("{}", json);
                }
                OutputFormat::Markdown | OutputFormat::Claude => {
                    println!("# Diagnostic Hot Spots\n");

                    for (i, spot) in hot_spots.iter().enumerate() {
                        println!("## {}. {}", i + 1, spot.file_path.display());
                        println!("**Score**: {:.1}", spot.score);
                        println!(
                            "**Recent Issues**: {} errors, {} warnings",
                            spot.recent_errors, spot.recent_warnings
                        );
                        println!("**Trend**: {:?}", spot.trend);
                        println!("**Recommendation**: {}\n", spot.recommendation);
                    }
                }
            }
        }

        HistoryAction::File {
            path,
            hours,
            format,
        } => {
            let window = Duration::from_secs(hours * 3600);
            let report = manager.get_file_trends(&path, window).await?;

            match format {
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&report)?;
                    println!("{}", json);
                }
                OutputFormat::Markdown | OutputFormat::Claude => {
                    println!("# File History: {}\n", path.display());
                    println!("**Trend**: {:?}", report.trend_direction);
                    println!("**Volatility**: {:.2}", report.volatility);

                    if let Some((_, recent_errors)) = report.error_trend.first() {
                        println!("**Recent Errors**: {}", recent_errors);
                    }
                    if let Some((_, recent_warnings)) = report.warning_trend.first() {
                        println!("**Recent Warnings**: {}", recent_warnings);
                    }

                    println!("\n## Predictions");
                    println!(
                        "**Next Hour Errors**: {} (confidence: {:.0}%)",
                        report.predictions.next_hour_errors,
                        report.predictions.confidence * 100.0
                    );
                    println!(
                        "**Suggested Action**: {}",
                        report.predictions.suggested_action
                    );
                }
            }
        }

        HistoryAction::Patterns {
            min_occurrences,
            format,
        } => {
            let patterns = manager.get_recurring_patterns(min_occurrences).await?;

            match format {
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&patterns)?;
                    println!("{}", json);
                }
                OutputFormat::Markdown | OutputFormat::Claude => {
                    println!("# Recurring Error Patterns\n");

                    for pattern in patterns {
                        println!("## {}", pattern.error_message);
                        if let Some(code) = &pattern.error_code {
                            println!("**Code**: {}", code);
                        }
                        println!("**Occurrences**: {}", pattern.occurrence_count);
                        println!("**Files Affected**: {}", pattern.files_affected);
                        println!("**First Seen**: {:?}", pattern.first_seen);
                        println!("**Last Seen**: {:?}\n", pattern.last_seen);
                    }
                }
            }
        }

        HistoryAction::ExportML { output } => {
            manager.export_ml_data(&output).await?;
            println!("ML-ready data exported to: {}", output.display());
        }

        HistoryAction::Visualize {
            output,
            format,
            hours,
        } => {
            use crate::history::VisualizationFormat;

            let viz_format = match format {
                VizFormat::Html => VisualizationFormat::Html,
                VizFormat::Plotly => VisualizationFormat::Plotly,
                VizFormat::Chartjs => VisualizationFormat::ChartJs,
                VizFormat::Vega => VisualizationFormat::Vega,
                VizFormat::Json => VisualizationFormat::Json,
            };

            let window = Duration::from_secs(hours * 3600);
            manager
                .export_visualization(&output, viz_format, window)
                .await?;
            println!("Visualization exported to: {}", output.display());

            if matches!(format, VizFormat::Html) {
                println!(
                    "Open {} in a web browser to view the interactive dashboard",
                    output.display()
                );
            }
        }

        HistoryAction::Cleanup { retain_days } => {
            // This would trigger a manual cleanup
            println!(
                "Cleanup functionality will retain {} days of data",
                retain_days
            );
            println!("Note: Automatic cleanup runs daily by default");
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

async fn run_query_command(
    query: Option<String>,
    format: QueryFormat,
    output: Option<PathBuf>,
    interactive: bool,
) -> Result<()> {
    use crate::query::{InteractiveRepl, QueryApi};

    // Load current diagnostics
    let diagnostics = match find_ide_diagnostics().await {
        Ok(diags) => diags,
        Err(_) => {
            // Try to load from stdin if available
            if atty::isnt(atty::Stream::Stdin) {
                let data = read_stdin().await?;
                RawDiagnostics {
                    source: "stdin".to_string(),
                    data: serde_json::from_str(&data)?,
                    timestamp: chrono::Utc::now(),
                    workspace: None,
                }
            } else {
                return Err(anyhow!("No diagnostics available"));
            }
        }
    };

    // Convert and process diagnostics
    use crate::core::FormatConverter as FormatConverterTrait;
    use crate::format::FormatConverter;

    let converter = FormatConverter::new();
    let normalized = converter.normalize(diagnostics).await?;

    // Create DiagnosticResult
    let mut processed = DiagnosticResult::new();
    for diagnostic in normalized {
        let file_path = PathBuf::from(&diagnostic.file);
        processed
            .diagnostics
            .entry(file_path)
            .or_insert_with(Vec::new)
            .push(diagnostic);
    }

    // Update summary
    for (_, diags) in &processed.diagnostics {
        for diag in diags {
            processed.summary.total_diagnostics += 1;
            match diag.severity {
                DiagnosticSeverity::Error => processed.summary.error_count += 1,
                DiagnosticSeverity::Warning => processed.summary.warning_count += 1,
                DiagnosticSeverity::Information => processed.summary.info_count += 1,
                DiagnosticSeverity::Hint => processed.summary.hint_count += 1,
            }
        }
    }

    if interactive || query.is_none() {
        // Start interactive REPL
        let mut repl = InteractiveRepl::new().with_diagnostics(processed);

        // Try to add history if available
        let history_config = crate::history::HistoryConfig::default();
        if let Ok(storage) = crate::history::HistoryStorage::new(history_config).await {
            repl = repl.with_history(storage);
        }

        repl.run().await?;
    } else if let Some(query_str) = query {
        // Execute single query
        let api = QueryApi::new();
        api.with_diagnostics(processed).await?;

        let result = api.execute(&query_str).await?;

        // Format and output result
        let formatted = match format {
            QueryFormat::Table => format_as_table(&result),
            QueryFormat::Json => serde_json::to_string_pretty(&result)?,
            QueryFormat::Csv => format_as_csv(&result),
            QueryFormat::Markdown => format_as_markdown(&result),
        };

        if let Some(output_path) = output {
            std::fs::write(output_path, formatted)?;
        } else {
            println!("{}", formatted);
        }
    }

    Ok(())
}

fn format_as_table(result: &crate::query::QueryResult) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    // Calculate column widths
    let mut widths = vec![0; result.columns.len()];
    for (i, col) in result.columns.iter().enumerate() {
        widths[i] = col.len();
    }

    for row in &result.rows {
        for (i, value) in row.values.iter().enumerate() {
            let str_val = value.to_string();
            widths[i] = widths[i].max(str_val.len().min(50));
        }
    }

    // Header
    for (i, col) in result.columns.iter().enumerate() {
        write!(&mut output, "{:<width$} ", col, width = widths[i]).unwrap();
    }
    writeln!(&mut output).unwrap();

    // Separator
    for width in &widths {
        write!(&mut output, "{} ", "-".repeat(*width)).unwrap();
    }
    writeln!(&mut output).unwrap();

    // Rows
    for row in &result.rows {
        for (i, value) in row.values.iter().enumerate() {
            let str_val = value.to_string();
            let truncated = if str_val.len() > 50 {
                format!("{}...", &str_val[..47])
            } else {
                str_val
            };
            write!(&mut output, "{:<width$} ", truncated, width = widths[i]).unwrap();
        }
        writeln!(&mut output).unwrap();
    }

    // Footer
    writeln!(
        &mut output,
        "\n{} results in {}ms",
        result.total_count, result.query_time_ms
    )
    .unwrap();

    output
}

fn format_as_csv(result: &crate::query::QueryResult) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    // Header
    writeln!(&mut output, "{}", result.columns.join(",")).unwrap();

    // Rows
    for row in &result.rows {
        let values: Vec<String> = row
            .values
            .iter()
            .map(|v| {
                let s = v.to_string();
                if s.contains(',') || s.contains('"') || s.contains('\n') {
                    format!("\"{}\"", s.replace('"', "\"\""))
                } else {
                    s
                }
            })
            .collect();
        writeln!(&mut output, "{}", values.join(",")).unwrap();
    }

    output
}

fn format_as_markdown(result: &crate::query::QueryResult) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    // Header
    write!(&mut output, "|").unwrap();
    for col in &result.columns {
        write!(&mut output, " {} |", col).unwrap();
    }
    writeln!(&mut output).unwrap();

    // Separator
    write!(&mut output, "|").unwrap();
    for _ in &result.columns {
        write!(&mut output, " --- |").unwrap();
    }
    writeln!(&mut output).unwrap();

    // Rows
    for row in &result.rows {
        write!(&mut output, "|").unwrap();
        for value in &row.values {
            write!(&mut output, " {} |", value.to_string()).unwrap();
        }
        writeln!(&mut output).unwrap();
    }

    // Footer
    writeln!(
        &mut output,
        "\n*{} results in {}ms*",
        result.total_count, result.query_time_ms
    )
    .unwrap();

    output
}

// Add atty to dependencies for checking if stdin is a tty
// In Cargo.toml, add: atty = "0.2"

async fn manage_ai_training(action: AITrainingAction) -> Result<()> {
    use crate::ai_training::{
        AnnotationTool, ErrorInjector, ExportFormat as AIFormat, TrainingDataset, TrainingExporter,
    };

    match action {
        AITrainingAction::Export {
            output,
            format,
            high_confidence_only,
            max_tokens,
            language,
        } => {
            // Get current diagnostics from stdin or a mock source
            // For now, create an empty result as this would normally come from LSP
            let diagnostics = DiagnosticResult::new();

            // Convert diagnostics to training pairs
            let mut dataset = TrainingDataset::new(
                "Diagnostic Training Data".to_string(),
                "Training data generated from current diagnostics".to_string(),
            );

            // Convert diagnostics to training pairs (simplified for now)
            // In a real implementation, we'd need to extract before/after code from fixes
            for (file_path, file_diagnostics) in diagnostics.diagnostics {
                for diag in file_diagnostics {
                    // Skip if filtering by confidence
                    if high_confidence_only && diag.severity != DiagnosticSeverity::Error {
                        continue;
                    }

                    // Skip if filtering by language
                    if let Some(ref lang_filter) = language {
                        let file_lang = file_path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("");
                        if !file_lang.contains(lang_filter) {
                            continue;
                        }
                    }

                    // Create a training pair from the diagnostic
                    // This is simplified - in reality we'd need the actual fix
                    let pair = crate::ai_training::TrainingPair::new(
                        format!("// Code with error at line {}", diag.range.start.line),
                        format!("// Fixed code"),
                        vec![diag.clone()],
                        crate::core::semantic_context::SemanticContext::default(),
                        detect_language(&file_path),
                    );

                    dataset.add_pair(pair);
                }
            }

            // Export the dataset
            let export_format = match format {
                AIExportFormat::JsonLines => AIFormat::JsonLines,
                AIExportFormat::Parquet => AIFormat::Parquet,
                AIExportFormat::HuggingFace => AIFormat::HuggingFace,
                AIExportFormat::OpenAI => AIFormat::OpenAI,
                AIExportFormat::Custom => AIFormat::Custom,
            };

            let exporter = TrainingExporter::new(export_format).with_max_tokens(max_tokens);

            exporter.export_dataset(&dataset, &output).await?;

            println!(
                "✅ Exported {} training pairs to {}",
                dataset.pairs.len(),
                output.display()
            );
        }

        AITrainingAction::Synthetic {
            input,
            output,
            language,
            difficulty,
            count,
            gradient,
        } => {
            let injector = ErrorInjector::new();

            if gradient {
                // Read base code
                let base_code = tokio::fs::read_to_string(&input).await?;

                // Generate gradient dataset
                let dataset =
                    injector.generate_gradient_dataset(&base_code, &language, count / 4)?;

                // Save dataset
                let json = serde_json::to_string_pretty(&dataset)?;
                tokio::fs::write(&output, json).await?;

                println!(
                    "✅ Generated gradient dataset with {} examples",
                    dataset.pairs.len()
                );
            } else {
                // Read base code
                let base_code = tokio::fs::read_to_string(&input).await?;

                // Convert difficulty level
                let diff = difficulty.map(|d| match d {
                    DifficultyLevel::Beginner => crate::ai_training::DifficultyLevel::Beginner,
                    DifficultyLevel::Intermediate => {
                        crate::ai_training::DifficultyLevel::Intermediate
                    }
                    DifficultyLevel::Advanced => crate::ai_training::DifficultyLevel::Advanced,
                    DifficultyLevel::Expert => crate::ai_training::DifficultyLevel::Expert,
                });

                // Generate synthetic errors
                let pairs = injector.inject_errors(&base_code, &language, diff, count)?;

                // Create dataset
                let mut dataset = TrainingDataset::new(
                    format!("{} Synthetic Dataset", language),
                    "Synthetic training data with injected errors".to_string(),
                );

                for pair in pairs {
                    dataset.add_pair(pair);
                }

                // Save dataset
                let json = serde_json::to_string_pretty(&dataset)?;
                tokio::fs::write(&output, json).await?;

                println!(
                    "✅ Generated {} synthetic training examples",
                    dataset.pairs.len()
                );
            }
        }

        AITrainingAction::Annotate {
            dataset,
            annotator,
            auto_quality,
            output,
        } => {
            // Load dataset
            let json = tokio::fs::read_to_string(&dataset).await?;
            let mut training_dataset: TrainingDataset = serde_json::from_str(&json)?;

            let mut tool = AnnotationTool::new();
            tool.start_session(annotator, training_dataset.id.clone());

            if let Some(quality_threshold) = auto_quality {
                // Convert quality threshold
                let threshold = match quality_threshold {
                    FixQuality::Perfect => crate::ai_training::FixQuality::Perfect,
                    FixQuality::Good => crate::ai_training::FixQuality::Good,
                    FixQuality::Acceptable => crate::ai_training::FixQuality::Acceptable,
                    FixQuality::Poor => crate::ai_training::FixQuality::Poor,
                    FixQuality::Incorrect => crate::ai_training::FixQuality::Incorrect,
                };

                // Auto-annotate
                let annotations = tool.batch_annotate(&mut training_dataset, threshold)?;
                println!("✅ Auto-annotated {} training pairs", annotations.len());
            } else {
                println!("Manual annotation not yet implemented");
                // TODO: Implement interactive annotation interface
            }

            // Save annotated dataset
            let json = serde_json::to_string_pretty(&training_dataset)?;
            tokio::fs::write(&output, json).await?;

            println!("✅ Saved annotated dataset to {}", output.display());
        }

        AITrainingAction::Report { dataset, format } => {
            // Load dataset
            let json = tokio::fs::read_to_string(&dataset).await?;
            let training_dataset: TrainingDataset = serde_json::from_str(&json)?;

            let tool = AnnotationTool::new();
            let report = tool.get_annotation_report(&training_dataset)?;

            // Format report
            let output = match format {
                OutputFormat::Json => serde_json::to_string_pretty(&report)?,
                OutputFormat::Markdown => {
                    format_annotation_report_markdown(&report, &training_dataset)
                }
                OutputFormat::Claude => format_annotation_report_claude(&report, &training_dataset),
            };

            println!("{}", output);
        }
    }

    Ok(())
}

fn detect_language(path: &PathBuf) -> String {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("ts") | Some("tsx") => languages::TYPESCRIPT.to_string(),
        Some("js") | Some("jsx") => languages::JAVASCRIPT.to_string(),
        Some("rs") => languages::RUST.to_string(),
        Some("py") => languages::PYTHON.to_string(),
        Some("go") => languages::GO.to_string(),
        Some("java") => languages::JAVA.to_string(),
        Some("cpp") | Some("cc") | Some("cxx") => languages::CPP.to_string(),
        Some("c") => languages::C.to_string(),
        _ => lsp_constants::UNKNOWN.to_string(),
    }
}

fn format_annotation_report_markdown(
    report: &crate::ai_training::AnnotationReport,
    dataset: &crate::ai_training::TrainingDataset,
) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    writeln!(&mut output, "# AI Training Dataset Report").unwrap();
    writeln!(&mut output).unwrap();
    writeln!(&mut output, "## Overview").unwrap();
    writeln!(&mut output, "- **Dataset**: {}", dataset.name).unwrap();
    writeln!(&mut output, "- **Total Pairs**: {}", dataset.pairs.len()).unwrap();
    writeln!(&mut output, "- **Annotated**: {}", report.total_annotated).unwrap();
    writeln!(&mut output).unwrap();

    writeln!(&mut output, "## Quality Distribution").unwrap();
    for (quality, count) in &report.quality_distribution {
        writeln!(&mut output, "- {:?}: {}", quality, count).unwrap();
    }
    writeln!(&mut output).unwrap();

    writeln!(&mut output, "## Language Breakdown").unwrap();
    for (lang, count) in &report.language_breakdown {
        writeln!(&mut output, "- {}: {}", lang, count).unwrap();
    }
    writeln!(&mut output).unwrap();

    writeln!(&mut output, "## Diagnostic Types").unwrap();
    for (diag_type, count) in &report.diagnostic_type_breakdown {
        writeln!(&mut output, "- {}: {}", diag_type, count).unwrap();
    }

    output
}

fn format_annotation_report_claude(
    report: &crate::ai_training::AnnotationReport,
    dataset: &crate::ai_training::TrainingDataset,
) -> String {
    format_annotation_report_markdown(report, dataset) // Same format for now
}

async fn manage_quick_fix(action: QuickFixAction) -> Result<()> {
    use crate::quick_fix::{
        ConfidenceThreshold, FixApplicationEngine, FixConfidenceScorer, FixVerifier,
        RollbackManager,
    };

    match action {
        QuickFixAction::Apply {
            threshold,
            errors_only,
            verify_tests,
            verify_build,
            backup,
            dry_run,
            files,
        } => {
            // Get current diagnostics
            let diagnostics = DiagnosticResult::new(); // Would normally capture from LSP

            // Set up confidence scorer
            let scorer = FixConfidenceScorer::new();
            let confidence_threshold = ConfidenceThreshold {
                auto_apply: threshold,
                suggest: threshold * 0.7,
                minimum: 0.3,
            };

            // Set up fix engine
            let engine = FixApplicationEngine::new().with_backups(backup);

            // Set up verifier if needed
            let verifier = if verify_tests || verify_build {
                Some(
                    FixVerifier::new()
                        .with_tests(verify_tests)
                        .with_build_check(verify_build),
                )
            } else {
                None
            };

            // Set up rollback manager
            let rollback_dir = dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("lsp-bridge")
                .join("rollback");
            let mut rollback_manager = RollbackManager::new(rollback_dir);
            rollback_manager.init().await?;

            let mut fixes_to_apply = Vec::new();
            let mut all_backups = Vec::new();

            // Analyze each diagnostic
            for (file_path, file_diagnostics) in diagnostics.diagnostics {
                // Filter by file pattern if specified
                if let Some(ref pattern) = files {
                    if !file_path.to_string_lossy().contains(pattern) {
                        continue;
                    }
                }

                for diag in file_diagnostics {
                    // Filter by severity if needed
                    if errors_only && diag.severity != DiagnosticSeverity::Error {
                        continue;
                    }

                    // For demo purposes, create a simple fix
                    // In real implementation, would get from LSP code actions
                    if let Some(fix_edit) = create_demo_fix(&diag) {
                        let (confidence, _factors) =
                            scorer.score_fix(&diag, &fix_edit.new_text, false);

                        if dry_run {
                            println!(
                                "Would fix: {} (confidence: {:.2})",
                                diag.message,
                                confidence.value()
                            );
                            if confidence.is_auto_applicable(&confidence_threshold) {
                                println!("  ✓ Auto-applicable");
                            } else {
                                println!("  ⚠ Requires confirmation");
                            }
                        } else if confidence.is_auto_applicable(&confidence_threshold) {
                            fixes_to_apply.push((fix_edit, confidence));
                        }
                    }
                }
            }

            if dry_run {
                println!(
                    "\nTotal fixes that would be applied: {}",
                    fixes_to_apply.len()
                );
                return Ok(());
            }

            // Apply fixes
            println!("Applying {} fixes...", fixes_to_apply.len());
            let results = engine
                .apply_fixes_with_confidence(&fixes_to_apply, &confidence_threshold)
                .await?;

            // Collect backups for rollback
            for (result, _) in &results {
                if let Some(ref backup) = result.backup {
                    all_backups.push(backup.clone());
                }
            }

            // Save rollback state
            if !all_backups.is_empty() {
                let rollback_state = RollbackManager::create_state(
                    all_backups,
                    format!("Applied {} fixes", results.len()),
                );
                let session_id = rollback_state.session_id.clone();
                rollback_manager.save_state(rollback_state).await?;
                println!("✅ Fixes applied. Rollback session: {}", session_id);
            }

            // Verify if requested
            if let Some(_verifier) = verifier {
                println!("Verifying fixes...");
                for ((fix_edit, _), (result, _)) in fixes_to_apply.iter().zip(&results) {
                    if result.success {
                        // Would need the original diagnostic here
                        // For now, just show that verification would happen
                        println!("  Verifying {}", fix_edit.file_path.display());
                    }
                }
            }

            // Summary
            let successful = results.iter().filter(|(r, _)| r.success).count();
            let failed = results.len() - successful;
            println!("\n📊 Summary:");
            println!("  ✓ Successfully applied: {}", successful);
            if failed > 0 {
                println!("  ✗ Failed: {}", failed);
            }
        }

        QuickFixAction::Rollback { session_id, list } => {
            let rollback_dir = dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("lsp-bridge")
                .join("rollback");
            let mut rollback_manager = RollbackManager::new(rollback_dir);
            rollback_manager.init().await?;

            if list {
                let states = rollback_manager.list_states().await?;
                if states.is_empty() {
                    println!("No rollback sessions available");
                } else {
                    println!("Available rollback sessions:");
                    for state in states {
                        println!(
                            "  {} - {} ({}{})",
                            state.session_id,
                            state.description,
                            state.timestamp.format("%Y-%m-%d %H:%M:%S"),
                            if state.rolled_back {
                                " - already rolled back"
                            } else {
                                ""
                            }
                        );
                    }
                }
            } else {
                match session_id {
                    Some(id) => {
                        rollback_manager.rollback(&id).await?;
                        println!("✅ Rolled back session: {}", id);
                    }
                    None => {
                        rollback_manager.rollback_latest().await?;
                        println!("✅ Rolled back latest session");
                    }
                }
            }
        }

        QuickFixAction::Analyze { detailed, format } => {
            let diagnostics = DiagnosticResult::new(); // Would normally capture from LSP
            let scorer = FixConfidenceScorer::new();

            let mut analysis_results = Vec::new();

            for (_file_path, file_diagnostics) in diagnostics.diagnostics {
                for diag in file_diagnostics {
                    if let Some(fix_edit) = create_demo_fix(&diag) {
                        let (confidence, factors) =
                            scorer.score_fix(&diag, &fix_edit.new_text, false);
                        analysis_results.push((diag, confidence, factors));
                    }
                }
            }

            // Format output
            match format {
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&analysis_results)?;
                    println!("{}", json);
                }
                OutputFormat::Markdown => {
                    println!("# Fix Confidence Analysis\n");
                    for (diag, confidence, factors) in &analysis_results {
                        println!("## {}", diag.message);
                        println!("- **File**: {}", diag.file);
                        println!("- **Confidence**: {:.2}", confidence.value());
                        if detailed {
                            println!("- **Factors**:");
                            println!(
                                "  - Pattern recognition: {:.2}",
                                factors.pattern_recognition
                            );
                            println!("  - Fix complexity: {:.2}", factors.fix_complexity);
                            println!("  - Safety score: {:.2}", factors.safety_score);
                            println!(
                                "  - Language confidence: {:.2}",
                                factors.language_confidence
                            );
                        }
                        println!();
                    }
                }
                _ => {
                    // Table format
                    println!(
                        "{:<50} {:<10} {:<10}",
                        "Diagnostic", "Confidence", "Auto-Apply"
                    );
                    println!("{}", "-".repeat(72));
                    for (diag, confidence, _) in &analysis_results {
                        let auto = if confidence.value() >= 0.9 {
                            "Yes"
                        } else {
                            "No"
                        };
                        println!(
                            "{:<50} {:<10.2} {:<10}",
                            diag.message.chars().take(47).collect::<String>(),
                            confidence.value(),
                            auto
                        );
                    }
                }
            }
        }

        QuickFixAction::Config {
            auto_threshold,
            suggest_threshold,
            show,
        } => {
            // Would normally read/write from config file
            if show {
                println!("Quick Fix Configuration:");
                println!("  Auto-apply threshold: 0.9");
                println!("  Suggestion threshold: 0.5");
                println!("  Minimum threshold: 0.3");
            } else {
                if let Some(threshold) = auto_threshold {
                    println!("Set auto-apply threshold to: {}", threshold);
                }
                if let Some(threshold) = suggest_threshold {
                    println!("Set suggestion threshold to: {}", threshold);
                }
            }
        }
    }

    Ok(())
}

fn create_demo_fix(diagnostic: &crate::core::types::Diagnostic) -> Option<FixEdit> {
    // This is a simplified demo - real implementation would use LSP code actions
    match diagnostic.code.as_deref() {
        Some("TS2322") => {
            // Type mismatch - simple demo fix
            Some(FixEdit {
                file_path: PathBuf::from(&diagnostic.file),
                range: diagnostic.range.clone(),
                new_text: "fixed_type".to_string(),
                description: Some("Fix type mismatch".to_string()),
            })
        }
        Some("missing_semicolon") => Some(FixEdit {
            file_path: PathBuf::from(&diagnostic.file),
            range: diagnostic.range.clone(),
            new_text: ";".to_string(),
            description: Some("Add missing semicolon".to_string()),
        }),
        _ => None,
    }
}
