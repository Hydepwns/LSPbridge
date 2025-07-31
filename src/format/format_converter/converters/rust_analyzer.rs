//! Rust analyzer diagnostic converter

use crate::core::errors::ParseError;
use crate::core::{Diagnostic, Location, RawDiagnostics, RelatedInformation};
use crate::format::format_converter::types::SpecificFormatConverter;
use crate::format::format_converter::utils::{
    generate_id, normalize_file_path, RangeConverter, SeverityConverter,
};
use async_trait::async_trait;
use serde_json::Value;

pub struct RustAnalyzerConverter;

impl RustAnalyzerConverter {
    pub fn new() -> Self {
        Self
    }

    fn convert_single_diagnostic(
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

        let range = RangeConverter::convert_rust(main_span)?;

        let level = d.get("level").and_then(|l| l.as_str()).unwrap_or("error");
        let severity = SeverityConverter::convert_rust(level);

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
            Some(self.convert_related_spans(&spans[1..])?)
        } else {
            None
        };

        Ok(Diagnostic {
            id: generate_id("rust", index),
            file: normalize_file_path(&file),
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

    fn convert_related_spans(
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

            let range = RangeConverter::convert_rust(span)?;

            let message = span
                .get("label")
                .and_then(|l| l.as_str())
                .unwrap_or("")
                .to_string();

            result.push(RelatedInformation {
                location: Location {
                    uri: normalize_file_path(&file),
                    range,
                },
                message,
            });
        }

        Ok(result)
    }
}

#[async_trait]
impl SpecificFormatConverter for RustAnalyzerConverter {
    async fn convert(&self, raw: &RawDiagnostics) -> Result<Vec<Diagnostic>, ParseError> {
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
            let diagnostic = self.convert_single_diagnostic(d, index)?;
            result.push(diagnostic);
        }

        Ok(result)
    }

    fn can_handle(&self, source: &str) -> bool {
        let source_lower = source.to_lowercase();
        source_lower.contains("rust") || source_lower.contains("analyzer")
    }

    fn name(&self) -> &'static str {
        "Rust Analyzer"
    }
}

impl Default for RustAnalyzerConverter {
    fn default() -> Self {
        Self::new()
    }
}