//! Generic LSP diagnostic converter

use crate::core::errors::ParseError;
use crate::core::{Diagnostic, RawDiagnostics};
use crate::format::format_converter::types::SpecificFormatConverter;
use crate::format::format_converter::utils::{
    generate_id, normalize_file_path, RangeConverter, SeverityConverter,
};
use async_trait::async_trait;
use serde_json::Value;

pub struct GenericLSPConverter;

impl GenericLSPConverter {
    pub fn new() -> Self {
        Self
    }

    fn convert_single_diagnostic(
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

        let range = RangeConverter::convert_lsp(d.get("range"))?;

        let severity_num = d.get("severity").and_then(|s| s.as_u64()).unwrap_or(1) as u8;
        let severity = SeverityConverter::convert_lsp(severity_num);

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
            id: generate_id("generic", index),
            file: normalize_file_path(&file),
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
}

#[async_trait]
impl SpecificFormatConverter for GenericLSPConverter {
    async fn convert(&self, raw: &RawDiagnostics) -> Result<Vec<Diagnostic>, ParseError> {
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
                    found: format!("{diagnostics_array:?}"),
                })?;

        let mut result = Vec::new();

        for (index, d) in diagnostics.iter().enumerate() {
            let diagnostic = self.convert_single_diagnostic(d, &raw.source, index)?;
            result.push(diagnostic);
        }

        Ok(result)
    }

    fn can_handle(&self, _source: &str) -> bool {
        // Generic LSP converter can handle any source
        // but should be used as a fallback
        true
    }

    fn name(&self) -> &'static str {
        "Generic LSP"
    }
}

impl Default for GenericLSPConverter {
    fn default() -> Self {
        Self::new()
    }
}