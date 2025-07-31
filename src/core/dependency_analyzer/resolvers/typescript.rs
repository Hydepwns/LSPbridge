use super::LanguageResolver;
use crate::core::dependency_analyzer::types::{ImportDependency, ImportType, ExportInfo, ExportType};
use std::path::{Path, PathBuf};
use tree_sitter::Node;

pub struct TypeScriptResolver;

impl TypeScriptResolver {
    pub fn new() -> Self {
        Self
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

impl LanguageResolver for TypeScriptResolver {
    fn extract_import_dependency(
        &self,
        node: &Node,
        source: &str,
        current_file: &Path,
        line: u32,
    ) -> Option<ImportDependency> {
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
    
    fn extract_exports(&self, root: &Node, source: &str) -> Vec<ExportInfo> {
        let mut exports = Vec::new();
        let mut cursor = root.walk();
        
        self.visit_nodes(&mut cursor, |node| {
            match node.kind() {
                "export_statement" => {
                    if let Some(declaration) = node.child_by_field_name("declaration") {
                        let line = node.start_position().row as u32;
                        match declaration.kind() {
                            "function_declaration" => {
                                if let Some(name_node) = declaration.child_by_field_name("name") {
                                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                        exports.push(ExportInfo {
                                            symbol_name: name.to_string(),
                                            export_type: ExportType::Function,
                                            line,
                                        });
                                    }
                                }
                            }
                            "class_declaration" => {
                                if let Some(name_node) = declaration.child_by_field_name("name") {
                                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                        exports.push(ExportInfo {
                                            symbol_name: name.to_string(),
                                            export_type: ExportType::Class,
                                            line,
                                        });
                                    }
                                }
                            }
                            "variable_declaration" => {
                                // Extract variable names
                                let mut var_cursor = declaration.walk();
                                self.visit_nodes(&mut var_cursor, |n| {
                                    if n.kind() == "variable_declarator" {
                                        if let Some(name_node) = n.child_by_field_name("name") {
                                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                                exports.push(ExportInfo {
                                                    symbol_name: name.to_string(),
                                                    export_type: ExportType::Variable,
                                                    line,
                                                });
                                            }
                                        }
                                    }
                                });
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        });
        
        exports
    }
}