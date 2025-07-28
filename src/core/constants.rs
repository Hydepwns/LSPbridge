/// String constants to avoid repeated allocations throughout the codebase.
///
/// This module contains frequently used string literals that would otherwise
/// be allocated repeatedly via .to_string() or String::from() calls.
/// Using these constants reduces memory allocations and improves performance.
/// Language identifiers used across analyzers and processors
pub mod languages {
    pub const TYPESCRIPT: &str = "typescript";
    pub const RUST: &str = "rust";
    pub const PYTHON: &str = "python";
    pub const JAVASCRIPT: &str = "javascript";
    pub const GO: &str = "go";
    pub const JAVA: &str = "java";
    pub const CPP: &str = "cpp";
    pub const C: &str = "c";
}

/// Configuration file names used in project detection and analysis
pub mod config_files {
    pub const PACKAGE_JSON: &str = "package.json";
    pub const CARGO_TOML: &str = "Cargo.toml";
    pub const TSCONFIG_JSON: &str = "tsconfig.json";
    pub const ESLINTRC: &str = ".eslintrc";
    pub const ESLINTRC_JS: &str = ".eslintrc.js";
    pub const ESLINTRC_JSON: &str = ".eslintrc.json";
    pub const PYPROJECT_TOML: &str = "pyproject.toml";
    pub const REQUIREMENTS_TXT: &str = "requirements.txt";
    pub const GO_MOD: &str = "go.mod";
    pub const POM_XML: &str = "pom.xml";
    pub const BUILD_GRADLE: &str = "build.gradle";
    pub const MAKEFILE: &str = "Makefile";
    pub const CMAKE: &str = "CMakeLists.txt";
}

/// Diagnostic severity labels used in export formatting and UI display
pub mod severity_labels {
    pub const ERROR: &str = "Error";
    pub const WARNING: &str = "Warning";
    pub const INFO: &str = "Info";
    pub const INFORMATION: &str = "Information";
    pub const HINT: &str = "Hint";
}

/// Common error message patterns that appear frequently in diagnostics
pub mod error_patterns {
    pub const MISSING_SEMICOLON: &str = "Missing semicolon";
    pub const CANNOT_BORROW_MUTABLE: &str = "cannot borrow as mutable";
    pub const TYPE_MISMATCH: &str = "Type mismatch";
    pub const DOES_NOT_LIVE_LONG_ENOUGH: &str = "does not live long enough";
    pub const VARIABLE_NOT_FOUND: &str = "Variable not found";
    pub const FUNCTION_NOT_FOUND: &str = "Function not found";
    pub const MODULE_NOT_FOUND: &str = "Module not found";
    pub const SYNTAX_ERROR: &str = "Syntax error";
    pub const PARSE_ERROR: &str = "Parse error";
    pub const COMPILATION_ERROR: &str = "Compilation error";
}

/// Build system and package manager identifiers
pub mod build_systems {
    pub const NPM: &str = "npm";
    pub const YARN: &str = "yarn";
    pub const PNPM: &str = "pnpm";
    pub const CARGO: &str = "cargo";
    pub const PIP: &str = "pip";
    pub const POETRY: &str = "poetry";
    pub const MAVEN: &str = "maven";
    pub const GRADLE: &str = "gradle";
    pub const MAKE: &str = "make";
    pub const CMAKE: &str = "cmake";
    pub const GO_BUILD: &str = "go";
}

/// Workspace and project type identifiers
pub mod workspace_types {
    pub const MONOREPO: &str = "monorepo";
    pub const SINGLE_PROJECT: &str = "single-project";
    pub const MULTI_ROOT: &str = "multi-root";
    pub const WORKSPACE: &str = "workspace";
    pub const LIBRARY: &str = "library";
    pub const APPLICATION: &str = "application";
    pub const PACKAGE: &str = "package";
}

/// Common directory and file patterns
pub mod file_patterns {
    pub const NODE_MODULES: &str = "node_modules";
    pub const TARGET: &str = "target";
    pub const BUILD: &str = "build";
    pub const DIST: &str = "dist";
    pub const OUT: &str = "out";
    pub const DOT_GIT: &str = ".git";
    pub const DOT_GITIGNORE: &str = ".gitignore";
    pub const SRC: &str = "src";
    pub const LIB: &str = "lib";
    pub const TESTS: &str = "tests";
    pub const TEST: &str = "test";
    pub const DOCS: &str = "docs";
    pub const EXAMPLES: &str = "examples";
}

/// Metadata keys used in training data and annotations
pub mod metadata_keys {
    pub const DIFFICULTY: &str = "difficulty";
    pub const FIX_QUALITY: &str = "fix_quality";
    pub const LANGUAGE: &str = "language";
    pub const VERSION: &str = "version";
    pub const SOURCE: &str = "source";
    pub const CATEGORY: &str = "category";
    pub const PRIORITY: &str = "priority";
    pub const CONFIDENCE: &str = "confidence";
    pub const WORKSPACE: &str = "workspace";
    pub const TIMESTAMP: &str = "timestamp";
}

/// LSP and editor integration constants
pub mod lsp_constants {
    pub const UNKNOWN: &str = "unknown";
    pub const RUST_ANALYZER: &str = "rust-analyzer";
    pub const TYPESCRIPT_LANGUAGE_SERVER: &str = "typescript-language-server";
    pub const PYLSP: &str = "pylsp";
    pub const GOPLS: &str = "gopls";
    pub const CLANGD: &str = "clangd";
    pub const JDTLS: &str = "jdtls";
}

/// Export format identifiers
pub mod export_formats {
    pub const JSON: &str = "json";
    pub const MARKDOWN: &str = "markdown";
    pub const HTML: &str = "html";
    pub const XML: &str = "xml";
    pub const CSV: &str = "csv";
    pub const YAML: &str = "yaml";
    pub const TOML: &str = "toml";
}

/// Capture method identifiers
pub mod capture_methods {
    pub const MANUAL: &str = "manual";
    pub const AUTOMATIC: &str = "automatic";
    pub const SCHEDULED: &str = "scheduled";
}
