use crate::core::semantic_context::{DependencyInfo, DependencyType};
use crate::core::types::Diagnostic;
use crate::core::dependency_analyzer::types::{
    FileDependencies, DependencyGraph, Language, ImportDependency, TypeReference, ExternalFunctionCall
};
use crate::core::dependency_analyzer::cache::DependencyCache;
use crate::core::dependency_analyzer::resolvers;
use anyhow::{anyhow, Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

/// Core dependency analysis engine
pub struct AnalysisEngine {
    typescript_parser: Parser,
    rust_parser: Parser,
    python_parser: Parser,
    cache: DependencyCache,
}

impl AnalysisEngine {
    pub fn new() -> Result<Self> {
        let mut typescript_parser = Parser::new();
        typescript_parser.set_language(tree_sitter_typescript::language_typescript())?;

        let mut rust_parser = Parser::new();
        rust_parser.set_language(tree_sitter_rust::language())?;

        let mut python_parser = Parser::new();
        python_parser.set_language(tree_sitter_python::language())?;

        Ok(Self {
            typescript_parser,
            rust_parser,
            python_parser,
            cache: DependencyCache::new(),
        })
    }

    /// Build dependency graph for a set of files
    pub async fn build_graph<P: AsRef<Path>>(&mut self, files: &[P]) -> Result<DependencyGraph> {
        let mut file_dependencies = HashMap::new();
        let mut reverse_dependencies: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();

        // First pass: analyze each file's dependencies
        for file_path in files {
            let path = file_path.as_ref().to_path_buf();
            if let Ok(deps) = self.analyze_file_dependencies(&path).await {
                file_dependencies.insert(path.clone(), deps.clone());

                // Build reverse dependencies
                for import in &deps.imports {
                    reverse_dependencies
                        .entry(import.source_file.clone())
                        .or_default()
                        .insert(path.clone());
                }
            }
        }

        Ok(DependencyGraph {
            file_dependencies,
            reverse_dependencies,
            impact_files: Vec::new(),
        })
    }

    /// Analyze dependencies for a specific diagnostic location
    pub async fn analyze_diagnostic_dependencies(
        &mut self,
        diagnostic: &Diagnostic,
        graph: &DependencyGraph,
    ) -> Result<Vec<DependencyInfo>> {
        let diagnostic_file = PathBuf::from(&diagnostic.file);
        let mut dependencies = Vec::new();

        // Get file dependencies
        if let Some(file_deps) = graph.file_dependencies.get(&diagnostic_file) {
            // Add import dependencies
            for import in &file_deps.imports {
                dependencies.push(DependencyInfo {
                    file_path: import.source_file.to_string_lossy().to_string(),
                    imported_symbols: import.imported_symbols.clone(),
                    export_symbols: vec![],
                    dependency_type: DependencyType::Direct,
                });
            }

            // Find type references relevant to the diagnostic
            if let Ok(relevant_types) = self.extract_types_from_diagnostic(diagnostic).await {
                for type_ref in &file_deps.type_references {
                    if relevant_types.contains(&type_ref.type_name) {
                        if let Some(source_file) = &type_ref.source_file {
                            dependencies.push(DependencyInfo {
                                file_path: source_file.to_string_lossy().to_string(),
                                imported_symbols: vec![type_ref.type_name.clone()],
                                export_symbols: vec![],
                                dependency_type: DependencyType::TypeOnly,
                            });
                        }
                    }
                }
            }
        }

        // Find files that might be affected by changes to the diagnostic location
        if let Some(reverse_deps) = graph.reverse_dependencies.get(&diagnostic_file) {
            for dependent_file in reverse_deps {
                // Check if the dependent file uses symbols near the diagnostic location
                if let Ok(used_symbols) = self.get_symbols_near_diagnostic(diagnostic).await {
                    if let Some(dep_file_info) = graph.file_dependencies.get(dependent_file) {
                        let imports_affected_symbols = dep_file_info.imports.iter().any(|imp| {
                            imp.source_file == diagnostic_file
                                && imp
                                    .imported_symbols
                                    .iter()
                                    .any(|sym| used_symbols.contains(sym))
                        });

                        if imports_affected_symbols {
                            dependencies.push(DependencyInfo {
                                file_path: dependent_file.to_string_lossy().to_string(),
                                imported_symbols: used_symbols.clone(),
                                export_symbols: vec![],
                                dependency_type: DependencyType::Direct,
                            });
                        }
                    }
                }
            }
        }

        Ok(dependencies)
    }

    /// Analyze dependencies for a single file
    async fn analyze_file_dependencies(&mut self, file_path: &Path) -> Result<FileDependencies> {
        // Check cache first
        if let Some(cached) = self.cache.get_dependencies(file_path) {
            return Ok(cached.clone());
        }

        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
        let language = self.detect_language(file_path);

        let tree = self.parse_file(&content, language)?;
        let root_node = tree.root_node();

        // Get language-specific resolver
        let resolver = resolvers::get_resolver(language)
            .ok_or_else(|| anyhow!("No resolver for language: {:?}", language))?;

        let imports = self.extract_imports(&root_node, &content, language, file_path, &*resolver)?;
        let exports = resolver.extract_exports(&root_node, &content);
        let type_references = self.extract_type_references(&root_node, &content, language, file_path)?;
        let function_calls = self.extract_external_function_calls(&root_node, &content, language)?;

        let dependencies = FileDependencies {
            file_path: file_path.to_path_buf(),
            imports,
            exports,
            type_references,
            function_calls,
            last_modified: fs::metadata(file_path)?.modified()?,
        };

        // Cache the result
        self.cache.cache_dependencies(file_path.to_path_buf(), dependencies.clone());
        self.cache.cache_ast(file_path.to_path_buf(), tree);

        Ok(dependencies)
    }

    fn detect_language(&self, file_path: &Path) -> Language {
        match file_path.extension().and_then(|ext| ext.to_str()) {
            Some("ts") | Some("tsx") => Language::TypeScript,
            Some("js") | Some("jsx") => Language::TypeScript,
            Some("rs") => Language::Rust,
            Some("py") => Language::Python,
            _ => Language::Unknown,
        }
    }

    fn parse_file(&mut self, content: &str, language: Language) -> Result<tree_sitter::Tree> {
        let tree = match language {
            Language::TypeScript => self.typescript_parser.parse(content, None),
            Language::Rust => self.rust_parser.parse(content, None),
            Language::Python => self.python_parser.parse(content, None),
            _ => return Err(anyhow!("Unsupported language")),
        };

        tree.ok_or_else(|| anyhow!("Failed to parse source file"))
    }

    fn extract_imports(
        &self,
        root: &Node,
        source: &str,
        language: Language,
        current_file: &Path,
        resolver: &dyn resolvers::LanguageResolver,
    ) -> Result<Vec<ImportDependency>> {
        let mut imports = Vec::new();
        let mut cursor = root.walk();

        let import_kinds = match language {
            Language::TypeScript => vec!["import_statement"],
            Language::Rust => vec!["use_declaration"],
            Language::Python => vec!["import_statement", "import_from_statement"],
            _ => vec![],
        };

        self.visit_nodes(&mut cursor, |node| {
            if import_kinds.contains(&node.kind()) {
                let line = node.start_position().row as u32;
                if let Some(import) = resolver.extract_import_dependency(&node, source, current_file, line) {
                    imports.push(import);
                }
            }
        });

        Ok(imports)
    }

    fn extract_type_references(
        &self,
        _root: &Node,
        _source: &str,
        _language: Language,
        _current_file: &Path,
    ) -> Result<Vec<TypeReference>> {
        // Placeholder for type reference extraction
        // This would find type annotations and references
        Ok(Vec::new())
    }

    fn extract_external_function_calls(
        &self,
        _root: &Node,
        _source: &str,
        _language: Language,
    ) -> Result<Vec<ExternalFunctionCall>> {
        // Placeholder for external function call extraction
        Ok(Vec::new())
    }

    async fn extract_types_from_diagnostic(
        &self,
        diagnostic: &Diagnostic,
    ) -> Result<HashSet<String>> {
        // Extract type names mentioned in the diagnostic message
        let mut types = HashSet::new();

        // Simple heuristics for now - could be enhanced with NLP
        let message = &diagnostic.message;

        // Look for common type patterns
        if let Some(captures) = regex::Regex::new(r"type `([^`]+)`")
            .unwrap()
            .captures(message)
        {
            types.insert(captures[1].to_string());
        }

        if let Some(captures) = regex::Regex::new(r"struct `([^`]+)`")
            .unwrap()
            .captures(message)
        {
            types.insert(captures[1].to_string());
        }

        if let Some(captures) = regex::Regex::new(r"interface `([^`]+)`")
            .unwrap()
            .captures(message)
        {
            types.insert(captures[1].to_string());
        }

        Ok(types)
    }

    async fn get_symbols_near_diagnostic(&self, diagnostic: &Diagnostic) -> Result<Vec<String>> {
        // Extract symbols near the diagnostic location
        // This is a simplified implementation
        let content = fs::read_to_string(&diagnostic.file)?;
        let lines: Vec<&str> = content.lines().collect();

        let start_line = diagnostic.range.start.line.saturating_sub(2) as usize;
        let end_line = (diagnostic.range.end.line + 2).min(lines.len() as u32 - 1) as usize;

        let mut symbols = Vec::new();
        for line_idx in start_line..=end_line {
            if let Some(line) = lines.get(line_idx) {
                // Simple regex to extract identifiers
                for cap in regex::Regex::new(r"\b[a-zA-Z_][a-zA-Z0-9_]*\b")
                    .unwrap()
                    .captures_iter(line)
                {
                    symbols.push(cap[0].to_string());
                }
            }
        }

        Ok(symbols)
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
}