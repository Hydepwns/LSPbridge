//! # Semantic Context Extraction
//!
//! This module provides semantic context extraction capabilities for IDE diagnostics,
//! enabling AI assistants to better understand and provide fixes for coding errors.
//!
//! ## Key Components
//!
//! - **SemanticContext**: Core data structure containing all contextual information
//! - **ContextExtractor**: Multi-language parser-based context extraction engine
//! - **Language Detection**: Automatic language detection from file extensions
//! - **Context Filtering**: Relevance scoring and context optimization

pub mod extractors;
pub mod types;

pub use types::*;

use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tree_sitter::{Node, Parser};

use crate::core::types::Diagnostic;
use extractors::{LanguageExtractor, utils};
use extractors::{typescript::TypeScriptExtractor, rust::RustExtractor, python::PythonExtractor};

/// Main context extraction engine
pub struct ContextExtractor {
    parsers: HashMap<String, Parser>,
    extractors: HashMap<Language, Box<dyn LanguageExtractor>>,
}

impl ContextExtractor {
    /// Create a new context extractor with all supported language parsers
    pub fn new() -> Result<Self> {
        let mut extractors = HashMap::new();
        extractors.insert(Language::TypeScript, Box::new(TypeScriptExtractor::new()) as Box<dyn LanguageExtractor>);
        extractors.insert(Language::JavaScript, Box::new(TypeScriptExtractor::new()) as Box<dyn LanguageExtractor>);
        extractors.insert(Language::Rust, Box::new(RustExtractor::new()) as Box<dyn LanguageExtractor>);
        extractors.insert(Language::Python, Box::new(PythonExtractor::new()) as Box<dyn LanguageExtractor>);

        let mut extractor = Self {
            parsers: HashMap::new(),
            extractors,
        };

        // Initialize parsers
        extractor.init_parsers()?;
        
        Ok(extractor)
    }

    fn init_parsers(&mut self) -> Result<()> {
        // TypeScript/JavaScript
        let mut ts_parser = Parser::new();
        ts_parser.set_language(tree_sitter_typescript::language_typescript())?;
        self.parsers.insert("typescript".to_string(), ts_parser);

        // JavaScript (uses TypeScript parser)
        let mut js_parser = Parser::new();
        js_parser.set_language(tree_sitter_typescript::language_tsx())?;
        self.parsers.insert("javascript".to_string(), js_parser);

        // Rust
        let mut rust_parser = Parser::new();
        rust_parser.set_language(tree_sitter_rust::language())?;
        self.parsers.insert("rust".to_string(), rust_parser);

        // Python
        let mut python_parser = Parser::new();
        python_parser.set_language(tree_sitter_python::language())?;
        self.parsers.insert("python".to_string(), python_parser);

        Ok(())
    }

    fn get_parser(&mut self, language: &str) -> Option<&mut Parser> {
        self.parsers.get_mut(language)
    }

    /// Extract semantic context for a diagnostic
    pub fn extract_context(
        &mut self,
        diagnostic: &Diagnostic,
        file_content: &str,
    ) -> Result<SemanticContext> {
        let language = self.detect_language(&diagnostic.file);

        let parser_key = match language {
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Rust => "rust",
            Language::Python => "python",
            Language::Unknown => return Ok(SemanticContext::default()),
        };

        let tree = if let Some(parser) = self.get_parser(parser_key) {
            parser.parse(file_content, None)
        } else {
            return Ok(SemanticContext::default());
        };

        let tree = tree.ok_or_else(|| anyhow!("Failed to parse source file"))?;
        let root_node = tree.root_node();

        // Find the node at the diagnostic location
        let diagnostic_node = utils::find_node_at_position(
            root_node,
            diagnostic.range.start.line,
            diagnostic.range.start.character,
            file_content,
        );

        let mut context = SemanticContext::default();

        // Get the appropriate extractor
        let extractor = self.extractors.get(&language)
            .ok_or_else(|| anyhow!("No extractor for language {:?}", language))?;

        // Extract various context elements
        if let Some(node) = diagnostic_node {
            // Function context
            if let Some(func_node) = extractor.find_enclosing_function(&node, file_content) {
                context.function_context = extractor.extract_function_context(&func_node, file_content);
            }

            // Class context
            if let Some(class_node) = extractor.find_enclosing_class(&node, file_content) {
                context.class_context = extractor.extract_class_context(&class_node, file_content);
            }

            // Local variables
            context.local_variables = extractor.extract_local_variables(&node, file_content, diagnostic.range.start.line);
        }

        // Extract global context elements
        context.imports = extractor.extract_imports(&root_node, file_content);
        context.type_definitions = extractor.extract_type_definitions(&root_node, file_content, diagnostic);
        
        // Extract call hierarchy
        if let Some(node) = diagnostic_node {
            context.call_hierarchy = self.extract_call_hierarchy(&node, file_content, language, extractor.as_ref())?;
        }

        // Extract dependencies
        context.dependencies = self.extract_dependencies(&context.imports, &diagnostic.file)?;

        // Calculate relevance score
        context.relevance_score = self.calculate_relevance_score(&context);

        Ok(context)
    }

    /// Extract context from a file path (convenience method)
    pub fn extract_context_from_file(
        &mut self,
        diagnostic: &Diagnostic,
    ) -> Result<SemanticContext> {
        let file_content = fs::read_to_string(&diagnostic.file)
            .with_context(|| format!("Failed to read file: {}", diagnostic.file))?;
        self.extract_context(diagnostic, &file_content)
    }

    fn detect_language(&self, file_path: &str) -> Language {
        match Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
        {
            Some("ts") | Some("tsx") => Language::TypeScript,
            Some("js") | Some("jsx") => Language::JavaScript,
            Some("rs") => Language::Rust,
            Some("py") => Language::Python,
            _ => Language::Unknown,
        }
    }

    fn extract_call_hierarchy(
        &self,
        node: &Node,
        source: &str,
        _language: Language,
        extractor: &dyn LanguageExtractor,
    ) -> Result<CallHierarchy> {
        let mut hierarchy = CallHierarchy::default();

        // Find the enclosing function
        if let Some(func_node) = extractor.find_enclosing_function(node, source) {
            // Extract callees (functions called by this function)
            hierarchy.callees = extractor.extract_function_calls(&func_node, source);
            
            // For callers, we would need to search the entire codebase
            // This is a simplified version that only looks in the current file
            // In a real implementation, this would use an index or cross-file analysis
        }

        hierarchy.depth = 1; // Single file analysis for now

        Ok(hierarchy)
    }

    fn extract_dependencies(
        &self,
        imports: &[ImportContext],
        _current_file: &str,
    ) -> Result<Vec<DependencyInfo>> {
        let mut dependencies = Vec::new();

        for import in imports {
            let dep_type = if import.source.starts_with('.') {
                DependencyType::Direct
            } else if import.source.contains("@types/") {
                DependencyType::TypeOnly
            } else {
                DependencyType::Direct
            };

            dependencies.push(DependencyInfo {
                file_path: import.source.clone(),
                imported_symbols: import.imported_names.clone(),
                export_symbols: Vec::new(), // Would need cross-file analysis
                dependency_type: dep_type,
            });
        }

        Ok(dependencies)
    }

    fn calculate_relevance_score(&self, context: &SemanticContext) -> f32 {
        let mut score = 0.0_f32;

        // Base score components
        if context.function_context.is_some() {
            score += 0.3;
        }
        if context.class_context.is_some() {
            score += 0.2;
        }
        if !context.imports.is_empty() {
            score += 0.1;
        }
        if !context.type_definitions.is_empty() {
            score += 0.15;
        }
        if !context.local_variables.is_empty() {
            score += 0.15;
        }
        if !context.call_hierarchy.callees.is_empty() {
            score += 0.1;
        }

        score.min(1.0_f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{DiagnosticSeverity, Position, Range};

    #[test]
    fn test_context_extraction_typescript() {
        let mut extractor = ContextExtractor::new().unwrap();
        
        let source = r#"
interface User {
    id: number;
    name: string;
}

function processUser(user: User): string {
    return user.name.toUpperCase();
}
"#;

        let diagnostic = Diagnostic {
            id: "test-diag-1".to_string(),
            file: "test.ts".to_string(),
            range: Range {
                start: Position { line: 7, character: 12 },
                end: Position { line: 7, character: 16 },
            },
            severity: DiagnosticSeverity::Error,
            code: Some("TS2339".to_string()),
            source: "typescript".to_string(),
            message: "Property 'name' does not exist on type 'User'.".to_string(),
            tags: None,
            related_information: None,
            data: None,
        };

        let context = extractor.extract_context(&diagnostic, source).unwrap();
        
        assert!(context.function_context.is_some());
        assert!(context.type_definitions.iter().any(|t| t.name == "User"));
    }
}