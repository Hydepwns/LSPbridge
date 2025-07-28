use crate::core::errors::ParseError;
use crate::core::{
    Diagnostic, DiagnosticSeverity, FormatConverter as FormatConverterTrait, Location, Position,
    Range, RawDiagnostics, RelatedInformation,
};
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

pub struct FormatConverter;

impl FormatConverter {
    pub fn new() -> Self {
        Self
    }

    fn convert_typescript(&self, raw: &RawDiagnostics) -> Result<Vec<Diagnostic>, ParseError> {
        let diagnostics_array = match &raw.data {
            Value::Object(obj) => obj.get("diagnostics").unwrap_or(&raw.data),
            _ => &raw.data,
        };

        let diagnostics =
            diagnostics_array
                .as_array()
                .ok_or_else(|| ParseError::InvalidFormat {
                    context: "TypeScript diagnostics".to_string(),
                    expected: "array of diagnostics".to_string(),
                    found: format!("{:?}", diagnostics_array),
                })?;

        let mut result = Vec::new();

        for (index, d) in diagnostics.iter().enumerate() {
            let diagnostic = self.convert_single_typescript_diagnostic(d, index)?;
            result.push(diagnostic);
        }

        Ok(result)
    }

    fn convert_single_typescript_diagnostic(
        &self,
        d: &Value,
        index: usize,
    ) -> Result<Diagnostic, ParseError> {
        let file = d
            .get("file")
            .or_else(|| d.get("fileName"))
            .and_then(|f| f.as_str())
            .unwrap_or("")
            .to_string();

        let start = d.get("start");
        let end = d.get("end");
        let range = self.convert_ts_range(start, end)?;

        let category = d.get("category").and_then(|c| c.as_u64()).unwrap_or(1) as u8;
        let severity = self.convert_ts_severity(category);

        let message = d
            .get("messageText")
            .or_else(|| d.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        let code = d
            .get("code")
            .and_then(|c| {
                c.as_str().or_else(|| {
                    c.as_u64()
                        .map(|n| Box::leak(n.to_string().into_boxed_str()) as &str)
                })
            })
            .map(|s| s.to_string());

        let related_information = d
            .get("relatedInformation")
            .and_then(|r| r.as_array())
            .map(|arr| self.convert_ts_related_info(arr))
            .transpose()?;

        Ok(Diagnostic {
            id: self.generate_id("ts", index),
            file: self.normalize_file_path(&file),
            range,
            severity,
            message,
            code,
            source: "typescript".to_string(),
            related_information,
            tags: None,
            data: None,
        })
    }

    fn convert_rust_analyzer(&self, raw: &RawDiagnostics) -> Result<Vec<Diagnostic>, ParseError> {
        let diagnostics_array = match &raw.data {
            Value::Object(obj) => obj.get("diagnostics").unwrap_or(&raw.data),
            _ => &raw.data,
        };

        let diagnostics =
            diagnostics_array
                .as_array()
                .ok_or_else(|| ParseError::InvalidFormat {
                    context: "Rust analyzer diagnostics".to_string(),
                    expected: "array of diagnostics".to_string(),
                    found: format!("{:?}", diagnostics_array),
                })?;

        let mut result = Vec::new();

        for (index, d) in diagnostics.iter().enumerate() {
            let diagnostic = self.convert_single_rust_diagnostic(d, index)?;
            result.push(diagnostic);
        }

        Ok(result)
    }

    fn convert_single_rust_diagnostic(
        &self,
        d: &Value,
        index: usize,
    ) -> Result<Diagnostic, ParseError> {
        let spans =
            d.get("spans")
                .and_then(|s| s.as_array())
                .ok_or_else(|| ParseError::InvalidFormat {
                    context: "Rust diagnostic".to_string(),
                    expected: "spans array".to_string(),
                    found: "missing spans".to_string(),
                })?;

        let main_span = spans.first().ok_or_else(|| ParseError::InvalidFormat {
            context: "Rust diagnostic spans".to_string(),
            expected: "at least one span".to_string(),
            found: "empty spans array".to_string(),
        })?;

        let file = main_span
            .get("file_name")
            .and_then(|f| f.as_str())
            .unwrap_or("")
            .to_string();

        let range = self.convert_rust_range(main_span)?;

        let level = d.get("level").and_then(|l| l.as_str()).unwrap_or("error");
        let severity = self.convert_rust_severity(level);

        let message = d
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        let code = d
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        // Convert additional spans to related information
        let related_information = if spans.len() > 1 {
            Some(self.convert_rust_related_spans(&spans[1..])?)
        } else {
            None
        };

        Ok(Diagnostic {
            id: self.generate_id("rust", index),
            file: self.normalize_file_path(&file),
            range,
            severity,
            message,
            code,
            source: "rust-analyzer".to_string(),
            related_information,
            tags: None,
            data: None,
        })
    }

    fn convert_eslint(&self, raw: &RawDiagnostics) -> Result<Vec<Diagnostic>, ParseError> {
        let results_array = match &raw.data {
            Value::Object(obj) => obj.get("results").unwrap_or(&raw.data),
            _ => &raw.data,
        };

        let results = results_array
            .as_array()
            .ok_or_else(|| ParseError::InvalidFormat {
                context: "ESLint results".to_string(),
                expected: "array of ESLint results".to_string(),
                found: format!("{:?}", results_array),
            })?;

        let mut diagnostics = Vec::new();
        let mut global_index = 0;

        for result in results {
            let file_path = result
                .get("filePath")
                .and_then(|f| f.as_str())
                .unwrap_or("")
                .to_string();

            let empty_vec = vec![];
            let messages = result
                .get("messages")
                .and_then(|m| m.as_array())
                .unwrap_or(&empty_vec);

            for message in messages {
                let diagnostic =
                    self.convert_single_eslint_diagnostic(message, &file_path, global_index)?;
                diagnostics.push(diagnostic);
                global_index += 1;
            }
        }

        Ok(diagnostics)
    }

    fn convert_single_eslint_diagnostic(
        &self,
        message: &Value,
        file_path: &str,
        index: usize,
    ) -> Result<Diagnostic, ParseError> {
        let range = self.convert_eslint_range(message)?;

        let severity_num = message
            .get("severity")
            .and_then(|s| s.as_u64())
            .unwrap_or(1) as u8;
        let severity = self.convert_eslint_severity(severity_num);

        let message_text = message
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        let rule_id = message
            .get("ruleId")
            .and_then(|r| r.as_str())
            .map(|s| s.to_string());

        Ok(Diagnostic {
            id: self.generate_id("eslint", index),
            file: self.normalize_file_path(file_path),
            range,
            severity,
            message: message_text,
            code: rule_id,
            source: "eslint".to_string(),
            related_information: None,
            tags: None,
            data: None,
        })
    }

    fn convert_generic_lsp(&self, raw: &RawDiagnostics) -> Result<Vec<Diagnostic>, ParseError> {
        let diagnostics_array = match &raw.data {
            Value::Object(obj) => obj.get("diagnostics").unwrap_or(&raw.data),
            _ => &raw.data,
        };

        let diagnostics =
            diagnostics_array
                .as_array()
                .ok_or_else(|| ParseError::InvalidFormat {
                    context: "Generic LSP diagnostics".to_string(),
                    expected: "array of diagnostics".to_string(),
                    found: format!("{:?}", diagnostics_array),
                })?;

        let mut result = Vec::new();

        for (index, d) in diagnostics.iter().enumerate() {
            let diagnostic = self.convert_single_generic_diagnostic(d, &raw.source, index)?;
            result.push(diagnostic);
        }

        Ok(result)
    }

    fn convert_single_generic_diagnostic(
        &self,
        d: &Value,
        source: &str,
        index: usize,
    ) -> Result<Diagnostic, ParseError> {
        let file = d
            .get("uri")
            .or_else(|| d.get("source"))
            .or_else(|| d.get("file"))
            .and_then(|f| f.as_str())
            .unwrap_or("")
            .to_string();

        let range = self.convert_lsp_range(d.get("range"))?;

        let severity_num = d.get("severity").and_then(|s| s.as_u64()).unwrap_or(1) as u8;
        let severity = self.convert_lsp_severity(severity_num);

        let message = d
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        let code = d
            .get("code")
            .and_then(|c| {
                c.as_str().or_else(|| {
                    c.as_u64()
                        .map(|n| Box::leak(n.to_string().into_boxed_str()) as &str)
                })
            })
            .map(|s| s.to_string());

        Ok(Diagnostic {
            id: self.generate_id("generic", index),
            file: self.normalize_file_path(&file),
            range,
            severity,
            message,
            code,
            source: source.to_string(),
            related_information: None,
            tags: None,
            data: None,
        })
    }

    // Range conversion helpers
    fn convert_ts_range(
        &self,
        start: Option<&Value>,
        end: Option<&Value>,
    ) -> Result<Range, ParseError> {
        let start_pos = match start {
            Some(s) => Position {
                line: s.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32,
                character: s.get("character").and_then(|c| c.as_u64()).unwrap_or(0) as u32,
            },
            None => Position {
                line: 0,
                character: 0,
            },
        };

        let end_pos = match end {
            Some(e) => Position {
                line: e.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32,
                character: e.get("character").and_then(|c| c.as_u64()).unwrap_or(0) as u32,
            },
            None => start_pos.clone(),
        };

        Ok(Range {
            start: start_pos,
            end: end_pos,
        })
    }

    fn convert_rust_range(&self, span: &Value) -> Result<Range, ParseError> {
        let line_start = span.get("line_start").and_then(|l| l.as_u64()).unwrap_or(1) as u32;
        let line_end = span
            .get("line_end")
            .and_then(|l| l.as_u64())
            .unwrap_or(line_start as u64) as u32;
        let column_start = span
            .get("column_start")
            .and_then(|c| c.as_u64())
            .unwrap_or(1) as u32;
        let column_end = span
            .get("column_end")
            .and_then(|c| c.as_u64())
            .unwrap_or(column_start as u64) as u32;

        Ok(Range {
            start: Position {
                line: line_start.saturating_sub(1), // Rust uses 1-based lines
                character: column_start.saturating_sub(1),
            },
            end: Position {
                line: line_end.saturating_sub(1),
                character: column_end.saturating_sub(1),
            },
        })
    }

    fn convert_eslint_range(&self, message: &Value) -> Result<Range, ParseError> {
        let line = message.get("line").and_then(|l| l.as_u64()).unwrap_or(1) as u32;
        let column = message.get("column").and_then(|c| c.as_u64()).unwrap_or(1) as u32;
        let end_line = message
            .get("endLine")
            .and_then(|l| l.as_u64())
            .unwrap_or(line as u64) as u32;
        let end_column = message
            .get("endColumn")
            .and_then(|c| c.as_u64())
            .unwrap_or(column as u64) as u32;

        Ok(Range {
            start: Position {
                line: line.saturating_sub(1), // ESLint uses 1-based lines
                character: column.saturating_sub(1),
            },
            end: Position {
                line: end_line.saturating_sub(1),
                character: end_column.saturating_sub(1),
            },
        })
    }

    fn convert_lsp_range(&self, range: Option<&Value>) -> Result<Range, ParseError> {
        let range = range.ok_or_else(|| ParseError::InvalidFormat {
            context: "LSP diagnostic".to_string(),
            expected: "range object".to_string(),
            found: "missing range".to_string(),
        })?;

        let start = range
            .get("start")
            .ok_or_else(|| ParseError::InvalidFormat {
                context: "LSP range".to_string(),
                expected: "start position".to_string(),
                found: "missing start".to_string(),
            })?;
        let end = range.get("end").ok_or_else(|| ParseError::InvalidFormat {
            context: "LSP range".to_string(),
            expected: "end position".to_string(),
            found: "missing end".to_string(),
        })?;

        let start_pos = Position {
            line: start.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32,
            character: start.get("character").and_then(|c| c.as_u64()).unwrap_or(0) as u32,
        };

        let end_pos = Position {
            line: end.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32,
            character: end.get("character").and_then(|c| c.as_u64()).unwrap_or(0) as u32,
        };

        Ok(Range {
            start: start_pos,
            end: end_pos,
        })
    }

    // Severity conversion helpers
    fn convert_ts_severity(&self, category: u8) -> DiagnosticSeverity {
        // TypeScript uses: 0=message, 1=error, 2=warning, 3=suggestion
        match category {
            0 => DiagnosticSeverity::Information,
            1 => DiagnosticSeverity::Error,
            2 => DiagnosticSeverity::Warning,
            3 => DiagnosticSeverity::Hint,
            _ => DiagnosticSeverity::Error,
        }
    }

    fn convert_rust_severity(&self, level: &str) -> DiagnosticSeverity {
        match level.to_lowercase().as_str() {
            "error" => DiagnosticSeverity::Error,
            "warning" => DiagnosticSeverity::Warning,
            "note" => DiagnosticSeverity::Information,
            "help" => DiagnosticSeverity::Hint,
            _ => DiagnosticSeverity::Error,
        }
    }

    fn convert_eslint_severity(&self, severity: u8) -> DiagnosticSeverity {
        match severity {
            1 => DiagnosticSeverity::Warning,
            2 => DiagnosticSeverity::Error,
            _ => DiagnosticSeverity::Warning,
        }
    }

    fn convert_lsp_severity(&self, severity: u8) -> DiagnosticSeverity {
        // LSP standard: 1=Error, 2=Warning, 3=Information, 4=Hint
        match severity {
            1 => DiagnosticSeverity::Error,
            2 => DiagnosticSeverity::Warning,
            3 => DiagnosticSeverity::Information,
            4 => DiagnosticSeverity::Hint,
            _ => DiagnosticSeverity::Error,
        }
    }

    // Helper methods
    fn convert_ts_related_info(
        &self,
        related: &[Value],
    ) -> Result<Vec<RelatedInformation>, ParseError> {
        let mut result = Vec::new();

        for info in related {
            let file = info
                .get("file")
                .and_then(|f| f.get("fileName"))
                .and_then(|f| f.as_str())
                .unwrap_or("")
                .to_string();

            let range = self.convert_ts_range(info.get("start"), info.get("end"))?;

            let message = info
                .get("messageText")
                .or_else(|| info.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();

            result.push(RelatedInformation {
                location: Location {
                    uri: self.normalize_file_path(&file),
                    range,
                },
                message,
            });
        }

        Ok(result)
    }

    fn convert_rust_related_spans(
        &self,
        spans: &[Value],
    ) -> Result<Vec<RelatedInformation>, ParseError> {
        let mut result = Vec::new();

        for span in spans {
            let file = span
                .get("file_name")
                .and_then(|f| f.as_str())
                .unwrap_or("")
                .to_string();

            let range = self.convert_rust_range(span)?;

            let message = span
                .get("label")
                .and_then(|l| l.as_str())
                .unwrap_or("")
                .to_string();

            result.push(RelatedInformation {
                location: Location {
                    uri: self.normalize_file_path(&file),
                    range,
                },
                message,
            });
        }

        Ok(result)
    }

    fn normalize_file_path(&self, file_path: &str) -> String {
        if file_path.is_empty() {
            return file_path.to_string();
        }

        let mut path = file_path.to_string();

        // Remove file:// prefix if present
        if path.starts_with("file://") {
            path = path[7..].to_string();
        }

        // Normalize path separators
        path = path.replace('\\', "/");

        path
    }

    fn generate_id(&self, source: &str, _index: usize) -> String {
        format!("{}_{}", source, Uuid::new_v4())
    }

    pub fn detect_source_type(data: &Value) -> String {
        if let Some(obj) = data.as_object() {
            if obj.contains_key("diagnostics") {
                if let Some(first) = obj["diagnostics"].as_array().and_then(|arr| arr.first()) {
                    if first.get("code").is_some() && first.get("category").is_some() {
                        return "typescript".to_string();
                    }
                    if first.get("level").is_some() && first.get("spans").is_some() {
                        return "rust-analyzer".to_string();
                    }
                    if first.get("severity").is_some() && first.get("range").is_some() {
                        return "lsp-generic".to_string();
                    }
                }
            }

            if obj.contains_key("results") {
                return "eslint".to_string();
            }
        }

        "unknown".to_string()
    }
}

#[async_trait]
impl FormatConverterTrait for FormatConverter {
    async fn normalize(&self, raw: RawDiagnostics) -> Result<Vec<Diagnostic>, ParseError> {
        let source = raw.source.to_lowercase();

        if source.contains("typescript") || source.contains("ts") {
            self.convert_typescript(&raw)
        } else if source.contains("rust") || source.contains("analyzer") {
            self.convert_rust_analyzer(&raw)
        } else if source.contains("eslint") {
            self.convert_eslint(&raw)
        } else if source.contains("python")
            || source.contains("pylsp")
            || source.contains("pyright")
        {
            self.convert_generic_lsp(&raw)
        } else if source.contains("go") || source.contains("gopls") {
            self.convert_generic_lsp(&raw)
        } else if source.contains("java") || source.contains("jdtls") {
            self.convert_generic_lsp(&raw)
        } else {
            self.convert_generic_lsp(&raw)
        }
    }

    fn convert_to_unified(
        &self,
        diagnostics: Value,
        source: &str,
    ) -> Result<Vec<Diagnostic>, ParseError> {
        let raw = RawDiagnostics {
            source: source.to_string(),
            data: diagnostics,
            timestamp: chrono::Utc::now(),
            workspace: None,
        };

        tokio::runtime::Handle::current().block_on(self.normalize(raw))
    }
}

impl Default for FormatConverter {
    fn default() -> Self {
        Self::new()
    }
}
