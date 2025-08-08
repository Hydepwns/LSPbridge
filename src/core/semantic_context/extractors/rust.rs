use anyhow::Result;
use tree_sitter::{Node, Parser};

use crate::core::types::Diagnostic;
use crate::core::semantic_context::types::{
    FunctionContext, ClassContext, ImportContext, TypeDefinition,
    VariableContext, Language, FunctionCall
};
use super::{LanguageExtractor, utils};

pub struct RustExtractor;

impl Default for RustExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl RustExtractor {
    pub fn new() -> Self {
        Self
    }

    fn extract_use_names(&self, node: &Node, source: &str) -> Vec<String> {
        let mut names = Vec::new();

        match node.kind() {
            "use_list" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    names.extend(self.extract_use_names(&child, source));
                }
            }
            "use_as_clause" => {
                if let Some(alias) = node.child_by_field_name("alias") {
                    names.push(utils::node_text(&alias, source).to_string());
                } else if let Some(name) = node.child_by_field_name("name") {
                    names.push(utils::node_text(&name, source).to_string());
                }
            }
            "identifier" | "scoped_identifier" => {
                names.push(utils::node_text(node, source).to_string());
            }
            _ => {}
        }

        names
    }
}

impl LanguageExtractor for RustExtractor {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn get_parser(&self) -> Result<Parser> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language())?;
        Ok(parser)
    }

    fn extract_function_context(&self, node: &Node, source: &str) -> Option<FunctionContext> {
        if node.kind() != "function_item" {
            return None;
        }

        let name = node.child_by_field_name("name")
            .map(|n| utils::node_text(&n, source).to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());

        let signature = self.extract_function_signature(node, source);
        let body = utils::node_text(node, source).to_string();

        Some(FunctionContext {
            name,
            signature,
            body,
            start_line: node.start_position().row as u32,
            end_line: node.end_position().row as u32,
        })
    }

    fn extract_class_context(&self, node: &Node, source: &str) -> Option<ClassContext> {
        match node.kind() {
            "struct_item" | "impl_item" => {
                let name = if node.kind() == "struct_item" {
                    node.child_by_field_name("name")
                        .map(|n| utils::node_text(&n, source).to_string())
                        .unwrap_or_else(|| "<anonymous>".to_string())
                } else {
                    // For impl blocks, get the type name
                    node.child_by_field_name("type")
                        .map(|n| utils::node_text(&n, source).to_string())
                        .unwrap_or_else(|| "<anonymous>".to_string())
                };

                let definition = utils::node_text(node, source).to_string();
                let mut methods = Vec::new();
                let mut fields = Vec::new();

                if node.kind() == "struct_item" {
                    if let Some(body) = node.child_by_field_name("body") {
                        let mut cursor = body.walk();
                        for field in body.children(&mut cursor) {
                            if field.kind() == "field_declaration" {
                                if let Some(name_node) = field.child_by_field_name("name") {
                                    fields.push(utils::node_text(&name_node, source).to_string());
                                }
                            }
                        }
                    }
                } else if node.kind() == "impl_item" {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "function_item" {
                            if let Some(name_node) = child.child_by_field_name("name") {
                                methods.push(utils::node_text(&name_node, source).to_string());
                            }
                        }
                    }
                }

                Some(ClassContext {
                    name,
                    definition,
                    methods,
                    fields,
                    start_line: node.start_position().row as u32,
                    end_line: node.end_position().row as u32,
                })
            }
            _ => None,
        }
    }

    fn extract_imports(&self, root: &Node, source: &str) -> Vec<ImportContext> {
        let mut imports = Vec::new();
        let mut cursor = root.walk();

        utils::visit_nodes(&mut cursor, |node| {
            if node.kind() == "use_declaration" {
                let statement = utils::node_text(node, source).to_string();
                let imported_names = if let Some(tree) = node.child_by_field_name("tree") {
                    self.extract_use_names(&tree, source)
                } else {
                    Vec::new()
                };

                let source_path = statement
                    .trim_start_matches("use ")
                    .trim_end_matches(';')
                    .split("::")
                    .next()
                    .unwrap_or("")
                    .to_string();

                imports.push(ImportContext {
                    statement,
                    imported_names,
                    source: source_path,
                    line: node.start_position().row as u32,
                });
            }
        });

        imports
    }

    fn extract_type_definitions(&self, root: &Node, source: &str, _diagnostic: &Diagnostic) -> Vec<TypeDefinition> {
        let mut types = Vec::new();
        let mut cursor = root.walk();

        utils::visit_nodes(&mut cursor, |node| {
            match node.kind() {
                "type_item" | "struct_item" | "enum_item" => {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        let name = utils::node_text(&name_node, source).to_string();
                        let definition = utils::node_text(node, source).to_string();
                        
                        types.push(TypeDefinition {
                            name,
                            definition,
                            line: node.start_position().row as u32,
                        });
                    }
                }
                _ => {}
            }
        });

        types
    }

    fn extract_local_variables(&self, node: &Node, source: &str, target_line: u32) -> Vec<VariableContext> {
        let mut variables = Vec::new();
        let mut cursor = node.walk();

        utils::visit_nodes(&mut cursor, |n| {
            if n.start_position().row > target_line as usize {
                return;
            }

            match n.kind() {
                "let_declaration" => {
                    if let Some(pattern) = n.child_by_field_name("pattern") {
                        let name = utils::node_text(&pattern, source).to_string();
                        let type_annotation = n.child_by_field_name("type")
                            .map(|t| utils::node_text(&t, source).to_string());
                        let value = n.child_by_field_name("value")
                            .map(|v| utils::node_text(&v, source).to_string());

                        variables.push(VariableContext {
                            name,
                            type_annotation,
                            value,
                            line: n.start_position().row as u32,
                        });
                    }
                }
                "parameter" => {
                    if let Some(pattern) = n.child_by_field_name("pattern") {
                        let name = utils::node_text(&pattern, source).to_string();
                        let type_annotation = n.child_by_field_name("type")
                            .map(|t| utils::node_text(&t, source).to_string());

                        variables.push(VariableContext {
                            name,
                            type_annotation,
                            value: None,
                            line: n.start_position().row as u32,
                        });
                    }
                }
                _ => {}
            }

            if self.is_scope_boundary(n) {
            }
        });

        variables
    }

    fn extract_function_calls(&self, node: &Node, source: &str) -> Vec<FunctionCall> {
        let mut calls = Vec::new();
        let mut cursor = node.walk();

        utils::visit_nodes(&mut cursor, |n| {
            if n.kind() == "call_expression" {
                if let Some(function_node) = n.child_by_field_name("function") {
                    let function_name = utils::node_text(&function_node, source).to_string();
                    let arguments = n.child_by_field_name("arguments")
                        .map(|args| {
                            let mut arg_list = Vec::new();
                            let mut arg_cursor = args.walk();
                            for arg in args.children(&mut arg_cursor) {
                                if arg.kind() != "," && arg.kind() != "(" && arg.kind() != ")" {
                                    arg_list.push(utils::node_text(&arg, source).to_string());
                                }
                            }
                            arg_list
                        })
                        .unwrap_or_default();

                    calls.push(FunctionCall {
                        function_name,
                        file_path: String::new(), // To be filled by the caller
                        line: n.start_position().row as u32,
                        arguments,
                        is_direct: true,
                    });
                }
            }
        });

        calls
    }

    fn is_scope_boundary(&self, node: &Node) -> bool {
        matches!(
            node.kind(),
            "function_item" | "closure_expression" | "block" | 
            "match_expression" | "if_expression" | "while_expression" | 
            "for_expression" | "loop_expression"
        )
    }

    fn find_enclosing_function<'a>(&self, node: &'a Node<'a>, _source: &str) -> Option<Node<'a>> {
        let mut current = Some(*node);

        while let Some(n) = current {
            if n.kind() == "function_item" {
                return Some(n);
            }
            current = n.parent();
        }

        None
    }

    fn find_enclosing_class<'a>(&self, node: &'a Node<'a>, _source: &str) -> Option<Node<'a>> {
        let mut current = Some(*node);

        while let Some(n) = current {
            match n.kind() {
                "struct_item" | "impl_item" => return Some(n),
                _ => {}
            }
            current = n.parent();
        }

        None
    }

    fn extract_function_signature(&self, node: &Node, source: &str) -> String {
        if node.kind() != "function_item" {
            return utils::node_text(node, source).to_string();
        }

        let visibility = node.child(0)
            .filter(|n| n.kind() == "visibility_modifier")
            .map(|n| format!("{} ", utils::node_text(&n, source)))
            .unwrap_or_default();

        let name = node.child_by_field_name("name")
            .map(|n| utils::node_text(&n, source))
            .unwrap_or("<anonymous>");

        let params = node.child_by_field_name("parameters")
            .map(|n| utils::node_text(&n, source))
            .unwrap_or("()");

        let return_type = node.child_by_field_name("return_type")
            .map(|n| format!(" {}", utils::node_text(&n, source)))
            .unwrap_or_default();

        format!("{visibility}fn {name}{params}{return_type}")
    }

    fn is_builtin_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
            "f32" | "f64" | "bool" | "char" | "str" | "String" |
            "Vec" | "HashMap" | "HashSet" | "Option" | "Result" |
            "Box" | "Rc" | "Arc" | "RefCell" | "Mutex" | "RwLock"
        )
    }
}