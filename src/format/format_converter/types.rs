//! Format converter types and traits

use crate::core::errors::ParseError;
use crate::core::{Diagnostic, RawDiagnostics};
use async_trait::async_trait;
use serde_json::Value;

/// Trait for format-specific converters
#[async_trait]
pub trait SpecificFormatConverter: Send + Sync {
    /// Convert raw diagnostics to unified format
    async fn convert(&self, raw: &RawDiagnostics) -> Result<Vec<Diagnostic>, ParseError>;
    
    /// Check if this converter can handle the given source
    fn can_handle(&self, source: &str) -> bool;
    
    /// Get the name of this converter
    fn name(&self) -> &'static str;
}

/// Result of source type detection
#[derive(Debug, Clone, PartialEq)]
pub enum SourceType {
    TypeScript,
    RustAnalyzer,
    ESLint,
    GenericLSP(String), // Contains the actual source name
}

impl SourceType {
    /// Detect source type from diagnostic data
    pub fn detect(data: &Value, source: &str) -> Self {
        // First check explicit source string
        let source_lower = source.to_lowercase();
        
        if source_lower.contains("typescript") || source_lower.contains("ts") {
            return SourceType::TypeScript;
        }
        
        if source_lower.contains("rust") || source_lower.contains("analyzer") {
            return SourceType::RustAnalyzer;
        }
        
        if source_lower.contains("eslint") {
            return SourceType::ESLint;
        }
        
        // Then try to detect from data structure
        if let Some(obj) = data.as_object() {
            if obj.contains_key("diagnostics") {
                if let Some(first) = obj["diagnostics"].as_array().and_then(|arr| arr.first()) {
                    if first.get("code").is_some() && first.get("category").is_some() {
                        return SourceType::TypeScript;
                    }
                    if first.get("level").is_some() && first.get("spans").is_some() {
                        return SourceType::RustAnalyzer;
                    }
                }
            }
            
            if obj.contains_key("results") {
                return SourceType::ESLint;
            }
        }
        
        SourceType::GenericLSP(source.to_string())
    }
}