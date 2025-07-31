use anyhow::Result;
use tree_sitter::{Node, Parser};

use crate::core::types::Diagnostic;
use super::types::{
    FunctionContext, ClassContext, ImportContext, TypeDefinition, 
    VariableContext, Language, FunctionCall
};

pub mod typescript;
pub mod rust;
pub mod python;

/// Trait for language-specific context extraction
pub trait LanguageExtractor: Send + Sync {
    /// Get the language this extractor handles
    fn language(&self) -> Language;

    /// Get or create a parser for this language
    fn get_parser(&self) -> Result<Parser>;

    /// Extract function context from a node
    fn extract_function_context(
        &self,
        node: &Node,
        source: &str,
    ) -> Option<FunctionContext>;

    /// Extract class/struct/interface context
    fn extract_class_context(
        &self,
        node: &Node,
        source: &str,
    ) -> Option<ClassContext>;

    /// Extract imports from the file
    fn extract_imports(
        &self,
        root: &Node,
        source: &str,
    ) -> Vec<ImportContext>;

    /// Extract type definitions
    fn extract_type_definitions(
        &self,
        root: &Node,
        source: &str,
        diagnostic: &Diagnostic,
    ) -> Vec<TypeDefinition>;

    /// Extract local variables in scope
    fn extract_local_variables(
        &self,
        node: &Node,
        source: &str,
        target_line: u32,
    ) -> Vec<VariableContext>;

    /// Extract function calls from a node
    fn extract_function_calls(
        &self,
        node: &Node,
        source: &str,
    ) -> Vec<FunctionCall>;

    /// Check if a node is a scope boundary
    fn is_scope_boundary(&self, node: &Node) -> bool;

    /// Find the enclosing function for a node
    fn find_enclosing_function<'a>(
        &self,
        node: &'a Node<'a>,
        source: &str,
    ) -> Option<Node<'a>>;

    /// Find the enclosing class for a node  
    fn find_enclosing_class<'a>(
        &self,
        node: &'a Node<'a>,
        source: &str,
    ) -> Option<Node<'a>>;

    /// Extract the signature of a function
    fn extract_function_signature(&self, node: &Node, source: &str) -> String;

    /// Check if a type is a built-in type
    fn is_builtin_type(&self, type_name: &str) -> bool;
}

/// Common utilities for extractors
pub mod utils {
    use tree_sitter::Node;

    /// Get the text content of a node
    pub fn node_text<'a>(node: &Node, source: &'a str) -> &'a str {
        &source[node.byte_range()]
    }

    /// Find a node at a specific position
    pub fn find_node_at_position<'a>(
        node: Node<'a>,
        line: u32,
        column: u32,
        source: &str,
    ) -> Option<Node<'a>> {
        let mut cursor = node.walk();
        let mut result = None;
        let mut smallest_range = usize::MAX;

        loop {
            let current_node = cursor.node();
            let start_pos = current_node.start_position();
            let end_pos = current_node.end_position();

            if start_pos.row <= line as usize
                && end_pos.row >= line as usize
                && (start_pos.row < line as usize || start_pos.column <= column as usize)
                && (end_pos.row > line as usize || end_pos.column >= column as usize)
            {
                let range_size = current_node.byte_range().len();
                if range_size < smallest_range {
                    smallest_range = range_size;
                    result = Some(current_node);
                }
            }

            if cursor.goto_first_child() {
                continue;
            }

            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return result;
                }
            }
        }
    }

    /// Visit all nodes in a tree with a callback
    pub fn visit_nodes<F>(cursor: &mut tree_sitter::TreeCursor, mut callback: F)
    where
        F: FnMut(&Node),
    {
        loop {
            let node = cursor.node();
            callback(&node);

            if cursor.goto_first_child() {
                continue;
            }

            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return;
                }
            }
        }
    }
}