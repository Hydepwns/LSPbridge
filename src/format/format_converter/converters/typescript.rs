//! TypeScript diagnostic converter

use crate::core::errors::ParseError;
use crate::core::{Diagnostic, Location, RawDiagnostics, RelatedInformation};
use crate::format::format_converter::types::SpecificFormatConverter;
use crate::format::format_converter::utils::{
    generate_id, normalize_file_path, RangeConverter, SeverityConverter,
};
use async_trait::async_trait;
use serde_json::Value;

pub struct TypeScriptConverter;

impl TypeScriptConverter {
    pub fn new() -> Self {
        Self
    }

    fn convert_single_diagnostic(
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
        let range = RangeConverter::convert_typescript(start, end)?;

        let category = d.get("category").and_then(|c| c.as_u64()).unwrap_or(1) as u8;
        let severity = SeverityConverter::convert_typescript(category);

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
            .map(|arr| self.convert_related_info(arr))
            .transpose()?;

        Ok(Diagnostic {
            id: generate_id("ts", index),
            file: normalize_file_path(&file),
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

    fn convert_related_info(
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

            let range = RangeConverter::convert_typescript(info.get("start"), info.get("end"))?;

            let message = info
                .get("messageText")
                .or_else(|| info.get("message"))
                .and_then(|m| m.as_str())
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
impl SpecificFormatConverter for TypeScriptConverter {
    async fn convert(&self, raw: &RawDiagnostics) -> Result<Vec<Diagnostic>, ParseError> {
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
                    found: format!("{diagnostics_array:?}"),
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
        source_lower.contains("typescript") || source_lower.contains("ts")
    }

    fn name(&self) -> &'static str {
        "TypeScript"
    }
}

impl Default for TypeScriptConverter {
    fn default() -> Self {
        Self::new()
    }
}