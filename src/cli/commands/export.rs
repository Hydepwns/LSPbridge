use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tokio::fs;

use crate::capture::{CaptureService, MemoryCache};
use crate::core::DiagnosticsCaptureService;
use crate::cli::args::{ExportArgs, OutputFormat};
use crate::cli::commands::Command;
use crate::core::traits::ExportService as ExportServiceTrait;
use crate::core::{
    DiagnosticFilter, DiagnosticSnapshot, ExportConfig, ExportFormat,
    RawDiagnostics, SortBy,
};
use crate::core::security_config::PrivacyLevel;
use crate::core::PrivacyPolicy;
use crate::export::ExportService;
use crate::format::FormatConverter;
use crate::privacy::PrivacyFilter;
use crate::security::validate_path;

use super::utils::create_diagnostic_filter;

pub struct ExportCommand {
    args: ExportArgs,
}

impl ExportCommand {
    pub fn new(args: ExportArgs) -> Self {
        Self { args }
    }
}

#[async_trait]
impl Command for ExportCommand {
    async fn execute(&self) -> Result<()> {
        // Setup services
        let privacy_filter = PrivacyFilter::new(get_privacy_policy(&self.args.privacy));
        let format_converter = FormatConverter::new();
        let cache = MemoryCache::with_defaults();
        let mut capture_service = CaptureService::new(cache, privacy_filter, format_converter);
        
        // Try to detect project info from current directory
        let export_service = match std::env::current_dir() {
            Ok(cwd) => ExportService::with_project_info(&cwd),
            Err(_) => ExportService::new(),
        };

        // Create filter from options
        let filter = create_diagnostic_filter(
            self.args.errors_only,
            self.args.warnings_and_errors,
            self.args.files.clone(),
            self.args.exclude.clone(),
            self.args.max_results,
        )?;

        // Create export config
        let export_config = create_export_config(&self.args)?;

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
        capture_service.start_capture().await?;
        capture_service.process_diagnostics(raw_diagnostics).await?;
        let snapshot = capture_service
            .get_current_snapshot()
            .await?
            .ok_or_else(|| anyhow!("No diagnostics found"))?;

        // Apply additional filtering if specified
        let filtered_snapshot = apply_filtering(snapshot, &filter)?;

        // Export
        let output_content = match self.args.format {
            OutputFormat::Markdown => {
                export_service.export_to_markdown(&filtered_snapshot, &export_config)?
            }
            OutputFormat::Claude => {
                export_service.export_to_claude_optimized(&filtered_snapshot, &export_config)?
            }
            OutputFormat::Json => {
                export_service.export_to_json(&filtered_snapshot, &export_config)?
            }
        };

        // Write output
        if let Some(output_path) = &self.args.output {
            // Validate the output path for security
            let validated_path = validate_path(output_path)?;
            fs::write(&validated_path, &output_content).await?;
            eprintln!("Diagnostics exported to {}", validated_path.display());
        } else {
            print!("{output_content}");
        }

        Ok(())
    }
}

// Helper functions specific to export command

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

pub fn get_privacy_policy(level: &PrivacyLevel) -> PrivacyPolicy {
    match level {
        PrivacyLevel::Strict => PrivacyPolicy::strict(),
        PrivacyLevel::Minimal => PrivacyPolicy::permissive(),
        PrivacyLevel::Balanced => PrivacyPolicy::default(),
    }
}

fn apply_filtering(
    snapshot: DiagnosticSnapshot,
    filter: &DiagnosticFilter,
) -> Result<DiagnosticSnapshot> {
    if filter == &DiagnosticFilter::default() {
        return Ok(snapshot);
    }

    let mut filtered_diagnostics = snapshot.diagnostics.clone();

    if let Some(severities) = &filter.severities {
        let severity_set: std::collections::HashSet<_> = severities.iter().collect();
        filtered_diagnostics.retain(|d| severity_set.contains(&d.severity));
    }

    if let Some(max_results) = filter.max_results {
        filtered_diagnostics.truncate(max_results);
    }

    Ok(DiagnosticSnapshot {
        diagnostics: filtered_diagnostics,
        ..snapshot
    })
}

pub async fn find_ide_diagnostics() -> Result<RawDiagnostics> {
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

pub async fn read_stdin() -> Result<String> {
    use std::io::{self, Read};
    let mut buffer = String::new();
    let mut stdin = io::stdin();
    stdin.read_to_string(&mut buffer)?;
    Ok(buffer)
}