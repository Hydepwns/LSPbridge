//! Factory for creating format-specific converters

use crate::format::format_converter::converters::{
    ESLintConverter, GenericLSPConverter, RustAnalyzerConverter, TypeScriptConverter,
};
use crate::format::format_converter::types::{SourceType, SpecificFormatConverter};
use serde_json::Value;
use std::sync::Arc;

/// Factory for creating appropriate format converters
pub struct ConverterFactory {
    typescript_converter: Arc<dyn SpecificFormatConverter>,
    rust_converter: Arc<dyn SpecificFormatConverter>,
    eslint_converter: Arc<dyn SpecificFormatConverter>,
    generic_converter: Arc<dyn SpecificFormatConverter>,
}

impl ConverterFactory {
    /// Create a new converter factory
    pub fn new() -> Self {
        Self {
            typescript_converter: Arc::new(TypeScriptConverter::new()),
            rust_converter: Arc::new(RustAnalyzerConverter::new()),
            eslint_converter: Arc::new(ESLintConverter::new()),
            generic_converter: Arc::new(GenericLSPConverter::new()),
        }
    }

    /// Get the appropriate converter for the given source type
    pub fn get_converter(&self, source_type: &SourceType) -> Arc<dyn SpecificFormatConverter> {
        match source_type {
            SourceType::TypeScript => self.typescript_converter.clone(),
            SourceType::RustAnalyzer => self.rust_converter.clone(),
            SourceType::ESLint => self.eslint_converter.clone(),
            SourceType::GenericLSP(_) => self.generic_converter.clone(),
        }
    }

    /// Get converter by source string
    pub fn get_converter_by_source(&self, source: &str) -> Arc<dyn SpecificFormatConverter> {
        let source_lower = source.to_lowercase();

        if self.typescript_converter.can_handle(&source_lower) {
            self.typescript_converter.clone()
        } else if self.rust_converter.can_handle(&source_lower) {
            self.rust_converter.clone()
        } else if self.eslint_converter.can_handle(&source_lower) {
            self.eslint_converter.clone()
        } else {
            self.generic_converter.clone()
        }
    }

    /// Detect source type from data
    pub fn detect_source_type(&self, data: &Value, source: &str) -> SourceType {
        SourceType::detect(data, source)
    }
}

impl Default for ConverterFactory {
    fn default() -> Self {
        Self::new()
    }
}