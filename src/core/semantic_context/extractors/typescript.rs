use anyhow::Result;
use tree_sitter::{Node, Parser};

use crate::core::types::Diagnostic;
use crate::core::semantic_context::types::{
    FunctionContext, ClassContext, ImportContext, TypeDefinition,
    VariableContext, Language, FunctionCall
};
use super::{LanguageExtractor, utils};

pub struct TypeScriptExtractor;

impl TypeScriptExtractor {
    pub fn new() -> Self {
        Self
    }

    fn contains_arrow_function(&self, node: &Node, source: &str) -> bool {
        self.contains_arrow_function_recursive(node, source)
    }

    fn contains_arrow_function_recursive(&self, node: &Node, source: &str) -> bool {
        if node.kind() == "arrow_function" {
            return true;
        }

        // Don't descend into nested functions or classes
        match node.kind() {
            "function_declaration" | "method_definition" | "class_declaration" => {
                return false;
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.contains_arrow_function_recursive(&child, source) {
                return true;
            }
        }

        false
    }

    fn extract_import_names(&self, node: &Node, source: &str) -> Vec<String> {
        let mut names = Vec::new();

        match node.kind() {
            "import_clause" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    names.extend(self.extract_import_names(&child, source));
                }
            }
            "namespace_import" => {
                if let Some(name) = node.child_by_field_name("local") {
                    names.push(utils::node_text(&name, source).to_string());
                }
            }
            "named_imports" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "import_specifier" {
                        if let Some(name) = child.child_by_field_name("local") {
                            names.push(utils::node_text(&name, source).to_string());
                        } else if let Some(name) = child.child_by_field_name("name") {
                            names.push(utils::node_text(&name, source).to_string());
                        }
                    }
                }
            }
            "identifier" => {
                names.push(utils::node_text(node, source).to_string());
            }
            _ => {}
        }

        names
    }
}

impl LanguageExtractor for TypeScriptExtractor {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn get_parser(&self) -> Result<Parser> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_typescript::language_typescript())?;
        Ok(parser)
    }

    fn extract_function_context(&self, node: &Node, source: &str) -> Option<FunctionContext> {
        match node.kind() {
            "function_declaration" | "method_definition" | "arrow_function" => {
                let name = if node.kind() == "arrow_function" {
                    "<arrow function>".to_string()
                } else {
                    node.child_by_field_name("name")
                        .map(|n| utils::node_text(&n, source).to_string())
                        .unwrap_or_else(|| "<anonymous>".to_string())
                };

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
            _ => None,
        }
    }

    fn extract_class_context(&self, node: &Node, source: &str) -> Option<ClassContext> {
        if node.kind() != "class_declaration" {
            return None;
        }

        let name = node.child_by_field_name("name")
            .map(|n| utils::node_text(&n, source).to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());

        let definition = utils::node_text(node, source).to_string();
        let mut methods = Vec::new();
        let mut fields = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for member in body.children(&mut cursor) {
                match member.kind() {
                    "method_definition" => {
                        if let Some(name_node) = member.child_by_field_name("name") {
                            methods.push(utils::node_text(&name_node, source).to_string());
                        }
                    }
                    "public_field_definition" => {
                        if let Some(name_node) = member.child_by_field_name("name") {
                            fields.push(utils::node_text(&name_node, source).to_string());
                        }
                    }
                    _ => {}
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

    fn extract_imports(&self, root: &Node, source: &str) -> Vec<ImportContext> {
        let mut imports = Vec::new();
        let mut cursor = root.walk();

        utils::visit_nodes(&mut cursor, |node| {
            if node.kind() == "import_statement" {
                let statement = utils::node_text(node, source).to_string();
                let imported_names = if let Some(clause) = node.child_by_field_name("import") {
                    self.extract_import_names(&clause, source)
                } else {
                    Vec::new()
                };

                let source_path = node.child_by_field_name("source")
                    .map(|n| utils::node_text(&n, source).trim_matches(|c| c == '"' || c == '\'').to_string())
                    .unwrap_or_default();

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
                "type_alias_declaration" | "interface_declaration" => {
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
                "variable_declarator" => {
                    if let Some(name_node) = n.child_by_field_name("name") {
                        let name = utils::node_text(&name_node, source).to_string();
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
                return;
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
            "function_declaration" | "method_definition" | "arrow_function" | 
            "class_declaration" | "block_statement"
        )
    }

    fn find_enclosing_function<'a>(&self, node: &'a Node<'a>, _source: &str) -> Option<Node<'a>> {
        let mut current = Some(*node);

        while let Some(n) = current {
            match n.kind() {
                "function_declaration" | "method_definition" => {
                    return Some(n);
                }
                "arrow_function" => {
                    // Only return arrow functions that are not inside other functions
                    if !self.contains_arrow_function(&n, _source) {
                        return Some(n);
                    }
                }
                _ => {}
            }
            current = n.parent();
        }

        None
    }

    fn find_enclosing_class<'a>(&self, node: &'a Node<'a>, _source: &str) -> Option<Node<'a>> {
        let mut current = Some(*node);

        while let Some(n) = current {
            if n.kind() == "class_declaration" {
                return Some(n);
            }
            current = n.parent();
        }

        None
    }

    fn extract_function_signature(&self, node: &Node, source: &str) -> String {
        match node.kind() {
            "function_declaration" | "method_definition" => {
                let name = node.child_by_field_name("name")
                    .map(|n| utils::node_text(&n, source))
                    .unwrap_or("<anonymous>");
                
                let params = node.child_by_field_name("parameters")
                    .map(|n| utils::node_text(&n, source))
                    .unwrap_or("()");
                
                let return_type = node.child_by_field_name("return_type")
                    .map(|n| format!(": {}", utils::node_text(&n, source)))
                    .unwrap_or_default();
                
                format!("function {}{}{}", name, params, return_type)
            }
            "arrow_function" => {
                let params = node.child_by_field_name("parameters")
                    .map(|n| utils::node_text(&n, source))
                    .unwrap_or("()");
                
                let return_type = node.child_by_field_name("return_type")
                    .map(|n| format!(": {}", utils::node_text(&n, source)))
                    .unwrap_or_default();
                
                format!("{}{} => ...", params, return_type)
            }
            _ => utils::node_text(node, source).to_string(),
        }
    }

    fn is_builtin_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "string" | "number" | "boolean" | "void" | "any" | "unknown" | 
            "null" | "undefined" | "never" | "object" | "symbol" | "bigint" |
            "Array" | "Promise" | "Map" | "Set" | "Date" | "RegExp" | "Error"
        )
    }
}