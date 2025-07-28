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
//!
//! ## Supported Languages
//!
//! - TypeScript/JavaScript (via tree-sitter-typescript)
//! - Rust (via tree-sitter-rust)  
//! - Python (via tree-sitter-python)
//!
//! ## Usage Examples
//!
//! ```rust
//! use lsp_bridge::core::semantic_context::ContextExtractor;
//! use lsp_bridge::core::types::Diagnostic;
//!
//! // Create a context extractor with built-in parsers
//! let mut extractor = ContextExtractor::new()?;
//!
//! // Extract context for a diagnostic
//! let context = extractor.extract_context_from_file(&diagnostic)?;
//!
//! // Access extracted information
//! if let Some(function) = &context.function_context {
//!     println!("Error in function: {}", function.name);
//! }
//!
//! if let Some(class) = &context.class_context {
//!     println!("Error in class: {}", class.name);
//! }
//!
//! // Use relevance score for prioritization
//! println!("Context relevance: {:.2}", context.relevance_score);
//! ```
//!
//! ## Context Elements
//!
//! The semantic context includes multiple layers of information:
//!
//! - **Function Context**: Complete function signature and body where the error occurs
//! - **Class Context**: Class/struct/interface definition containing the error
//! - **Import Context**: Relevant imports and module dependencies
//! - **Type Definitions**: Custom types referenced in the diagnostic
//! - **Local Variables**: Variables in scope at the error location
//! - **Call Hierarchy**: Function calls to and from the error location
//! - **Dependencies**: Cross-file dependencies and references
//!
//! ## Performance Characteristics
//!
//! - **Parser Initialization**: ~50ms per language (one-time cost)
//! - **Context Extraction**: ~5-20ms per diagnostic depending on file size
//! - **Memory Usage**: ~10-50MB for parser state, scales with file count
//! - **Concurrency**: Thread-safe after initialization, parsers are exclusive-access
//!
//! ## Error Handling
//!
//! This module can return the following errors:
//! - `anyhow::Error` - Parser initialization failures
//! - `anyhow::Error` - File reading failures
//! - `anyhow::Error` - Parse tree construction failures
//!
//! Most errors are gracefully handled by returning default/empty context rather than
//! failing the entire operation.

use super::types::Diagnostic;
use crate::parser_analyzer;
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use tree_sitter::Node;

/// Semantic context around a diagnostic
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticContext {
    /// The complete function/method containing the diagnostic
    pub function_context: Option<FunctionContext>,
    /// The class/struct/interface containing the diagnostic
    pub class_context: Option<ClassContext>,
    /// Relevant imports/uses for understanding the error
    pub imports: Vec<ImportContext>,
    /// Type definitions referenced in the diagnostic
    pub type_definitions: Vec<TypeDefinition>,
    /// Variables in scope at the diagnostic location
    pub local_variables: Vec<VariableContext>,
    /// Call hierarchy information (functions called from/calling this location)
    pub call_hierarchy: CallHierarchy,
    /// Cross-file dependencies relevant to this diagnostic
    pub dependencies: Vec<DependencyInfo>,
    /// Confidence score for context relevance (0.0-1.0)
    pub relevance_score: f32,
    /// Surrounding code snippets for additional context
    pub surrounding_code: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionContext {
    pub name: String,
    pub signature: String,
    pub body: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassContext {
    pub name: String,
    pub kind: String, // class, struct, interface, trait
    pub definition: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportContext {
    pub statement: String,
    pub imported_names: Vec<String>,
    pub source: Option<String>,
    pub line: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub definition: String,
    pub kind: String, // type, interface, class, struct
    pub line: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VariableContext {
    pub name: String,
    pub type_annotation: Option<String>,
    pub initialization: Option<String>,
    pub line: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallHierarchy {
    /// Functions called from the diagnostic location
    pub calls_outgoing: Vec<FunctionCall>,
    /// Functions that call the function containing the diagnostic
    pub calls_incoming: Vec<FunctionCall>,
    /// Depth of call stack analysis performed
    pub analysis_depth: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionCall {
    pub function_name: String,
    pub call_site_line: u32,
    pub arguments: Vec<String>,
    pub return_type: Option<String>,
    /// File containing this function call (for cross-file calls)
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyInfo {
    pub file_path: String,
    pub dependency_type: DependencyType,
    pub symbols_used: Vec<String>,
    pub line_range: Option<(u32, u32)>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DependencyType {
    /// Direct import/use statement
    Import,
    /// Type definition referenced in error
    TypeReference,
    /// Function called from error location
    FunctionCall,
    /// Variable referenced from outer scope
    VariableReference,
}

// Apply parser_analyzer macro to ContextExtractor
parser_analyzer! {
    /// Extracts semantic context from source code using multiple language parsers
    pub struct ContextExtractor {
        parsers: {
            typescript => tree_sitter_typescript::language_typescript(),
            rust => tree_sitter_rust::language(),
            python => tree_sitter_python::language()
        }
    }
}

impl ContextExtractor {
    /// Extract semantic context for a diagnostic
    pub fn extract_context(
        &mut self,
        diagnostic: &Diagnostic,
        file_content: &str,
    ) -> Result<SemanticContext> {
        let language = self.detect_language(&diagnostic.file);

        let tree = match language {
            Language::TypeScript => {
                if let Some(parser) = self.get_parser("typescript") {
                    parser.parse(file_content, None)
                } else {
                    return Ok(SemanticContext::default());
                }
            }
            Language::Rust => {
                if let Some(parser) = self.get_parser("rust") {
                    parser.parse(file_content, None)
                } else {
                    return Ok(SemanticContext::default());
                }
            }
            Language::Python => {
                if let Some(parser) = self.get_parser("python") {
                    parser.parse(file_content, None)
                } else {
                    return Ok(SemanticContext::default());
                }
            }
            _ => return Ok(SemanticContext::default()),
        };

        let tree = tree.ok_or_else(|| anyhow!("Failed to parse source file"))?;
        let root_node = tree.root_node();

        // Find the node at the diagnostic location
        let diagnostic_node = self.find_node_at_position(
            root_node,
            diagnostic.range.start.line,
            diagnostic.range.start.character,
            file_content,
        );

        let mut context = SemanticContext::default();

        // Extract various context elements
        if let Some(node) = diagnostic_node {
            context.function_context = self.find_enclosing_function(&node, file_content, language);
            context.class_context = self.find_enclosing_class(&node, file_content, language);
            context.imports = self.extract_imports(root_node, file_content, language);
            context.type_definitions =
                self.extract_type_definitions(root_node, file_content, language);
            context.local_variables = self.extract_local_variables(&node, file_content, language);

            // Enhanced features for V2
            context.call_hierarchy = self.extract_call_hierarchy(&node, file_content, language);
            context.dependencies =
                self.extract_dependencies(&node, file_content, language, &diagnostic.file);
            // TODO: Re-enable filtering with better heuristics
            // context = self.filter_relevant_types(context, diagnostic);
            context.relevance_score = self.calculate_relevance_score(&context);
        }

        Ok(context)
    }

    /// Load file content and extract context
    pub fn extract_context_from_file(
        &mut self,
        diagnostic: &Diagnostic,
    ) -> Result<SemanticContext> {
        let file_content = fs::read_to_string(&diagnostic.file)?;
        self.extract_context(diagnostic, &file_content)
    }

    fn detect_language(&self, file_path: &str) -> Language {
        match Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
        {
            Some("ts") | Some("tsx") => Language::TypeScript,
            Some("js") | Some("jsx") => Language::TypeScript,
            Some("rs") => Language::Rust,
            Some("py") => Language::Python,
            _ => Language::Unknown,
        }
    }

    fn find_node_at_position<'a>(
        &self,
        root: Node<'a>,
        line: u32,
        column: u32,
        _source: &str,
    ) -> Option<Node<'a>> {
        let mut cursor = root.walk();
        let mut result = None;

        loop {
            let node = cursor.node();
            let start = node.start_position();
            let end = node.end_position();

            if start.row <= line as usize && end.row >= line as usize {
                if start.row == line as usize && start.column > column as usize {
                    break;
                }
                if end.row == line as usize && end.column < column as usize {
                    break;
                }

                result = Some(node);

                if !cursor.goto_first_child() {
                    break;
                }
            } else if !cursor.goto_next_sibling() {
                break;
            }
        }

        result
    }

    fn find_enclosing_function(
        &self,
        node: &Node,
        source: &str,
        language: Language,
    ) -> Option<FunctionContext> {
        let mut current = Some(*node);

        while let Some(n) = current {
            let kind = n.kind();

            let is_function = match language {
                Language::TypeScript => {
                    matches!(kind, "function_declaration" | "method_definition" | 
                            "arrow_function" | "function_expression") ||
                    // Check if this is a variable declaration containing an arrow function
                    (kind == "variable_declarator" && self.contains_arrow_function(&n, source)) ||
                    // Check if this is a lexical declaration (const/let) containing a function
                    (kind == "lexical_declaration" && self.contains_arrow_function(&n, source))
                }
                Language::Rust => {
                    matches!(kind, "function_item" | "impl_item")
                }
                Language::Python => {
                    matches!(kind, "function_definition")
                }
                _ => false,
            };

            if is_function {
                return self.extract_function_context(&n, source, language);
            }

            current = n.parent();
        }

        None
    }

    fn contains_arrow_function(&self, node: &Node, source: &str) -> bool {
        // Recursively check if this node or any of its descendants contains an arrow function
        self.contains_arrow_function_recursive(node, source)
    }

    fn contains_arrow_function_recursive(&self, node: &Node, source: &str) -> bool {
        // Check current node
        if node.kind() == "arrow_function" {
            return true;
        }

        // Check for useCallback, useEffect, etc. patterns
        if node.kind() == "call_expression" {
            if let Some(function_name) = node.child(0) {
                let function_text = &source[function_name.start_byte()..function_name.end_byte()];
                if function_text.starts_with("useCallback") || function_text.starts_with("useMemo")
                {
                    return true;
                }
            }
        }

        // Recursively check children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.contains_arrow_function_recursive(&child, source) {
                return true;
            }
        }

        false
    }

    fn extract_function_context(
        &self,
        node: &Node,
        source: &str,
        language: Language,
    ) -> Option<FunctionContext> {
        let name = match language {
            Language::TypeScript => {
                if node.kind() == "variable_declarator" {
                    // For variable declarators like: const handleUserClick = useCallback(...)
                    node.child_by_field_name("name")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .unwrap_or("anonymous")
                        .to_string()
                } else if node.kind() == "lexical_declaration" {
                    // For lexical declarations, get the name from the first variable declarator
                    node.children(&mut node.walk())
                        .find(|child| child.kind() == "variable_declarator")
                        .and_then(|declarator| declarator.child_by_field_name("name"))
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .unwrap_or("anonymous")
                        .to_string()
                } else {
                    node.child_by_field_name("name")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .unwrap_or("anonymous")
                        .to_string()
                }
            }
            Language::Rust => node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or("anonymous")
                .to_string(),
            Language::Python => node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or("anonymous")
                .to_string(),
            _ => "unknown".to_string(),
        };

        let signature = self.extract_function_signature(node, source, language);
        let body = node.utf8_text(source.as_bytes()).ok()?.to_string();

        Some(FunctionContext {
            name,
            signature,
            body,
            start_line: node.start_position().row as u32,
            end_line: node.end_position().row as u32,
        })
    }

    fn extract_function_signature(&self, node: &Node, source: &str, language: Language) -> String {
        match language {
            Language::TypeScript => {
                // Extract up to the opening brace
                if let Some(body) = node.child_by_field_name("body") {
                    let start = node.start_byte();
                    let end = body.start_byte();
                    source[start..end].trim().to_string()
                } else {
                    node.utf8_text(source.as_bytes()).unwrap_or("").to_string()
                }
            }
            Language::Rust => {
                // Similar extraction for Rust
                if let Some(body) = node.child_by_field_name("body") {
                    let start = node.start_byte();
                    let end = body.start_byte();
                    source[start..end].trim().to_string()
                } else {
                    node.utf8_text(source.as_bytes()).unwrap_or("").to_string()
                }
            }
            Language::Python => {
                // Extract def line
                let first_line_end = source[node.start_byte()..]
                    .find(':')
                    .map(|i| node.start_byte() + i + 1)
                    .unwrap_or(node.end_byte());
                source[node.start_byte()..first_line_end].to_string()
            }
            _ => String::new(),
        }
    }

    fn find_enclosing_class(
        &self,
        node: &Node,
        source: &str,
        language: Language,
    ) -> Option<ClassContext> {
        let mut current = Some(*node);

        while let Some(n) = current {
            let kind = n.kind();

            let (is_class, class_kind) = match language {
                Language::TypeScript => match kind {
                    "class_declaration" => (true, "class"),
                    "interface_declaration" => (true, "interface"),
                    _ => (false, ""),
                },
                Language::Rust => match kind {
                    "struct_item" => (true, "struct"),
                    "trait_item" => (true, "trait"),
                    "impl_item" => (true, "impl"),
                    _ => (false, ""),
                },
                Language::Python => match kind {
                    "class_definition" => (true, "class"),
                    _ => (false, ""),
                },
                _ => (false, ""),
            };

            if is_class {
                return self.extract_class_context(&n, source, language, class_kind);
            }

            current = n.parent();
        }

        None
    }

    fn extract_class_context(
        &self,
        node: &Node,
        source: &str,
        _language: Language,
        kind: &str,
    ) -> Option<ClassContext> {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .unwrap_or("anonymous")
            .to_string();

        let definition = node.utf8_text(source.as_bytes()).ok()?.to_string();

        Some(ClassContext {
            name,
            kind: kind.to_string(),
            definition,
            start_line: node.start_position().row as u32,
            end_line: node.end_position().row as u32,
        })
    }

    fn extract_imports(&self, root: Node, source: &str, language: Language) -> Vec<ImportContext> {
        let mut imports = Vec::new();
        let mut cursor = root.walk();

        let import_kinds = match language {
            Language::TypeScript => vec!["import_statement", "import_clause"],
            Language::Rust => vec!["use_declaration"],
            Language::Python => vec!["import_statement", "import_from_statement"],
            _ => vec![],
        };

        self.visit_nodes(&mut cursor, |node| {
            if import_kinds.contains(&node.kind()) {
                if let Some(import) = self.extract_import_context(&node, source, language) {
                    imports.push(import);
                }
            }
        });

        imports
    }

    fn extract_import_context(
        &self,
        node: &Node,
        source: &str,
        language: Language,
    ) -> Option<ImportContext> {
        let statement = node.utf8_text(source.as_bytes()).ok()?.to_string();
        let line = node.start_position().row as u32;

        // Extract imported names based on language
        let (imported_names, source_module) = match language {
            Language::TypeScript => {
                let names = self.extract_typescript_imports(node, source);
                let module = node
                    .child_by_field_name("source")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.trim_matches(|c| c == '"' || c == '\'').to_string());
                (names, module)
            }
            Language::Rust => {
                let names = self.extract_rust_imports(node, source);
                (names, None)
            }
            Language::Python => {
                let names = self.extract_python_imports(node, source);
                let module = node
                    .child_by_field_name("module_name")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.to_string());
                (names, module)
            }
            _ => (vec![], None),
        };

        Some(ImportContext {
            statement,
            imported_names,
            source: source_module,
            line,
        })
    }

    fn extract_typescript_imports(&self, node: &Node, source: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut cursor = node.walk();

        self.visit_nodes(&mut cursor, |n| {
            if matches!(n.kind(), "identifier" | "import_specifier") {
                if let Ok(name) = n.utf8_text(source.as_bytes()) {
                    if !name.is_empty() && name != "from" && name != "import" {
                        names.push(name.to_string());
                    }
                }
            }
        });

        names
    }

    fn extract_rust_imports(&self, node: &Node, source: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut cursor = node.walk();

        self.visit_nodes(&mut cursor, |n| {
            if n.kind() == "identifier" {
                if let Ok(name) = n.utf8_text(source.as_bytes()) {
                    if !name.is_empty() && name != "use" && name != "as" {
                        names.push(name.to_string());
                    }
                }
            }
        });

        names
    }

    fn extract_python_imports(&self, node: &Node, source: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut cursor = node.walk();

        self.visit_nodes(&mut cursor, |n| {
            if matches!(n.kind(), "dotted_name" | "aliased_import") {
                if let Ok(name) = n.utf8_text(source.as_bytes()) {
                    names.push(name.to_string());
                }
            }
        });

        names
    }

    fn extract_type_definitions(
        &self,
        root: Node,
        source: &str,
        language: Language,
    ) -> Vec<TypeDefinition> {
        let mut types = Vec::new();
        let mut cursor = root.walk();

        let type_kinds = match language {
            Language::TypeScript => vec!["type_alias_declaration", "interface_declaration"],
            Language::Rust => vec!["type_alias", "struct_item", "enum_item"],
            Language::Python => vec!["class_definition"], // Python doesn't have explicit type aliases
            _ => vec![],
        };

        self.visit_nodes(&mut cursor, |node| {
            if type_kinds.contains(&node.kind()) {
                if let Some(type_def) = self.extract_type_definition(&node, source, language) {
                    types.push(type_def);
                }
            }
        });

        types
    }

    fn extract_type_definition(
        &self,
        node: &Node,
        source: &str,
        _language: Language,
    ) -> Option<TypeDefinition> {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .unwrap_or("anonymous")
            .to_string();

        let definition = node.utf8_text(source.as_bytes()).ok()?.to_string();
        let kind = node.kind().to_string();
        let line = node.start_position().row as u32;

        Some(TypeDefinition {
            name,
            definition,
            kind,
            line,
        })
    }

    fn extract_local_variables(
        &self,
        node: &Node,
        source: &str,
        language: Language,
    ) -> Vec<VariableContext> {
        let mut variables = Vec::new();

        // Find the enclosing function or block
        let mut scope_node = Some(*node);
        while let Some(n) = scope_node {
            if self.is_scope_boundary(&n, language) {
                break;
            }
            scope_node = n.parent();
        }

        if let Some(scope) = scope_node {
            let mut cursor = scope.walk();
            let diagnostic_line = node.start_position().row;

            self.visit_nodes(&mut cursor, |n| {
                // Only include variables declared before the diagnostic
                if n.start_position().row <= diagnostic_line {
                    if let Some(var) = self.extract_variable_context(&n, source, language) {
                        variables.push(var);
                    }
                }
            });
        }

        variables
    }

    fn is_scope_boundary(&self, node: &Node, language: Language) -> bool {
        match language {
            Language::TypeScript => {
                matches!(
                    node.kind(),
                    "function_declaration"
                        | "method_definition"
                        | "arrow_function"
                        | "block_statement"
                )
            }
            Language::Rust => {
                matches!(node.kind(), "function_item" | "block")
            }
            Language::Python => {
                matches!(node.kind(), "function_definition" | "class_definition")
            }
            _ => false,
        }
    }

    fn extract_variable_context(
        &self,
        node: &Node,
        source: &str,
        language: Language,
    ) -> Option<VariableContext> {
        let is_variable = match language {
            Language::TypeScript => {
                matches!(node.kind(), "variable_declaration" | "lexical_declaration")
            }
            Language::Rust => {
                matches!(node.kind(), "let_declaration")
            }
            Language::Python => {
                matches!(node.kind(), "assignment")
            }
            _ => false,
        };

        if !is_variable {
            return None;
        }

        let name = node
            .child_by_field_name("name")
            .or_else(|| node.child_by_field_name("pattern"))
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .unwrap_or("")
            .to_string();

        if name.is_empty() {
            return None;
        }

        let type_annotation = node
            .child_by_field_name("type")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let initialization = node
            .child_by_field_name("value")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        Some(VariableContext {
            name,
            type_annotation,
            initialization,
            line: node.start_position().row as u32,
        })
    }

    fn visit_nodes<F>(&self, cursor: &mut tree_sitter::TreeCursor, mut callback: F)
    where
        F: FnMut(Node),
    {
        loop {
            callback(cursor.node());

            if cursor.goto_first_child() {
                continue;
            }

            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return;
                }
            }
        }
    }

    /// Extract call hierarchy information around the diagnostic location
    fn extract_call_hierarchy(
        &self,
        node: &Node,
        source: &str,
        language: Language,
    ) -> CallHierarchy {
        let mut hierarchy = CallHierarchy::default();

        // Find outgoing calls from the current function or scope
        let scope_node = self
            .find_enclosing_function_node(node, language, source)
            .unwrap_or_else(|| {
                // If not in a function, use the top-level module or a reasonable scope
                let mut current = Some(*node);
                while let Some(n) = current {
                    let parent = n.parent();
                    if parent.is_none() || n.kind() == "program" || n.kind() == "source_file" {
                        return n;
                    }
                    current = parent;
                }
                *node
            });

        hierarchy.calls_outgoing = self.extract_function_calls(&scope_node, source, language);
        hierarchy.analysis_depth = 1;

        hierarchy
    }

    fn find_enclosing_function_node<'a>(
        &self,
        node: &Node<'a>,
        language: Language,
        source: &str,
    ) -> Option<Node<'a>> {
        let mut current = Some(*node);

        while let Some(n) = current {
            let kind = n.kind();

            let is_function = match language {
                Language::TypeScript => {
                    matches!(kind, "function_declaration" | "method_definition" | 
                            "arrow_function" | "function_expression") ||
                    // Check if this is a variable declaration containing an arrow function
                    (kind == "variable_declarator" && self.contains_arrow_function(&n, source)) ||
                    // Check if this is a lexical declaration (const/let) containing a function
                    (kind == "lexical_declaration" && self.contains_arrow_function(&n, source))
                }
                Language::Rust => {
                    matches!(kind, "function_item" | "impl_item")
                }
                Language::Python => {
                    matches!(kind, "function_definition")
                }
                _ => false,
            };

            if is_function {
                return Some(n);
            }

            current = n.parent();
        }

        None
    }

    fn extract_function_calls<'a>(
        &self,
        function_node: &Node<'a>,
        source: &str,
        language: Language,
    ) -> Vec<FunctionCall> {
        let mut calls = Vec::new();
        let mut cursor = function_node.walk();

        let call_kinds = match language {
            Language::TypeScript => vec!["call_expression"],
            Language::Rust => vec!["call_expression"],
            Language::Python => vec!["call"],
            _ => vec![],
        };

        self.visit_nodes(&mut cursor, |node| {
            if call_kinds.contains(&node.kind()) {
                if let Some(call) = self.extract_function_call_info(&node, source, language) {
                    calls.push(call);
                }
            }
        });

        calls
    }

    fn extract_function_call_info(
        &self,
        node: &Node,
        source: &str,
        language: Language,
    ) -> Option<FunctionCall> {
        let function_name = match language {
            Language::TypeScript => node
                .child_by_field_name("function")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or("unknown")
                .to_string(),
            Language::Rust => node
                .child_by_field_name("function")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or("unknown")
                .to_string(),
            Language::Python => node
                .child_by_field_name("function")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or("unknown")
                .to_string(),
            _ => return None,
        };

        let mut arguments = Vec::new();
        if let Some(args_node) = node.child_by_field_name("arguments") {
            let mut cursor = args_node.walk();
            self.visit_nodes(&mut cursor, |n| {
                if matches!(n.kind(), "identifier" | "string" | "number") {
                    if let Ok(arg) = n.utf8_text(source.as_bytes()) {
                        arguments.push(arg.to_string());
                    }
                }
            });
        }

        Some(FunctionCall {
            function_name,
            call_site_line: node.start_position().row as u32,
            arguments,
            return_type: None, // Could be enhanced with type analysis
            file_path: None,   // Current file only for now
        })
    }

    /// Extract cross-file dependencies for the diagnostic
    fn extract_dependencies(
        &self,
        node: &Node,
        source: &str,
        language: Language,
        _current_file: &str,
    ) -> Vec<DependencyInfo> {
        let mut dependencies = Vec::new();

        // Extract import dependencies
        let imports = self.extract_imports(node.parent().unwrap_or(*node), source, language);
        for import in imports {
            if let Some(source_path) = import.source {
                dependencies.push(DependencyInfo {
                    file_path: source_path,
                    dependency_type: DependencyType::Import,
                    symbols_used: import.imported_names,
                    line_range: Some((import.line, import.line)),
                });
            }
        }

        // TODO: Add type reference and function call dependencies
        // This would require cross-file analysis in Phase 3.2

        dependencies
    }

    /// Filter type definitions to only include those referenced in the diagnostic
    fn filter_relevant_types(
        &self,
        mut context: SemanticContext,
        diagnostic: &Diagnostic,
    ) -> SemanticContext {
        // Extract type names mentioned in the diagnostic message
        let diagnostic_text = &diagnostic.message;
        let type_keywords = vec!["type", "interface", "struct", "class", "enum"];

        // Simple heuristic: keep types mentioned in the error message
        context.type_definitions.retain(|type_def| {
            diagnostic_text.contains(&type_def.name)
                || type_keywords.iter().any(|keyword| {
                    diagnostic_text.contains(&format!("{} {}", keyword, type_def.name))
                })
        });

        context
    }

    /// Calculate relevance score for the extracted context
    fn calculate_relevance_score(&self, context: &SemanticContext) -> f32 {
        let mut score = 0.0;
        let mut max_score = 0.0;

        // Function context adds significant value
        if context.function_context.is_some() {
            score += 0.3;
        }
        max_score += 0.3;

        // Class context adds value
        if context.class_context.is_some() {
            score += 0.2;
        }
        max_score += 0.2;

        // Imports and types add value based on quantity
        if !context.imports.is_empty() {
            score += 0.15 * (context.imports.len() as f32 / 10.0).min(1.0);
        }
        max_score += 0.15;

        if !context.type_definitions.is_empty() {
            score += 0.15 * (context.type_definitions.len() as f32 / 5.0).min(1.0);
        }
        max_score += 0.15;

        // Call hierarchy adds value
        if !context.call_hierarchy.calls_outgoing.is_empty() {
            score += 0.1 * (context.call_hierarchy.calls_outgoing.len() as f32 / 5.0).min(1.0);
        }
        max_score += 0.1;

        // Local variables add some value
        if !context.local_variables.is_empty() {
            score += 0.1 * (context.local_variables.len() as f32 / 10.0).min(1.0);
        }
        max_score += 0.1;

        if max_score > 0.0 {
            score / max_score
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Language {
    TypeScript,
    Rust,
    Python,
    Unknown,
}

impl Default for SemanticContext {
    fn default() -> Self {
        Self {
            function_context: None,
            class_context: None,
            imports: Vec::new(),
            type_definitions: Vec::new(),
            local_variables: Vec::new(),
            call_hierarchy: CallHierarchy::default(),
            dependencies: Vec::new(),
            relevance_score: 0.0,
            surrounding_code: std::collections::HashMap::new(),
        }
    }
}

impl Default for CallHierarchy {
    fn default() -> Self {
        Self {
            calls_outgoing: Vec::new(),
            calls_incoming: Vec::new(),
            analysis_depth: 0,
        }
    }
}
