use anyhow::Result;
use tree_sitter::{Node, Parser};

use crate::core::types::Diagnostic;
use crate::core::semantic_context::types::{
    FunctionContext, ClassContext, ImportContext, TypeDefinition,
    VariableContext, Language, FunctionCall
};
use super::{LanguageExtractor, utils};

pub struct PythonExtractor;

impl Default for PythonExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonExtractor {
    pub fn new() -> Self {
        Self
    }

    fn extract_import_names(&self, node: &Node, source: &str) -> Vec<String> {
        let mut names = Vec::new();

        match node.kind() {
            "dotted_name" => {
                names.push(utils::node_text(node, source).to_string());
            }
            "aliased_import" => {
                if let Some(alias) = node.child_by_field_name("alias") {
                    names.push(utils::node_text(&alias, source).to_string());
                } else if let Some(name) = node.child_by_field_name("name") {
                    names.push(utils::node_text(&name, source).to_string());
                }
            }
            "identifier" => {
                names.push(utils::node_text(node, source).to_string());
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    names.extend(self.extract_import_names(&child, source));
                }
            }
        }

        names
    }
}

impl LanguageExtractor for PythonExtractor {
    fn language(&self) -> Language {
        Language::Python
    }

    fn get_parser(&self) -> Result<Parser> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_python::language())?;
        Ok(parser)
    }

    fn extract_function_context(&self, node: &Node, source: &str) -> Option<FunctionContext> {
        if node.kind() != "function_definition" {
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
        if node.kind() != "class_definition" {
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
                    "function_definition" => {
                        if let Some(name_node) = member.child_by_field_name("name") {
                            methods.push(utils::node_text(&name_node, source).to_string());
                        }
                    }
                    "expression_statement" => {
                        // Look for self.field = value patterns
                        if let Some(expr) = member.child(0) {
                            if expr.kind() == "assignment" {
                                if let Some(left) = expr.child_by_field_name("left") {
                                    if left.kind() == "attribute" {
                                        if let Some(attr) = left.child_by_field_name("attribute") {
                                            fields.push(utils::node_text(&attr, source).to_string());
                                        }
                                    }
                                }
                            }
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
            match node.kind() {
                "import_statement" | "import_from_statement" => {
                    let statement = utils::node_text(node, source).to_string();
                    let imported_names = if node.kind() == "import_statement" {
                        let mut names = Vec::new();
                        let mut import_cursor = node.walk();
                        for child in node.children(&mut import_cursor) {
                            if child.kind() == "dotted_name" || child.kind() == "aliased_import" {
                                names.extend(self.extract_import_names(&child, source));
                            }
                        }
                        names
                    } else {
                        // import_from_statement
                        let mut names = Vec::new();
                        if let Some(name_list) = node.child_by_field_name("name") {
                            names.extend(self.extract_import_names(&name_list, source));
                        }
                        names
                    };

                    let source_module = if node.kind() == "import_from_statement" {
                        node.child_by_field_name("module")
                            .map(|m| utils::node_text(&m, source).to_string())
                            .unwrap_or_default()
                    } else {
                        imported_names.first().cloned().unwrap_or_default()
                    };

                    imports.push(ImportContext {
                        statement,
                        imported_names,
                        source: source_module,
                        line: node.start_position().row as u32,
                    });
                }
                _ => {}
            }
        });

        imports
    }

    fn extract_type_definitions(&self, root: &Node, source: &str, _diagnostic: &Diagnostic) -> Vec<TypeDefinition> {
        let mut types = Vec::new();
        let mut cursor = root.walk();

        utils::visit_nodes(&mut cursor, |node| {
            // Python doesn't have explicit type definitions like TypeScript/Rust
            // We look for TypedDict, NamedTuple, and type aliases
            if node.kind() == "assignment" {
                if let Some(left) = node.child_by_field_name("left") {
                    if left.kind() == "identifier" {
                        let name = utils::node_text(&left, source);
                        if let Some(right) = node.child_by_field_name("right") {
                            let right_text = utils::node_text(&right, source);
                            // Simple heuristic for type aliases
                            if right_text.contains("TypedDict") || 
                               right_text.contains("NamedTuple") ||
                               right_text.contains("Union[") ||
                               right_text.contains("Optional[") ||
                               right_text.contains("List[") ||
                               right_text.contains("Dict[") {
                                types.push(TypeDefinition {
                                    name: name.to_string(),
                                    definition: utils::node_text(node, source).to_string(),
                                    line: node.start_position().row as u32,
                                });
                            }
                        }
                    }
                }
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
                "assignment" => {
                    if let Some(left) = n.child_by_field_name("left") {
                        if left.kind() == "identifier" {
                            let name = utils::node_text(&left, source).to_string();
                            let value = n.child_by_field_name("right")
                                .map(|v| utils::node_text(&v, source).to_string());
                            
                            // Look for type comments
                            let type_annotation = n.child_by_field_name("type")
                                .map(|t| utils::node_text(&t, source).to_string());

                            variables.push(VariableContext {
                                name,
                                type_annotation,
                                value,
                                line: n.start_position().row as u32,
                            });
                        }
                    }
                }
                "parameters" => {
                    let mut param_cursor = n.walk();
                    for param in n.children(&mut param_cursor) {
                        if param.kind() == "identifier" || param.kind() == "typed_parameter" {
                            let name = if param.kind() == "identifier" {
                                utils::node_text(&param, source).to_string()
                            } else {
                                param.child_by_field_name("identifier")
                                    .map(|id| utils::node_text(&id, source).to_string())
                                    .unwrap_or_default()
                            };

                            let type_annotation = param.child_by_field_name("type")
                                .map(|t| utils::node_text(&t, source).to_string());

                            variables.push(VariableContext {
                                name,
                                type_annotation,
                                value: None,
                                line: param.start_position().row as u32,
                            });
                        }
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
            if n.kind() == "call" {
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
            "function_definition" | "class_definition" | "for_statement" | 
            "while_statement" | "with_statement" | "if_statement" | 
            "try_statement" | "lambda"
        )
    }

    fn find_enclosing_function<'a>(&self, node: &'a Node<'a>, _source: &str) -> Option<Node<'a>> {
        let mut current = Some(*node);

        while let Some(n) = current {
            if n.kind() == "function_definition" {
                return Some(n);
            }
            current = n.parent();
        }

        None
    }

    fn find_enclosing_class<'a>(&self, node: &'a Node<'a>, _source: &str) -> Option<Node<'a>> {
        let mut current = Some(*node);

        while let Some(n) = current {
            if n.kind() == "class_definition" {
                return Some(n);
            }
            current = n.parent();
        }

        None
    }

    fn extract_function_signature(&self, node: &Node, source: &str) -> String {
        if node.kind() != "function_definition" {
            return utils::node_text(node, source).to_string();
        }

        let decorators = {
            let mut dec_list = Vec::new();
            let mut sibling = node.prev_sibling();
            while let Some(s) = sibling {
                if s.kind() == "decorator" {
                    dec_list.push(utils::node_text(&s, source));
                    sibling = s.prev_sibling();
                } else {
                    break;
                }
            }
            dec_list.reverse();
            dec_list
        };

        let name = node.child_by_field_name("name")
            .map(|n| utils::node_text(&n, source))
            .unwrap_or("<anonymous>");

        let params = node.child_by_field_name("parameters")
            .map(|n| utils::node_text(&n, source))
            .unwrap_or("()");

        let return_type = node.child_by_field_name("return_type")
            .map(|n| format!(" -> {}", utils::node_text(&n, source)))
            .unwrap_or_default();

        let decorator_str = if !decorators.is_empty() {
            format!("{}\n", decorators.join("\n"))
        } else {
            String::new()
        };

        format!("{decorator_str}def {name}{params}{return_type}")
    }

    fn is_builtin_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "int" | "float" | "str" | "bool" | "bytes" | "bytearray" |
            "list" | "tuple" | "dict" | "set" | "frozenset" |
            "None" | "object" | "type" | "callable" | "any" |
            "List" | "Tuple" | "Dict" | "Set" | "Optional" | "Union" |
            "Any" | "Callable" | "TypeVar" | "Generic"
        )
    }
}