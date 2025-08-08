use crate::analyzers::base::DiagnosticPatterns;
use crate::analyzers::language_analyzer::ContextRequirements;
use crate::core::constants::config_files;
use crate::core::Diagnostic;
use regex::Regex;

pub struct TypeScriptContextAnalyzer;

impl Default for TypeScriptContextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeScriptContextAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn extract_context_requirements(&self, diagnostic: &Diagnostic) -> ContextRequirements {
        let mut requirements = ContextRequirements::default();

        // Extract file references from the diagnostic
        if diagnostic.message.contains("import") || diagnostic.message.contains("from") {
            if let Some(cap) = Regex::new(r#"from ['"](.+?)['"]"#)
                .unwrap()
                .captures(&diagnostic.message)
            {
                if let Some(module) = cap.get(1) {
                    requirements
                        .required_files
                        .push(format!("{}.ts", module.as_str()));
                    requirements
                        .required_files
                        .push(format!("{}.tsx", module.as_str()));
                    requirements
                        .required_files
                        .push(format!("{}/index.ts", module.as_str()));
                }
            }
        }

        // Type definition files
        let identifiers = DiagnosticPatterns::extract_quoted_identifiers(&diagnostic.message);
        for ident in &identifiers {
            requirements.required_types.push(ident.clone());
        }

        // Config files
        if diagnostic.message.contains("tsconfig")
            || diagnostic.message.contains("Cannot find module")
        {
            requirements
                .config_files
                .push(config_files::TSCONFIG_JSON.to_string());
            requirements
                .config_files
                .push(config_files::PACKAGE_JSON.to_string());
        }

        // Common dependency checks
        if diagnostic.message.contains("@types/") {
            if let Some(cap) = Regex::new(r"@types/(\w+)")
                .unwrap()
                .captures(&diagnostic.message)
            {
                if let Some(pkg) = cap.get(1) {
                    requirements
                        .dependencies
                        .push(format!("@types/{}", pkg.as_str()));
                }
            }
        }

        requirements
    }
}