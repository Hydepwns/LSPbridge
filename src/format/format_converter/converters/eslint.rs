//! ESLint diagnostic converter

use crate::core::errors::ParseError;
use crate::core::{Diagnostic, RawDiagnostics};
use crate::format::format_converter::types::SpecificFormatConverter;
use crate::format::format_converter::utils::{
    generate_id, normalize_file_path, RangeConverter, SeverityConverter,
};
use async_trait::async_trait;
use serde_json::Value;

pub struct ESLintConverter;

impl ESLintConverter {
    pub fn new() -> Self {
        Self
    }

    fn convert_single_diagnostic(
        &self,
        message: &Value,
        file_path: &str,
        index: usize,
    ) -> Result<Diagnostic, ParseError> {
        let range = RangeConverter::convert_eslint(message)?;

        let severity_num = message
            .get("severity")
            .and_then(|s| s.as_u64())
            .unwrap_or(1) as u8;
        let severity = SeverityConverter::convert_eslint(severity_num);

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
            id: generate_id("eslint", index),
            file: normalize_file_path(file_path),
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
}

#[async_trait]
impl SpecificFormatConverter for ESLintConverter {
    async fn convert(&self, raw: &RawDiagnostics) -> Result<Vec<Diagnostic>, ParseError> {
        let results_array = match &raw.data {
            Value::Object(obj) => obj.get("results").unwrap_or(&raw.data),
            _ => &raw.data,
        };

        let results = results_array
            .as_array()
            .ok_or_else(|| ParseError::InvalidFormat {
                context: "ESLint results".to_string(),
                expected: "array of ESLint results".to_string(),
                found: format!("{results_array:?}"),
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
                    self.convert_single_diagnostic(message, &file_path, global_index)?;
                diagnostics.push(diagnostic);
                global_index += 1;
            }
        }

        Ok(diagnostics)
    }

    fn can_handle(&self, source: &str) -> bool {
        source.to_lowercase().contains("eslint")
    }

    fn name(&self) -> &'static str {
        "ESLint"
    }
}

impl Default for ESLintConverter {
    fn default() -> Self {
        Self::new()
    }
}