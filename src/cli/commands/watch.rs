use anyhow::Result;
use async_trait::async_trait;

use crate::capture::{CaptureService, MemoryCache};
use crate::core::DiagnosticsCaptureService;
use crate::cli::args::{OutputFormat, WatchArgs};
use crate::cli::commands::Command;
use crate::core::traits::ExportService as ExportServiceTrait;
use crate::core::{
    DiagnosticFilter, DiagnosticSeverity, DiagnosticSnapshot, ExportConfig, ExportFormat,
};
use crate::export::ExportService;
use crate::format::FormatConverter;
use crate::privacy::PrivacyFilter;

use super::export::{find_ide_diagnostics, get_privacy_policy};

pub struct WatchCommand {
    args: WatchArgs,
}

impl WatchCommand {
    pub fn new(args: WatchArgs) -> Self {
        Self { args }
    }
}

#[async_trait]
impl Command for WatchCommand {
    async fn execute(&self) -> Result<()> {
        eprintln!("Starting diagnostic watch mode...");

        let privacy_filter = PrivacyFilter::new(get_privacy_policy(&self.args.privacy));
        let format_converter = FormatConverter::new();
        let cache = MemoryCache::with_defaults();
        let mut capture_service = CaptureService::new(cache, privacy_filter, format_converter);
        
        // Try to detect project info from current directory
        let export_service = match std::env::current_dir() {
            Ok(cwd) => ExportService::with_project_info(&cwd),
            Err(_) => ExportService::new(),
        };

        let mut last_output = String::new();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(self.args.interval));

        capture_service.start_capture().await?;

        loop {
            interval.tick().await;

            match self.watch_iteration(&mut capture_service, &export_service).await {
                Ok(Some(output)) => {
                    if output != last_output {
                        println!("{output}");
                        last_output = output;
                    }
                }
                Ok(None) => {
                    // No change, continue
                }
                Err(e) => {
                    eprintln!("Watch iteration failed: {e}");
                }
            }
        }
    }
}

impl WatchCommand {
    async fn watch_iteration(
        &self,
        capture_service: &mut CaptureService<MemoryCache, PrivacyFilter, FormatConverter>,
        export_service: &ExportService,
    ) -> Result<Option<String>> {
        let raw_diagnostics = find_ide_diagnostics().await?;
        capture_service.process_diagnostics(raw_diagnostics).await?;

        let snapshot = match capture_service.get_current_snapshot().await? {
            Some(s) => s,
            None => return Ok(None),
        };

        let filter = if self.args.errors_only {
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
            format: match self.args.format {
                OutputFormat::Json => ExportFormat::Json,
                OutputFormat::Markdown => ExportFormat::Markdown,
                OutputFormat::Claude => ExportFormat::ClaudeOptimized,
            },
            ..Default::default()
        };

        let output = match self.args.format {
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

        Ok(Some(output))
    }
}