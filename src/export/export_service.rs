use crate::core::{
    ExportService as ExportServiceTrait, DiagnosticSnapshot, ExportConfig, DiagnosticSummary,
    Diagnostic, DiagnosticSeverity, SortBy
};
use anyhow::Result;
use std::collections::HashMap;

pub struct ExportService;

impl ExportService {
    pub fn new() -> Self {
        Self
    }

    fn sort_diagnostics(&self, diagnostics: &[Diagnostic], sort_by: &SortBy) -> Vec<Diagnostic> {
        let mut sorted = diagnostics.to_vec();
        
        match sort_by {
            SortBy::File => {
                sorted.sort_by(|a, b| {
                    let file_cmp = a.file.cmp(&b.file);
                    if file_cmp == std::cmp::Ordering::Equal {
                        a.range.start.line.cmp(&b.range.start.line)
                    } else {
                        file_cmp
                    }
                });
            }
            SortBy::Source => {
                sorted.sort_by(|a, b| {
                    let source_cmp = a.source.cmp(&b.source);
                    if source_cmp == std::cmp::Ordering::Equal {
                        (a.severity as u8).cmp(&(b.severity as u8))
                    } else {
                        source_cmp
                    }
                });
            }
            SortBy::Timestamp => {
                // For now, maintain original order as we don't have individual timestamps
                // Could be enhanced to sort by snapshot timestamp or add diagnostic timestamps
            }
            SortBy::Severity => {
                sorted.sort_by(|a, b| {
                    let severity_cmp = (a.severity as u8).cmp(&(b.severity as u8));
                    if severity_cmp == std::cmp::Ordering::Equal {
                        a.file.cmp(&b.file)
                    } else {
                        severity_cmp
                    }
                });
            }
        }
        
        sorted
    }

    fn export_markdown_by_severity(&self, lines: &mut Vec<String>, diagnostics: &[Diagnostic], _config: &ExportConfig) {
        let groups = self.group_by_severity(diagnostics);
        
        for (severity_name, group_diagnostics) in &groups {
            if group_diagnostics.is_empty() {
                continue;
            }
            
            lines.push(format!("## {}s", severity_name));
            lines.push(String::new());
            
            for diagnostic in group_diagnostics {
                self.add_markdown_diagnostic(lines, diagnostic, _config);
            }
        }
    }

    fn export_markdown_by_file(&self, lines: &mut Vec<String>, diagnostics: &[Diagnostic], _config: &ExportConfig) {
        let file_groups = self.group_by_file(diagnostics);
        
        for (file, file_diagnostics) in &file_groups {
            lines.push(format!("## {}", file));
            lines.push(String::new());
            
            for diagnostic in file_diagnostics {
                self.add_markdown_diagnostic(lines, diagnostic, _config);
            }
        }
    }

    fn export_claude_optimized_section(&self, lines: &mut Vec<String>, diagnostics: &[Diagnostic], config: &ExportConfig) {
        for diagnostic in diagnostics {
            let location = format!("{}:{}:{}", 
                diagnostic.file, 
                diagnostic.range.start.line + 1, 
                diagnostic.range.start.character + 1
            );
            let code = diagnostic.code.as_ref()
                .map(|c| format!(" ({})", c))
                .unwrap_or_default();
            
            lines.push(format!("### {}", location));
            lines.push(format!("**{}{}**: {}", diagnostic.source, code, diagnostic.message));
            lines.push(String::new());
            
            // Add context if requested and available
            if config.include_context {
                lines.push("```".to_string());
                lines.push("// Context would be added here if file reading is available".to_string());
                lines.push(format!("// Line {}: [diagnostic location]", diagnostic.range.start.line + 1));
                lines.push("```".to_string());
                lines.push(String::new());
            }
        }
    }

    fn add_markdown_diagnostic(&self, lines: &mut Vec<String>, diagnostic: &Diagnostic, _config: &ExportConfig) {
        let location = format!("{}:{}:{}", 
            diagnostic.file, 
            diagnostic.range.start.line + 1, 
            diagnostic.range.start.character + 1
        );
        let severity_icon = self.get_severity_icon(diagnostic.severity);
        let code = diagnostic.code.as_ref()
            .map(|c| format!(" ({})", c))
            .unwrap_or_default();
        
        lines.push(format!("### {} {}", severity_icon, location));
        lines.push(format!("**{}{}**: {}", diagnostic.source, code, diagnostic.message));
        
        if let Some(related_info) = &diagnostic.related_information {
            if !related_info.is_empty() {
                lines.push(String::new());
                lines.push("**Related:**".to_string());
                for info in related_info {
                    let related_location = format!("{}:{}", 
                        info.location.uri, 
                        info.location.range.start.line + 1
                    );
                    lines.push(format!("- {}: {}", related_location, info.message));
                }
            }
        }
        
        lines.push(String::new());
    }

    fn group_by_severity<'a>(&self, diagnostics: &'a [Diagnostic]) -> Vec<(String, Vec<&'a Diagnostic>)> {
        let mut groups = vec![
            ("Error".to_string(), Vec::new()),
            ("Warning".to_string(), Vec::new()),
            ("Info".to_string(), Vec::new()),
            ("Hint".to_string(), Vec::new()),
        ];
        
        for diagnostic in diagnostics {
            match diagnostic.severity {
                DiagnosticSeverity::Error => groups[0].1.push(diagnostic),
                DiagnosticSeverity::Warning => groups[1].1.push(diagnostic),
                DiagnosticSeverity::Information => groups[2].1.push(diagnostic),
                DiagnosticSeverity::Hint => groups[3].1.push(diagnostic),
            }
        }
        
        groups
    }

    fn group_by_file<'a>(&self, diagnostics: &'a [Diagnostic]) -> HashMap<String, Vec<&'a Diagnostic>> {
        let mut groups: HashMap<String, Vec<&'a Diagnostic>> = HashMap::new();
        
        for diagnostic in diagnostics {
            groups.entry(diagnostic.file.clone())
                .or_insert_with(Vec::new)
                .push(diagnostic);
        }
        
        groups
    }

    fn get_severity_icon(&self, severity: DiagnosticSeverity) -> &'static str {
        match severity {
            DiagnosticSeverity::Error => "❌",
            DiagnosticSeverity::Warning => "⚠️",
            DiagnosticSeverity::Information => "ℹ️",
            DiagnosticSeverity::Hint => "💡",
        }
    }
}

impl ExportServiceTrait for ExportService {
    fn export_to_json(&self, snapshot: &DiagnosticSnapshot, config: &ExportConfig) -> Result<String> {
        let sorted_diagnostics = self.sort_diagnostics(&snapshot.diagnostics, &config.sort_by);
        
        let mut export_data = serde_json::json!({
            "timestamp": snapshot.timestamp,
            "workspace": snapshot.workspace,
            "diagnostics": sorted_diagnostics,
            "metadata": snapshot.metadata
        });

        if config.include_summary {
            export_data["summary"] = serde_json::to_value(self.generate_summary(&snapshot.diagnostics))?;
        }

        Ok(serde_json::to_string_pretty(&export_data)?)
    }

    fn export_to_markdown(&self, snapshot: &DiagnosticSnapshot, config: &ExportConfig) -> Result<String> {
        let mut lines = Vec::new();
        let summary = self.generate_summary(&snapshot.diagnostics);
        let sorted_diagnostics = self.sort_diagnostics(&snapshot.diagnostics, &config.sort_by);

        // Header
        lines.push(format!("# Diagnostics Report - {}", snapshot.workspace.name));
        lines.push(String::new());
        lines.push(format!("Generated: {}", snapshot.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
        lines.push(String::new());

        // Summary
        if config.include_summary {
            lines.push("## Summary".to_string());
            lines.push(format!("- **Total Diagnostics**: {}", summary.total_diagnostics));
            lines.push(format!("- **Errors**: {}", summary.error_count));
            lines.push(format!("- **Warnings**: {}", summary.warning_count));
            lines.push(format!("- **Info**: {}", summary.info_count));
            lines.push(format!("- **Hints**: {}", summary.hint_count));
            lines.push(format!("- **Files Affected**: {}", summary.file_count));
            lines.push(String::new());
        }

        // Group by severity or file
        if config.group_by_file {
            self.export_markdown_by_file(&mut lines, &sorted_diagnostics, config);
        } else {
            self.export_markdown_by_severity(&mut lines, &sorted_diagnostics, config);
        }

        Ok(lines.join("\n"))
    }

    fn export_to_claude_optimized(&self, snapshot: &DiagnosticSnapshot, config: &ExportConfig) -> Result<String> {
        let mut lines = Vec::new();
        let summary = self.generate_summary(&snapshot.diagnostics);
        let sorted_diagnostics = self.sort_diagnostics(&snapshot.diagnostics, &config.sort_by);

        // Header optimized for Claude
        lines.push(format!("# Diagnostics Report - {}", snapshot.workspace.name));
        lines.push(String::new());
        lines.push(format!("Generated: {}", snapshot.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
        lines.push(String::new());

        // Summary
        lines.push("## Summary".to_string());
        lines.push(format!("- **Errors**: {}", summary.error_count));
        lines.push(format!("- **Warnings**: {}", summary.warning_count));
        lines.push(format!("- **Info**: {}", summary.info_count));
        lines.push(String::new());

        // Only show errors and warnings for Claude (reduce noise)
        let important_diagnostics: Vec<&Diagnostic> = sorted_diagnostics.iter()
            .filter(|d| matches!(d.severity, DiagnosticSeverity::Error | DiagnosticSeverity::Warning))
            .collect();

        if summary.error_count > 0 {
            lines.push("## Errors".to_string());
            lines.push(String::new());
            let errors: Vec<Diagnostic> = important_diagnostics.iter()
                .filter(|d| d.severity == DiagnosticSeverity::Error)
                .map(|d| (*d).clone())
                .collect();
            self.export_claude_optimized_section(&mut lines, &errors, config);
        }

        if summary.warning_count > 0 {
            lines.push("## Warnings".to_string());
            lines.push(String::new());
            let warnings: Vec<Diagnostic> = important_diagnostics.iter()
                .filter(|d| d.severity == DiagnosticSeverity::Warning)
                .map(|d| (*d).clone())
                .collect();
            self.export_claude_optimized_section(&mut lines, &warnings, config);
        }

        // Add helpful context for Claude
        if !important_diagnostics.is_empty() {
            lines.push("## Context for AI Analysis".to_string());
            lines.push(String::new());
            lines.push("This diagnostic report contains:".to_string());
            
            for (source, count) in &summary.source_breakdown {
                lines.push(format!("- {} diagnostic(s) from {}", count, source));
            }
            lines.push(String::new());
            lines.push("Please analyze these diagnostics and suggest fixes or improvements.".to_string());
        }

        Ok(lines.join("\n"))
    }

    fn generate_summary(&self, diagnostics: &[Diagnostic]) -> DiagnosticSummary {
        let mut summary = DiagnosticSummary {
            total_diagnostics: diagnostics.len(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            hint_count: 0,
            file_count: 0,
            source_breakdown: HashMap::new(),
        };

        let mut files = std::collections::HashSet::new();
        
        for diagnostic in diagnostics {
            files.insert(&diagnostic.file);
            
            match diagnostic.severity {
                DiagnosticSeverity::Error => summary.error_count += 1,
                DiagnosticSeverity::Warning => summary.warning_count += 1,
                DiagnosticSeverity::Information => summary.info_count += 1,
                DiagnosticSeverity::Hint => summary.hint_count += 1,
            }

            *summary.source_breakdown
                .entry(diagnostic.source.clone())
                .or_insert(0) += 1;
        }

        summary.file_count = files.len();
        summary
    }
}

impl Default for ExportService {
    fn default() -> Self {
        Self::new()
    }
}