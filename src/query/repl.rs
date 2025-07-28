use super::executor::Value;
use super::{QueryExecutor, QueryParser, QueryResult};
use crate::core::DiagnosticResult;
use crate::history::HistoryStorage;
use anyhow::Result;
use colored::*;
use crossterm::{
    cursor, execute,
    terminal::{self, ClearType},
};
use std::collections::VecDeque;
use std::io::{self, Write};

pub struct InteractiveRepl {
    parser: QueryParser,
    executor: QueryExecutor,
    history: VecDeque<String>,
    saved_queries: Vec<SavedQuery>,
    current_input: String,
    history_index: Option<usize>,
}

#[derive(Clone)]
struct SavedQuery {
    name: String,
    query: String,
    description: String,
}

impl InteractiveRepl {
    pub fn new() -> Self {
        Self {
            parser: QueryParser::new(),
            executor: QueryExecutor::new(),
            history: VecDeque::with_capacity(100),
            saved_queries: Self::default_saved_queries(),
            current_input: String::new(),
            history_index: None,
        }
    }

    pub fn with_diagnostics(mut self, diagnostics: DiagnosticResult) -> Self {
        self.executor.with_diagnostics(diagnostics);
        self
    }

    pub fn with_history(mut self, history: HistoryStorage) -> Self {
        self.executor.with_history(history);
        self
    }

    pub async fn run(&mut self) -> Result<()> {
        self.print_welcome();

        loop {
            self.print_prompt()?;

            let input = self.read_input()?;
            if input.trim().is_empty() {
                continue;
            }

            // Check for special commands
            match input.trim() {
                "exit" | "quit" | "\\q" => break,
                "help" | "\\h" | "?" => {
                    self.print_help();
                    continue;
                }
                "examples" | "\\e" => {
                    self.print_examples();
                    continue;
                }
                "saved" | "\\s" => {
                    self.print_saved_queries();
                    continue;
                }
                "clear" | "\\c" => {
                    self.clear_screen()?;
                    continue;
                }
                cmd if cmd.starts_with("\\r ") => {
                    let name = cmd[3..].trim();
                    self.run_saved_query(name).await?;
                    continue;
                }
                _ => {}
            }

            // Add to history
            self.add_to_history(input.clone());

            // Parse and execute query
            match self.parser.parse(&input) {
                Ok(query) => match self.executor.execute(&query).await {
                    Ok(result) => {
                        self.display_result(&result)?;
                    }
                    Err(e) => {
                        eprintln!("{}: {}", "Error executing query".red(), e);
                    }
                },
                Err(e) => {
                    eprintln!("{}: {}", "Parse error".red(), e);
                    self.suggest_fix(&input);
                }
            }
        }

        println!("\n{}", "Goodbye!".green());
        Ok(())
    }

    fn print_welcome(&self) {
        println!(
            "{}",
            "╔═══════════════════════════════════════════════════════╗".blue()
        );
        println!(
            "{}",
            "║       LSP Bridge Interactive Diagnostic Explorer      ║".blue()
        );
        println!(
            "{}",
            "╚═══════════════════════════════════════════════════════╝".blue()
        );
        println!();
        println!(
            "Type {} for help, {} to see examples, {} to quit",
            "help".yellow(),
            "examples".yellow(),
            "exit".yellow()
        );
        println!();
    }

    fn print_prompt(&self) -> Result<()> {
        print!("{} ", "lsp>".green().bold());
        io::stdout().flush()?;
        Ok(())
    }

    fn read_input(&mut self) -> Result<String> {
        // Simple line reading for now
        // In a full implementation, this would support readline-like features
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    fn add_to_history(&mut self, input: String) {
        if self.history.front() != Some(&input) {
            self.history.push_front(input);
            if self.history.len() > 100 {
                self.history.pop_back();
            }
        }
        self.history_index = None;
    }

    fn print_help(&self) {
        println!("\n{}", "=== Query Language Help ===".cyan().bold());
        println!();
        println!("{}", "Basic Syntax:".yellow());
        println!(
            "  SELECT <fields> FROM <source> WHERE <conditions> [ORDER BY <field>] [LIMIT <n>]"
        );
        println!();
        println!("{}", "Data Sources:".yellow());
        println!("  • {} - Current diagnostic results", "diagnostics".green());
        println!("  • {} - File-level statistics", "files".green());
        println!("  • {} - Historical diagnostic data", "history".green());
        println!("  • {} - Trend analysis", "trends".green());
        println!();
        println!("{}", "Common Fields:".yellow());
        println!("  • file, path - File path");
        println!("  • line, column - Position in file");
        println!("  • severity - error, warning, info, hint");
        println!("  • category - Diagnostic category");
        println!("  • message - Diagnostic message");
        println!();
        println!("{}", "Operators:".yellow());
        println!("  • = (equal), != (not equal)");
        println!("  • >, <, >=, <= (comparison)");
        println!("  • LIKE (pattern matching)");
        println!("  • IN (list membership)");
        println!();
        println!("{}", "Special Commands:".yellow());
        println!("  • {} - Show this help", "help, \\h".green());
        println!("  • {} - Show example queries", "examples, \\e".green());
        println!("  • {} - List saved queries", "saved, \\s".green());
        println!("  • {} - Run saved query", "\\r <name>".green());
        println!("  • {} - Clear screen", "clear, \\c".green());
        println!("  • {} - Exit REPL", "exit, quit, \\q".green());
        println!();
    }

    fn print_examples(&self) {
        println!("\n{}", "=== Example Queries ===".cyan().bold());
        println!();

        let examples = vec![
            (
                "All errors",
                "SELECT * FROM diagnostics WHERE severity = error",
            ),
            (
                "Count by severity",
                "SELECT severity, COUNT(*) FROM diagnostics GROUP BY severity",
            ),
            (
                "Type errors in src/",
                "SELECT * FROM diagnostics WHERE category = \"type\" AND path LIKE \"src/%\"",
            ),
            (
                "Top 10 problem files",
                "SELECT file, COUNT(*) FROM diagnostics GROUP BY file ORDER BY count DESC LIMIT 10",
            ),
            (
                "Recent errors",
                "SELECT * FROM history WHERE severity = error AND time > last 7 days",
            ),
            (
                "Error trends",
                "SELECT * FROM trends WHERE metric = \"error_velocity\"",
            ),
        ];

        for (desc, query) in examples {
            println!("{}: {}", desc.yellow(), query.green());
        }
        println!();
    }

    fn print_saved_queries(&self) {
        println!("\n{}", "=== Saved Queries ===".cyan().bold());
        println!();

        for query in &self.saved_queries {
            println!("{}: {}", query.name.yellow(), query.description);
            println!("  {}", query.query.green());
            println!();
        }

        println!("Run with: {} {}", "\\r".green(), "<name>".yellow());
    }

    fn display_result(&self, result: &QueryResult) -> Result<()> {
        println!();

        // Display metadata
        println!(
            "{}",
            format!(
                "Found {} results in {}ms",
                result.total_count, result.query_time_ms
            )
            .dimmed()
        );

        if result.metadata.cache_hit {
            println!("{}", "(cached result)".dimmed());
        }

        println!();

        if result.rows.is_empty() {
            println!("{}", "No results found.".yellow());
            return Ok(());
        }

        // Calculate column widths
        let mut column_widths = vec![0; result.columns.len()];
        for (i, col) in result.columns.iter().enumerate() {
            column_widths[i] = col.len();
        }

        for row in &result.rows {
            for (i, value) in row.values.iter().enumerate() {
                let str_val = value.to_string();
                column_widths[i] = column_widths[i].max(str_val.len().min(50));
            }
        }

        // Print header
        let mut header = String::new();
        for (i, col) in result.columns.iter().enumerate() {
            header.push_str(&format!("{:<width$} ", col, width = column_widths[i]));
        }
        println!("{}", header.bold());

        // Print separator
        let mut separator = String::new();
        for width in &column_widths {
            separator.push_str(&"─".repeat(*width));
            separator.push(' ');
        }
        println!("{}", separator.dimmed());

        // Print rows (limit display to 20 rows)
        let display_limit = 20;
        for (idx, row) in result.rows.iter().take(display_limit).enumerate() {
            let mut row_str = String::new();
            for (i, value) in row.values.iter().enumerate() {
                let str_val = value.to_string();
                let truncated = if str_val.len() > 50 {
                    format!("{}...", &str_val[..47])
                } else {
                    str_val
                };

                let colored_val = match value {
                    Value::Severity(sev) => match sev {
                        crate::core::DiagnosticSeverity::Error => truncated.red().to_string(),
                        crate::core::DiagnosticSeverity::Warning => truncated.yellow().to_string(),
                        crate::core::DiagnosticSeverity::Information => {
                            truncated.blue().to_string()
                        }
                        crate::core::DiagnosticSeverity::Hint => truncated.dimmed().to_string(),
                    },
                    Value::Path(_) => truncated.cyan().to_string(),
                    _ => truncated,
                };

                row_str.push_str(&format!(
                    "{:<width$} ",
                    colored_val,
                    width = column_widths[i]
                ));
            }
            println!("{}", row_str);
        }

        if result.rows.len() > display_limit {
            println!(
                "\n{}",
                format!("... and {} more rows", result.rows.len() - display_limit).dimmed()
            );
        }

        println!();
        Ok(())
    }

    fn suggest_fix(&self, input: &str) {
        // Simple suggestion system
        let lower = input.to_lowercase();

        if !lower.contains("from") {
            println!("{}: Did you forget the FROM clause?", "Hint".yellow());
        } else if !lower.contains("select") {
            println!("{}: Queries should start with SELECT", "Hint".yellow());
        } else if lower.contains("=") && !lower.contains("where") {
            println!(
                "{}: Conditions should be in a WHERE clause",
                "Hint".yellow()
            );
        }
    }

    async fn run_saved_query(&mut self, name: &str) -> Result<()> {
        if let Some(saved) = self.saved_queries.iter().find(|q| q.name == name) {
            println!("{}: {}", "Running".dimmed(), saved.query.green());

            match self.parser.parse(&saved.query) {
                Ok(query) => match self.executor.execute(&query).await {
                    Ok(result) => {
                        self.display_result(&result)?;
                    }
                    Err(e) => {
                        eprintln!("{}: {}", "Error executing query".red(), e);
                    }
                },
                Err(e) => {
                    eprintln!("{}: {}", "Parse error".red(), e);
                }
            }
        } else {
            eprintln!("{}: Unknown saved query '{}'", "Error".red(), name);
            println!("Available queries:");
            for query in &self.saved_queries {
                println!("  • {}", query.name.yellow());
            }
        }

        Ok(())
    }

    fn clear_screen(&self) -> Result<()> {
        execute!(
            io::stdout(),
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        self.print_welcome();
        Ok(())
    }

    fn default_saved_queries() -> Vec<SavedQuery> {
        vec![
            SavedQuery {
                name: "errors".to_string(),
                query: "SELECT * FROM diagnostics WHERE severity = error".to_string(),
                description: "Show all errors".to_string(),
            },
            SavedQuery {
                name: "hot-files".to_string(),
                query: "SELECT file, COUNT(*) as count FROM diagnostics GROUP BY file ORDER BY count DESC LIMIT 10".to_string(),
                description: "Top 10 files with most diagnostics".to_string(),
            },
            SavedQuery {
                name: "summary".to_string(),
                query: "SELECT severity, COUNT(*) FROM diagnostics GROUP BY severity".to_string(),
                description: "Summary by severity".to_string(),
            },
            SavedQuery {
                name: "type-errors".to_string(),
                query: "SELECT * FROM diagnostics WHERE category = \"type\" AND severity = error".to_string(),
                description: "All type errors".to_string(),
            },
            SavedQuery {
                name: "recent".to_string(),
                query: "SELECT * FROM history WHERE time > last 1 hours ORDER BY time DESC".to_string(),
                description: "Recent diagnostics from last hour".to_string(),
            },
        ]
    }
}
