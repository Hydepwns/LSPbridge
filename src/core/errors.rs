/// Domain-specific error types for LSPbridge using thiserror
///
/// This module provides structured error types that replace generic anyhow::Error
/// usage in specific domains. These error types enable better error matching,
/// recovery strategies, and integration with the sophisticated error recovery system.
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for LSPbridge operations
#[derive(Error, Debug)]
pub enum LSPBridgeError {
    #[error("File operation failed")]
    File(#[from] FileError),

    #[error("Parsing failed")]
    Parse(#[from] ParseError),

    #[error("Processing failed")]
    Processing(#[from] ProcessingError),

    #[error("Database operation failed")]
    Database(#[from] DatabaseError),

    #[error("Configuration error")]
    Config(#[from] ConfigError),

    #[error("Export operation failed")]
    Export(#[from] ExportError),

    #[error("Analysis failed")]
    Analysis(#[from] AnalysisError),

    #[error("Cache operation failed")]
    Cache(#[from] CacheError),
}

/// File operation errors
#[derive(Error, Debug)]
pub enum FileError {
    #[error("Failed to read file {path}: {reason}")]
    ReadFailed {
        path: PathBuf,
        reason: String,
        #[source]
        source: io::Error,
    },

    #[error("Failed to write file {path}: {reason}")]
    WriteFailed {
        path: PathBuf,
        reason: String,
        #[source]
        source: io::Error,
    },

    #[error("File not found: {path}")]
    NotFound { path: PathBuf },

    #[error("Permission denied for {path}")]
    PermissionDenied {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Invalid file path: {path}")]
    InvalidPath { path: String },

    #[error("File already exists: {path}")]
    AlreadyExists { path: PathBuf },

    #[error("Directory operation failed on {path}: {operation}")]
    DirectoryError {
        path: PathBuf,
        operation: String,
        #[source]
        source: io::Error,
    },
}

/// Parsing errors for various formats
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("JSON parsing failed in {context}: {message}")]
    Json {
        context: String,
        message: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("TOML parsing failed in {context}: {message}")]
    Toml {
        context: String,
        message: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("YAML parsing failed in {context}: {message}")]
    Yaml {
        context: String,
        message: String,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("Query parsing failed: {query} - {message}")]
    Query { query: String, message: String },

    #[error("Regex compilation failed: {pattern}")]
    Regex {
        pattern: String,
        #[source]
        source: regex::Error,
    },

    #[error("Invalid format in {context}: expected {expected}, found {found}")]
    InvalidFormat {
        context: String,
        expected: String,
        found: String,
    },

    #[error("Syntax error in {file} at line {line}: {message}")]
    Syntax {
        file: PathBuf,
        line: u32,
        message: String,
    },

    // Query language parsing errors
    #[error("Unexpected token: expected {expected}, found '{found}' at line {line}, column {column}")]
    UnexpectedToken {
        expected: String,
        found: String,
        line: usize,
        column: usize,
    },

    #[error("Unexpected character '{character}' at line {line}, column {column}")]
    UnexpectedCharacter {
        character: char,
        line: usize,
        column: usize,
    },

    #[error("Unterminated string literal at line {line}, column {column}")]
    UnterminatedString {
        line: usize,
        column: usize,
    },

    #[error("Invalid number '{value}' at line {line}, column {column}")]
    InvalidNumber {
        value: String,
        line: usize,
        column: usize,
    },

    #[error("Invalid time format '{value}' at line {line}, column {column}")]
    InvalidTimeFormat {
        value: String,
        line: usize,
        column: usize,
    },

    #[error("Unknown field '{field}'. Available fields: {available_fields:?}")]
    UnknownField {
        field: String,
        available_fields: Vec<String>,
    },

    #[error("Invalid aggregation: {function} on field '{field}' - {reason}")]
    InvalidAggregation {
        function: String,
        field: String,
        reason: String,
    },

    #[error("Missing GROUP BY clause: {reason}")]
    MissingGroupBy {
        reason: String,
    },

    #[error("Conflicting time range specification: {reason}")]
    ConflictingTimeRange {
        reason: String,
    },

    #[error("Invalid time range: {reason}")]
    InvalidTimeRange {
        reason: String,
    },

    #[error("Invalid LIMIT value {limit}: {reason}")]
    InvalidLimit {
        limit: u32,
        reason: String,
    },

    #[error("Incompatible data source '{data_source}' with field '{field}': {reason}")]
    IncompatibleDataSource {
        data_source: String,
        field: String,
        reason: String,
    },

    // Additional query parsing errors for the grammar module
    #[error("Unknown table '{table}' at line {line}, column {column}")]
    UnknownTable {
        table: String,
        line: usize,
        column: usize,
    },

    #[error("Invalid severity '{severity}' at line {line}, column {column}")]
    InvalidSeverity {
        severity: String,
        line: usize,
        column: usize,
    },

    #[error("Invalid datetime '{value}' at line {line}, column {column}")]
    InvalidDateTime {
        value: String,
        line: usize,
        column: usize,
    },

    #[error("Invalid boolean '{value}' at line {line}, column {column}")]
    InvalidBoolean {
        value: String,
        line: usize,
        column: usize,
    },

    #[error("Empty GROUP BY clause")]
    EmptyGroupBy,

    #[error("Empty ORDER BY clause")]
    EmptyOrderBy,

    #[error("Empty pattern for {filter_type} filter at line {line}, column {column}")]
    EmptyPattern {
        filter_type: String,
        line: usize,
        column: usize,
    },

    #[error("Empty field name at line {line}, column {column}")]
    EmptyFieldName {
        line: usize,
        column: usize,
    },

    #[error("Invalid relative time: {value} {unit} - {reason}")]
    InvalidRelativeTime {
        value: u32,
        unit: String,
        reason: String,
    },



    #[error("Incompatible clauses: {clause1} cannot be used with {clause2} - {reason}")]
    IncompatibleClauses {
        clause1: String,
        clause2: String,
        reason: String,
    },

    #[error("Invalid ORDER BY field '{field}': not in SELECT list. Available fields: {available_fields:?}")]
    InvalidOrderByField {
        field: String,
        available_fields: Vec<String>,
    },
}

/// Processing errors for analyzers and processors
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("Analysis failed for {file}: {reason}")]
    AnalysisFailed {
        file: PathBuf,
        reason: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Language detection failed for {file}")]
    LanguageDetectionFailed { file: PathBuf },

    #[error("Incremental processing failed: {reason}")]
    IncrementalFailed {
        reason: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Memory limit exceeded: used {used_mb}MB, limit {limit_mb}MB")]
    MemoryLimitExceeded { used_mb: usize, limit_mb: usize },

    #[error("Processing timeout after {duration_secs}s for {operation}")]
    Timeout {
        operation: String,
        duration_secs: u64,
    },

    #[error("Dependency resolution failed for {file}: {reason}")]
    DependencyResolution { file: PathBuf, reason: String },

    #[error("Context extraction failed for {file}: {reason}")]
    ContextExtraction { file: PathBuf, reason: String },

    #[error("Semantic analysis failed for {file}: {reason}")]
    SemanticAnalysis { file: PathBuf, reason: String },
}

/// Database operation errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SQLite error in {operation}: {message}")]
    Sqlite {
        operation: String,
        message: String,
        #[source]
        source: rusqlite::Error,
    },

    #[error("Sled database error in {operation}: {message}")]
    Sled {
        operation: String,
        message: String,
        #[source]
        source: sled::Error,
    },

    #[error("Serialization failed for {data_type}: {reason}")]
    Serialization {
        data_type: String,
        reason: String,
        #[source]
        source: bincode::Error,
    },

    #[error("Migration failed from version {from} to {to}: {reason}")]
    Migration { from: u32, to: u32, reason: String },

    #[error("Database corruption detected: {details}")]
    Corruption { details: String },

    #[error("Transaction failed: {reason}")]
    Transaction { reason: String },

    #[error("Query execution failed: {query}")]
    QueryExecution {
        query: String,
        #[source]
        source: rusqlite::Error,
    },

    #[error("Connection error in {operation}: {details:?}")]
    Connection {
        operation: String,
        details: Option<String>,
    },
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid configuration value for {field}: {value} - {reason}")]
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },

    #[error("Missing required configuration field: {field}")]
    MissingField { field: String },

    #[error("Configuration file not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Configuration validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Incompatible configuration version: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },

    #[error("Dynamic configuration update failed for {field}: {reason}")]
    DynamicUpdateFailed { field: String, reason: String },
}

/// Export operation errors
#[derive(Error, Debug)]
pub enum ExportError {
    #[error("Export format not supported: {format}")]
    UnsupportedFormat { format: String },

    #[error(transparent)]
    Other(#[from] anyhow::Error),

    #[error("Template rendering failed for {template}: {reason}")]
    TemplateRendering { template: String, reason: String },

    #[error("Data transformation failed from {from_format} to {to_format}: {reason}")]
    DataTransformation {
        from_format: String,
        to_format: String,
        reason: String,
    },

    #[error("Export target unreachable: {target}")]
    TargetUnreachable { target: String },

    #[error("Insufficient data for export: {reason}")]
    InsufficientData { reason: String },

    #[error("Export size limit exceeded: {size_mb}MB > {limit_mb}MB")]
    SizeLimitExceeded { size_mb: usize, limit_mb: usize },
}

/// Analysis errors
#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("Language analyzer failed for {language}: {reason}")]
    LanguageAnalyzer { language: String, reason: String },

    #[error("Diagnostic categorization failed: {reason}")]
    Categorization { reason: String },

    #[error("Pattern matching failed for {pattern}: {reason}")]
    PatternMatching { pattern: String, reason: String },

    #[error("Fix suggestion generation failed: {reason}")]
    FixSuggestion { reason: String },

    #[error("Code complexity analysis failed for {file}: {reason}")]
    ComplexityAnalysis { file: PathBuf, reason: String },

    #[error("Symbol resolution failed for {symbol} in {file}")]
    SymbolResolution { symbol: String, file: PathBuf },
}

/// Cache operation errors
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache miss for key: {key}")]
    Miss { key: String },

    #[error("Cache eviction failed: {reason}")]
    EvictionFailed { reason: String },

    #[error("Cache serialization failed for key {key}: {reason}")]
    SerializationFailed { key: String, reason: String },

    #[error("Cache corruption detected: {details}")]
    Corruption { details: String },

    #[error("Cache capacity exceeded: {current_size}MB > {max_size}MB")]
    CapacityExceeded {
        current_size: usize,
        max_size: usize,
    },

    #[error("Cache initialization failed: {reason}")]
    InitializationFailed { reason: String },
}

// From trait implementations for error conversions

impl From<ParseError> for ConfigError {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::Toml {
                context, message, ..
            } => ConfigError::ValidationFailed {
                reason: format!("TOML error in {}: {}", context, message),
            },
            ParseError::Json {
                context, message, ..
            } => ConfigError::ValidationFailed {
                reason: format!("JSON error in {}: {}", context, message),
            },
            _ => ConfigError::ValidationFailed {
                reason: err.to_string(),
            },
        }
    }
}

impl From<FileError> for ConfigError {
    fn from(err: FileError) -> Self {
        match err {
            FileError::NotFound { path } => ConfigError::FileNotFound { path },
            FileError::ReadFailed { path, reason, .. } => ConfigError::ValidationFailed {
                reason: format!("Failed to read config file {}: {}", path.display(), reason),
            },
            FileError::WriteFailed { path, reason, .. } => ConfigError::ValidationFailed {
                reason: format!("Failed to write config file {}: {}", path.display(), reason),
            },
            _ => ConfigError::ValidationFailed {
                reason: err.to_string(),
            },
        }
    }
}

impl From<std::num::ParseIntError> for ConfigError {
    fn from(err: std::num::ParseIntError) -> Self {
        ConfigError::InvalidValue {
            field: "numeric_field".to_string(),
            value: "invalid_number".to_string(),
            reason: err.to_string(),
        }
    }
}

impl From<std::num::ParseFloatError> for ConfigError {
    fn from(err: std::num::ParseFloatError) -> Self {
        ConfigError::InvalidValue {
            field: "float_field".to_string(),
            value: "invalid_float".to_string(),
            reason: err.to_string(),
        }
    }
}

impl From<std::str::ParseBoolError> for ConfigError {
    fn from(err: std::str::ParseBoolError) -> Self {
        ConfigError::InvalidValue {
            field: "boolean_field".to_string(),
            value: "invalid_boolean".to_string(),
            reason: err.to_string(),
        }
    }
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(err: rusqlite::Error) -> Self {
        let message = err.to_string();
        DatabaseError::Sqlite {
            operation: "database_operation".to_string(),
            message,
            source: err,
        }
    }
}

impl From<serde_json::Error> for DatabaseError {
    fn from(err: serde_json::Error) -> Self {
        DatabaseError::Serialization {
            data_type: "JSON".to_string(),
            reason: err.to_string(),
            source: bincode::ErrorKind::Custom(format!("JSON error: {}", err)).into(),
        }
    }
}

impl From<std::io::Error> for DatabaseError {
    fn from(err: std::io::Error) -> Self {
        DatabaseError::Serialization {
            data_type: "IO".to_string(),
            reason: err.to_string(),
            source: bincode::ErrorKind::Custom(format!("IO error: {}", err)).into(),
        }
    }
}

impl From<sled::Error> for CacheError {
    fn from(err: sled::Error) -> Self {
        match err {
            sled::Error::Corruption { .. } => CacheError::Corruption {
                details: err.to_string(),
            },
            sled::Error::ReportableBug(_) => CacheError::InitializationFailed {
                reason: format!("Sled database bug: {}", err),
            },
            _ => CacheError::SerializationFailed {
                key: "unknown".to_string(),
                reason: err.to_string(),
            },
        }
    }
}

impl From<bincode::Error> for CacheError {
    fn from(err: bincode::Error) -> Self {
        CacheError::SerializationFailed {
            key: "unknown".to_string(),
            reason: err.to_string(),
        }
    }
}

/// Utility functions for error handling
impl LSPBridgeError {
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            LSPBridgeError::File(FileError::NotFound { .. }) => false,
            LSPBridgeError::File(FileError::PermissionDenied { .. }) => false,
            LSPBridgeError::Processing(ProcessingError::MemoryLimitExceeded { .. }) => false,
            LSPBridgeError::Config(ConfigError::ValidationFailed { .. }) => false,
            _ => true,
        }
    }
}

/// Helper functions for common error patterns
impl FileError {
    pub fn read_error(path: PathBuf, source: io::Error) -> Self {
        FileError::ReadFailed {
            path,
            reason: source.to_string(),
            source,
        }
    }

    pub fn write_error(path: PathBuf, source: io::Error) -> Self {
        FileError::WriteFailed {
            path,
            reason: source.to_string(),
            source,
        }
    }
}

impl ParseError {
    pub fn json_error(context: impl Into<String>, source: serde_json::Error) -> Self {
        ParseError::Json {
            context: context.into(),
            message: source.to_string(),
            source,
        }
    }

    pub fn toml_error(context: impl Into<String>, source: toml::de::Error) -> Self {
        ParseError::Toml {
            context: context.into(),
            message: source.to_string(),
            source,
        }
    }
}

impl From<std::time::SystemTimeError> for DatabaseError {
    fn from(err: std::time::SystemTimeError) -> Self {
        DatabaseError::Transaction {
            reason: format!("Time error: {}", err),
        }
    }
}
