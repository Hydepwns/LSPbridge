use crate::analyzers::base::DiagnosticPatterns;
use crate::analyzers::error_codes::RustErrorCode;
use crate::analyzers::language_analyzer::ContextRequirements;
use crate::core::constants::config_files;
use crate::core::Diagnostic;
use regex::Regex;

pub struct ContextAnalyzer;

impl Default for ContextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn extract_context_requirements(&self, diagnostic: &Diagnostic) -> ContextRequirements {
        let mut requirements = ContextRequirements::default();

        // Extract symbols that need context
        let identifiers = DiagnosticPatterns::extract_quoted_identifiers(&diagnostic.message);
        requirements.required_symbols.extend(identifiers);

        // For borrow/lifetime errors, we need the full function
        if let Some(code_str) = &diagnostic.code {
            if let Some(rust_code) = RustErrorCode::from_str(code_str) {
                if rust_code.is_borrow_error() || rust_code.is_lifetime_error() {
                    requirements
                        .required_symbols
                        .push("_full_function_context".to_string());
                }
            }
        }

        // For trait errors, we need trait definitions
        if diagnostic.message.contains("trait") {
            if let Some(trait_match) = Regex::new(r"trait `([^`]+)`")
                .unwrap()
                .captures(&diagnostic.message)
            {
                if let Some(trait_name) = trait_match.get(1) {
                    requirements
                        .required_types
                        .push(trait_name.as_str().to_string());
                }
            }
        }

        // Config files
        if diagnostic.message.contains("Cargo.toml") {
            requirements
                .config_files
                .push(config_files::CARGO_TOML.to_string());
        }

        // External crates
        if diagnostic
            .message
            .contains("use of unstable library feature")
        {
            requirements
                .config_files
                .push(config_files::CARGO_TOML.to_string());
            requirements
                .dependencies
                .push("Check crate features".to_string());
        }

        requirements
    }
}