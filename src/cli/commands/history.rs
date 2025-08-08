use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;

use crate::cli::args::OutputFormat;
use crate::cli::commands::Command;
use crate::history::{HistoryAction, HistoryConfig, HistoryManager};
use crate::security::validate_path;

pub struct HistoryCommand {
    action: HistoryAction,
}

impl HistoryCommand {
    pub fn new(action: HistoryAction) -> Self {
        Self { action }
    }
}

#[async_trait]
impl Command for HistoryCommand {
    async fn execute(&self) -> Result<()> {
        // Initialize history manager with default config
        let config = HistoryConfig::default();
        let manager = HistoryManager::new(config).await?;

        match &self.action {
            HistoryAction::Trends { hours, format } => {
                let window = Duration::from_secs(hours * 3600);
                let trends = manager.get_trends(window).await?;

                match format {
                    OutputFormat::Json => {
                        let json = serde_json::to_string_pretty(&trends)?;
                        println!("{json}");
                    }
                    OutputFormat::Markdown | OutputFormat::Claude => {
                        println!("# Diagnostic Trends (Last {hours} hours)\n");
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
                let hot_spots = manager.get_hot_spots(*limit).await?;

                match format {
                    OutputFormat::Json => {
                        let json = serde_json::to_string_pretty(&hot_spots)?;
                        println!("{json}");
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
                let validated_path = validate_path(path)?;
                let window = Duration::from_secs(hours * 3600);
                let report = manager.get_file_trends(&validated_path, window).await?;

                match format {
                    OutputFormat::Json => {
                        let json = serde_json::to_string_pretty(&report)?;
                        println!("{json}");
                    }
                    OutputFormat::Markdown | OutputFormat::Claude => {
                        println!("# File History: {}\n", validated_path.display());
                        println!("**Time Period**: Last {hours} hours");
                        println!("**Trend Direction**: {:?}", report.trend_direction);
                        println!("**Volatility**: {:.2}", report.volatility);
                        
                        // Show current counts from the trends
                        if let Some((_, last_errors)) = report.error_trend.last() {
                            println!("**Current Error Count**: {last_errors}");
                        }
                        if let Some((_, last_warnings)) = report.warning_trend.last() {
                            println!("**Current Warning Count**: {last_warnings}");
                        }
                        
                        // Show predictions
                        println!("\n## Predictions");
                        println!("**Next Hour Errors**: {}", report.predictions.next_hour_errors);
                        println!("**Next Hour Warnings**: {}", report.predictions.next_hour_warnings);
                        println!("**Confidence**: {:.1}%", report.predictions.confidence * 100.0);
                        println!("**Suggested Action**: {}", report.predictions.suggested_action);
                        
                        // Show trend data
                        if !report.error_trend.is_empty() || !report.warning_trend.is_empty() {
                            println!("\n## Trends");
                            println!("**Error trend points**: {}", report.error_trend.len());
                            println!("**Warning trend points**: {}", report.warning_trend.len());
                        }
                    }
                }
            }

            HistoryAction::Clean { older_than_days } => {
                let cutoff_date = chrono::Utc::now() - chrono::Duration::days(*older_than_days as i64);
                let deleted_count = manager.clean_old_data(cutoff_date).await?;
                println!(
                    "✅ Cleaned {deleted_count} old diagnostic entries (older than {older_than_days} days)"
                );
            }
        }

        Ok(())
    }
}