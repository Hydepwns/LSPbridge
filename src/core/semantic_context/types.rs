use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Semantic context around a diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub surrounding_code: HashMap<String, String>,
}

/// Function/method context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionContext {
    pub name: String,
    pub signature: String,
    pub body: String,
    pub start_line: u32,
    pub end_line: u32,
}

/// Class/struct/interface context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassContext {
    pub name: String,
    pub definition: String,
    pub methods: Vec<String>,
    pub fields: Vec<String>,
    pub start_line: u32,
    pub end_line: u32,
}

/// Import/use statement context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportContext {
    pub statement: String,
    pub imported_names: Vec<String>,
    pub source: String,
    pub line: u32,
}

/// Type definition context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub definition: String,
    pub line: u32,
}

/// Variable context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableContext {
    pub name: String,
    pub type_annotation: Option<String>,
    pub value: Option<String>,
    pub line: u32,
}

/// Call hierarchy information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchy {
    /// Functions that call the current function
    pub callers: Vec<FunctionCall>,
    /// Functions called by the current function
    pub callees: Vec<FunctionCall>,
    /// Maximum depth explored
    pub depth: u32,
}

/// Information about a function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub function_name: String,
    pub file_path: String,
    pub line: u32,
    pub arguments: Vec<String>,
    /// Whether this is a direct call or indirect (via callback/pointer)
    pub is_direct: bool,
}

/// Cross-file dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub file_path: String,
    pub imported_symbols: Vec<String>,
    pub export_symbols: Vec<String>,
    pub dependency_type: DependencyType,
}

/// Type of dependency relationship
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyType {
    /// Direct import/require
    Direct,
    /// Transitive dependency
    Transitive,
    /// Type-only import (TypeScript)
    TypeOnly,
    /// Dynamic import
    Dynamic,
    /// Re-export
    ReExport,
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    TypeScript,
    JavaScript,
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
            surrounding_code: HashMap::new(),
        }
    }
}

impl Default for CallHierarchy {
    fn default() -> Self {
        Self {
            callers: Vec::new(),
            callees: Vec::new(),
            depth: 0,
        }
    }
}