//! Format converter module for normalizing diagnostics from various sources

pub mod converters;
pub mod factory;
pub mod types;
pub mod utils;

use crate::core::errors::ParseError;
use crate::core::{
    Diagnostic, FormatConverter as FormatConverterTrait, RawDiagnostics,
};
use async_trait::async_trait;
use factory::ConverterFactory;
use serde_json::Value;
use types::SourceType;

/// Main format converter that delegates to specific converters
pub struct FormatConverter {
    factory: ConverterFactory,
}

impl FormatConverter {
    /// Create a new format converter
    pub fn new() -> Self {
        Self {
            factory: ConverterFactory::new(),
        }
    }

    /// Detect source type from diagnostic data
    pub fn detect_source_type(data: &Value) -> String {
        match SourceType::detect(data, "unknown") {
            SourceType::TypeScript => "typescript".to_string(),
            SourceType::RustAnalyzer => "rust-analyzer".to_string(),
            SourceType::ESLint => "eslint".to_string(),
            SourceType::GenericLSP(_) => "lsp-generic".to_string(),
        }
    }
}

#[async_trait]
impl FormatConverterTrait for FormatConverter {
    async fn normalize(&self, raw: RawDiagnostics) -> Result<Vec<Diagnostic>, ParseError> {
        let converter = self.factory.get_converter_by_source(&raw.source);
        converter.convert(&raw).await
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