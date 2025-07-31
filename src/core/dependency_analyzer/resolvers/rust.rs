use super::LanguageResolver;
use crate::core::dependency_analyzer::types::{ImportDependency, ImportType, ExportInfo, ExportType};
use std::path::{Path, PathBuf};
use tree_sitter::Node;

pub struct RustResolver;

impl RustResolver {
    pub fn new() -> Self {
        Self
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

impl LanguageResolver for RustResolver {
    fn extract_import_dependency(
        &self,
        node: &Node,
        source: &str,
        current_file: &Path,
        line: u32,
    ) -> Option<ImportDependency> {
        // Extract module path from use declaration
        if let Some(path_node) = node.child_by_field_name("argument") {
            if let Ok(path_text) = path_node.utf8_text(source.as_bytes()) {
                // Convert Rust module path to file path
                if let Some(resolved_path) = self.resolve_import_path(current_file, path_text) {
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
    
    fn resolve_import_path(&self, current_file: &Path, import_path: &str) -> Option<PathBuf> {
        // Basic Rust module resolution
        let current_dir = current_file.parent()?;

        // Convert :: to / and look for .rs files
        let file_path = import_path.replace("::", "/");
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
    
    fn extract_exports(&self, root: &Node, source: &str) -> Vec<ExportInfo> {
        let mut exports = Vec::new();
        let mut cursor = root.walk();
        
        self.visit_nodes(&mut cursor, |node| {
            let line = node.start_position().row as u32;
            match node.kind() {
                "function_item" => {
                    // Check if it's public
                    if node.child(0).map(|n| n.kind() == "visibility_modifier").unwrap_or(false) {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                exports.push(ExportInfo {
                                    symbol_name: name.to_string(),
                                    export_type: ExportType::Function,
                                    line,
                                });
                            }
                        }
                    }
                }
                "struct_item" => {
                    if node.child(0).map(|n| n.kind() == "visibility_modifier").unwrap_or(false) {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                exports.push(ExportInfo {
                                    symbol_name: name.to_string(),
                                    export_type: ExportType::Type,
                                    line,
                                });
                            }
                        }
                    }
                }
                "impl_item" => {
                    // Extract public methods from impl blocks
                    let mut impl_cursor = node.walk();
                    self.visit_nodes(&mut impl_cursor, |impl_node| {
                        if impl_node.kind() == "function_item" {
                            if impl_node.child(0).map(|n| n.kind() == "visibility_modifier").unwrap_or(false) {
                                if let Some(name_node) = impl_node.child_by_field_name("name") {
                                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                        exports.push(ExportInfo {
                                            symbol_name: name.to_string(),
                                            export_type: ExportType::Function,
                                            line: impl_node.start_position().row as u32,
                                        });
                                    }
                                }
                            }
                        }
                    });
                }
                _ => {}
            }
        });
        
        exports
    }
}