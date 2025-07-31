use super::LanguageResolver;
use crate::core::dependency_analyzer::types::{ImportDependency, ImportType, ExportInfo, ExportType};
use std::path::{Path, PathBuf};
use tree_sitter::Node;

pub struct PythonResolver;

impl PythonResolver {
    pub fn new() -> Self {
        Self
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

impl LanguageResolver for PythonResolver {
    fn extract_import_dependency(
        &self,
        node: &Node,
        source: &str,
        current_file: &Path,
        line: u32,
    ) -> Option<ImportDependency> {
        // Handle Python imports
        let module_name = node
            .child_by_field_name("module_name")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())?;

        if let Some(resolved_path) = self.resolve_import_path(current_file, module_name) {
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
    
    fn resolve_import_path(&self, current_file: &Path, import_path: &str) -> Option<PathBuf> {
        let current_dir = current_file.parent()?;

        // Convert . to / and look for .py files
        let file_path = import_path.replace(".", "/");
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
    
    fn extract_exports(&self, root: &Node, source: &str) -> Vec<ExportInfo> {
        let mut exports = Vec::new();
        let mut cursor = root.walk();
        
        self.visit_nodes(&mut cursor, |node| {
            let line = node.start_position().row as u32;
            match node.kind() {
                "function_definition" => {
                    // Check if it's not private (doesn't start with _)
                    if let Some(name_node) = node.child_by_field_name("name") {
                        if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                            if !name.starts_with('_') {
                                exports.push(ExportInfo {
                                    symbol_name: name.to_string(),
                                    export_type: ExportType::Function,
                                    line,
                                });
                            }
                        }
                    }
                }
                "class_definition" => {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                            if !name.starts_with('_') {
                                exports.push(ExportInfo {
                                    symbol_name: name.to_string(),
                                    export_type: ExportType::Class,
                                    line,
                                });
                            }
                        }
                    }
                }
                "assignment" => {
                    // Look for module-level variable assignments
                    if let Some(left) = node.child_by_field_name("left") {
                        if left.kind() == "identifier" {
                            if let Ok(name) = left.utf8_text(source.as_bytes()) {
                                if !name.starts_with('_') && name.chars().next().unwrap().is_uppercase() {
                                    // Likely a constant or exported variable
                                    exports.push(ExportInfo {
                                        symbol_name: name.to_string(),
                                        export_type: ExportType::Variable,
                                        line,
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        });
        
        exports
    }
}