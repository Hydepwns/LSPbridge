use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::path::PathBuf;

use crate::cli::args::{QueryArgs, QueryOutputFormat};
use crate::cli::commands::Command;
use crate::core::{DiagnosticResult, DiagnosticSeverity, RawDiagnostics};
use crate::format::FormatConverter;
use crate::query::{InteractiveRepl, QueryApi, QueryResult};

use super::export::{find_ide_diagnostics, read_stdin};

pub struct QueryCommand {
    args: QueryArgs,
}

impl QueryCommand {
    pub fn new(args: QueryArgs) -> Self {
        Self { args }
    }
}

#[async_trait]
impl Command for QueryCommand {
    async fn execute(&self) -> Result<()> {
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

        if self.args.interactive || self.args.query.is_none() {
            // Start interactive REPL
            let mut repl = InteractiveRepl::new().with_diagnostics(processed);

            // Try to add history if available
            let history_config = crate::history::HistoryConfig::default();
            if let Ok(storage) = crate::history::HistoryStorage::new(history_config).await {
                repl = repl.with_history(storage);
            }

            repl.run().await?;
        } else if let Some(query_str) = &self.args.query {
            // Execute single query
            let api = QueryApi::new();
            api.with_diagnostics(processed).await?;

            let result = api.execute(query_str).await?;

            // Format and output result
            let formatted = match self.args.format {
                QueryOutputFormat::Table => format_as_table(&result),
                QueryOutputFormat::Json => serde_json::to_string_pretty(&result)?,
                QueryOutputFormat::Csv => format_as_csv(&result),
            };

            if let Some(output_path) = &self.args.output {
                std::fs::write(output_path, formatted)?;
            } else {
                println!("{}", formatted);
            }
        }

        Ok(())
    }
}

fn format_as_table(result: &QueryResult) -> String {
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
        let _ = write!(&mut output, "{:<width$} ", col, width = widths[i]);
    }
    let _ = writeln!(&mut output);

    // Separator
    for width in &widths {
        let _ = write!(&mut output, "{} ", "-".repeat(*width));
    }
    let _ = writeln!(&mut output);

    // Rows
    for row in &result.rows {
        for (i, value) in row.values.iter().enumerate() {
            let str_val = value.to_string();
            let truncated = if str_val.len() > 50 {
                format!("{}...", &str_val[..47])
            } else {
                str_val
            };
            let _ = write!(&mut output, "{:<width$} ", truncated, width = widths[i]);
        }
        let _ = writeln!(&mut output);
    }

    // Footer
    let _ = writeln!(
        &mut output,
        "\n{} results in {}ms",
        result.total_count, result.query_time_ms
    );

    output
}

fn format_as_csv(result: &QueryResult) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    // Header
    let _ = writeln!(&mut output, "{}", result.columns.join(","));

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
        let _ = writeln!(&mut output, "{}", values.join(","));
    }

    output
}