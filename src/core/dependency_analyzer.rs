use super::semantic_context::{DependencyInfo, DependencyType};
use super::types::Diagnostic;
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

/// Analyzes dependency relationships across files
pub struct DependencyAnalyzer {
    typescript_parser: Parser,
    rust_parser: Parser,
    python_parser: Parser,
    /// Cache of file dependency graphs
    dependency_cache: HashMap<PathBuf, FileDependencies>,
    /// Cache of parsed syntax trees
    ast_cache: HashMap<PathBuf, tree_sitter::Tree>,
}

/// Dependencies for a single file
#[derive(Debug, Clone)]
pub struct FileDependencies {
    pub file_path: PathBuf,
    pub imports: Vec<ImportDependency>,
    pub exports: Vec<ExportInfo>,
    pub type_references: Vec<TypeReference>,
    pub function_calls: Vec<ExternalFunctionCall>,
    pub last_modified: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct ImportDependency {
    pub source_file: PathBuf,
    pub imported_symbols: Vec<String>,
    pub import_type: ImportType,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub enum ImportType {
    Default,
    Named(Vec<String>),
    Namespace(String),
    Wildcard,
}

#[derive(Debug, Clone)]
pub struct ExportInfo {
    pub symbol_name: String,
    pub export_type: ExportType,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub enum ExportType {
    Function,
    Type,
    Variable,
    Class,
    Default,
}

#[derive(Debug, Clone)]
pub struct TypeReference {
    pub type_name: String,
    pub source_file: Option<PathBuf>,
    pub line: u32,
    pub context: String, // The surrounding context where type is used
}

#[derive(Debug, Clone)]
pub struct ExternalFunctionCall {
    pub function_name: String,
    pub module_path: Option<PathBuf>,
    pub line: u32,
    pub arguments_count: usize,
}

/// Dependency graph for cross-file analysis
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Map from file to its dependencies
    pub file_dependencies: HashMap<PathBuf, FileDependencies>,
    /// Reverse map: which files depend on each file
    pub reverse_dependencies: HashMap<PathBuf, HashSet<PathBuf>>,
    /// Files that import symbols used in diagnostic location
    pub impact_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
enum Language {
    TypeScript,
    Rust,
    Python,
    Unknown,
}

impl DependencyAnalyzer {
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
            dependency_cache: HashMap::new(),
            ast_cache: HashMap::new(),
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
                        .or_insert_with(HashSet::new)
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
                    dependency_type: DependencyType::Import,
                    symbols_used: import.imported_symbols.clone(),
                    line_range: Some((import.line, import.line)),
                });
            }

            // Find type references relevant to the diagnostic
            if let Ok(relevant_types) = self.extract_types_from_diagnostic(diagnostic).await {
                for type_ref in &file_deps.type_references {
                    if relevant_types.contains(&type_ref.type_name) {
                        if let Some(source_file) = &type_ref.source_file {
                            dependencies.push(DependencyInfo {
                                file_path: source_file.to_string_lossy().to_string(),
                                dependency_type: DependencyType::TypeReference,
                                symbols_used: vec![type_ref.type_name.clone()],
                                line_range: Some((type_ref.line, type_ref.line)),
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
                                dependency_type: DependencyType::VariableReference,
                                symbols_used: used_symbols.clone(),
                                line_range: None,
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
        if let Some(cached) = self.dependency_cache.get(file_path) {
            if let Ok(metadata) = fs::metadata(file_path) {
                if let Ok(modified) = metadata.modified() {
                    if cached.last_modified >= modified {
                        return Ok(cached.clone());
                    }
                }
            }
        }

        let content = fs::read_to_string(file_path)?;
        let language = self.detect_language(file_path);

        let tree = self.parse_file(&content, language)?;
        let root_node = tree.root_node();

        let imports = self.extract_imports(&root_node, &content, language, file_path)?;
        let exports = self.extract_exports(&root_node, &content, language)?;
        let type_references =
            self.extract_type_references(&root_node, &content, language, file_path)?;
        let function_calls =
            self.extract_external_function_calls(&root_node, &content, language)?;

        let dependencies = FileDependencies {
            file_path: file_path.to_path_buf(),
            imports,
            exports,
            type_references,
            function_calls,
            last_modified: fs::metadata(file_path)?.modified()?,
        };

        // Cache the result
        self.dependency_cache
            .insert(file_path.to_path_buf(), dependencies.clone());
        self.ast_cache.insert(file_path.to_path_buf(), tree);

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
                if let Some(import) =
                    self.extract_import_dependency(&node, source, language, current_file)
                {
                    imports.push(import);
                }
            }
        });

        Ok(imports)
    }

    fn extract_import_dependency(
        &self,
        node: &Node,
        source: &str,
        language: Language,
        current_file: &Path,
    ) -> Option<ImportDependency> {
        let line = node.start_position().row as u32;

        match language {
            Language::TypeScript => {
                // Extract source path
                let source_path = node
                    .child_by_field_name("source")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.trim_matches(|c| c == '"' || c == '\''))?;

                let resolved_path = self.resolve_import_path(current_file, source_path)?;

                // Extract imported symbols
                let mut imported_symbols = Vec::new();
                let mut import_type = ImportType::Default;

                if let Some(clause) = node.child_by_field_name("import_clause") {
                    // Handle different import patterns
                    let mut cursor = clause.walk();
                    self.visit_nodes(&mut cursor, |n| match n.kind() {
                        "identifier" => {
                            if let Ok(name) = n.utf8_text(source.as_bytes()) {
                                imported_symbols.push(name.to_string());
                            }
                        }
                        "named_imports" => {
                            import_type = ImportType::Named(Vec::new());
                        }
                        "namespace_import" => {
                            if let Ok(name) = n.utf8_text(source.as_bytes()) {
                                import_type = ImportType::Namespace(name.to_string());
                            }
                        }
                        _ => {}
                    });
                }

                Some(ImportDependency {
                    source_file: resolved_path,
                    imported_symbols,
                    import_type,
                    line,
                })
            }
            Language::Rust => {
                // Extract module path from use declaration
                if let Some(path_node) = node.child_by_field_name("argument") {
                    if let Ok(path_text) = path_node.utf8_text(source.as_bytes()) {
                        // Convert Rust module path to file path
                        if let Some(resolved_path) =
                            self.resolve_rust_module_path(current_file, path_text)
                        {
                            let symbols = self.extract_rust_use_symbols(&path_node, source);
                            return Some(ImportDependency {
                                source_file: resolved_path,
                                imported_symbols: symbols,
                                import_type: ImportType::Named(Vec::new()),
                                line,
                            });
                        }
                    }
                }
                None
            }
            Language::Python => {
                // Handle Python imports
                let module_name = node
                    .child_by_field_name("module_name")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())?;

                if let Some(resolved_path) =
                    self.resolve_python_module_path(current_file, module_name)
                {
                    let symbols = self.extract_python_import_symbols(&node, source);
                    return Some(ImportDependency {
                        source_file: resolved_path,
                        imported_symbols: symbols,
                        import_type: ImportType::Named(Vec::new()),
                        line,
                    });
                }
                None
            }
            _ => None,
        }
    }

    fn resolve_import_path(&self, current_file: &Path, import_path: &str) -> Option<PathBuf> {
        let current_dir = current_file.parent()?;

        // Handle relative imports
        if import_path.starts_with("./") || import_path.starts_with("../") {
            let resolved = current_dir.join(import_path);
            // Try common extensions
            for ext in &[".ts", ".tsx", ".js", ".jsx"] {
                let with_ext = resolved.with_extension(&ext[1..]);
                if with_ext.exists() {
                    return Some(with_ext);
                }
            }
            // Try index files
            let index_path = resolved.join("index");
            for ext in &[".ts", ".tsx", ".js", ".jsx"] {
                let with_ext = index_path.with_extension(&ext[1..]);
                if with_ext.exists() {
                    return Some(with_ext);
                }
            }
        }

        // For absolute imports, we'd need package resolution logic
        // For now, return None for node_modules imports
        None
    }

    fn resolve_rust_module_path(&self, current_file: &Path, module_path: &str) -> Option<PathBuf> {
        // Basic Rust module resolution
        let current_dir = current_file.parent()?;

        // Convert :: to / and look for .rs files
        let file_path = module_path.replace("::", "/");
        let rs_path = current_dir.join(format!("{}.rs", file_path));

        if rs_path.exists() {
            Some(rs_path)
        } else {
            // Try mod.rs in subdirectory
            let mod_path = current_dir.join(&file_path).join("mod.rs");
            if mod_path.exists() {
                Some(mod_path)
            } else {
                None
            }
        }
    }

    fn resolve_python_module_path(
        &self,
        current_file: &Path,
        module_name: &str,
    ) -> Option<PathBuf> {
        let current_dir = current_file.parent()?;

        // Convert . to / and look for .py files
        let file_path = module_name.replace(".", "/");
        let py_path = current_dir.join(format!("{}.py", file_path));

        if py_path.exists() {
            Some(py_path)
        } else {
            // Try __init__.py in subdirectory
            let init_path = current_dir.join(&file_path).join("__init__.py");
            if init_path.exists() {
                Some(init_path)
            } else {
                None
            }
        }
    }

    fn extract_rust_use_symbols(&self, node: &Node, source: &str) -> Vec<String> {
        let mut symbols = Vec::new();
        let mut cursor = node.walk();

        self.visit_nodes(&mut cursor, |n| {
            if n.kind() == "identifier" {
                if let Ok(name) = n.utf8_text(source.as_bytes()) {
                    symbols.push(name.to_string());
                }
            }
        });

        symbols
    }

    fn extract_python_import_symbols(&self, node: &Node, source: &str) -> Vec<String> {
        let mut symbols = Vec::new();
        let mut cursor = node.walk();

        self.visit_nodes(&mut cursor, |n| {
            if matches!(n.kind(), "dotted_name" | "aliased_import") {
                if let Ok(name) = n.utf8_text(source.as_bytes()) {
                    symbols.push(name.to_string());
                }
            }
        });

        symbols
    }

    fn extract_exports(
        &self,
        _root: &Node,
        _source: &str,
        _language: Language,
    ) -> Result<Vec<ExportInfo>> {
        // Placeholder for export extraction
        // This would analyze export statements and public symbols
        Ok(Vec::new())
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

impl Default for DependencyAnalyzer {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
