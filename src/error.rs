use std::fmt;
use std::path::PathBuf;

/// Main error type for LSPbridge
#[derive(Debug)]
pub enum LspBridgeError {
    /// IO-related errors with context
    Io {
        path: Option<PathBuf>,
        operation: String,
        source: std::io::Error,
    },
    /// Configuration errors
    Config {
        message: String,
        path: Option<PathBuf>,
    },
    /// JSON parsing errors
    Json {
        context: String,
        source: serde_json::Error,
    },
    /// Project structure analysis errors
    ProjectAnalysis {
        message: String,
        path: PathBuf,
    },
    /// LSP communication errors
    LspCommunication {
        message: String,
    },
    /// Database operation errors
    Database {
        operation: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Validation errors
    Validation {
        field: String,
        reason: String,
    },
    /// Generic error with context
    Other {
        message: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl fmt::Display for LspBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, operation, source } => {
                if let Some(p) = path {
                    write!(f, "IO error during {operation} on {p:?}: {source}")
                } else {
                    write!(f, "IO error during {operation}: {source}")
                }
            }
            Self::Config { message, path } => {
                if let Some(p) = path {
                    write!(f, "Configuration error in {p:?}: {message}")
                } else {
                    write!(f, "Configuration error: {message}")
                }
            }
            Self::Json { context, source } => {
                write!(f, "JSON parsing error in {context}: {source}")
            }
            Self::ProjectAnalysis { message, path } => {
                write!(f, "Project analysis error in {path:?}: {message}")
            }
            Self::LspCommunication { message } => {
                write!(f, "LSP communication error: {message}")
            }
            Self::Database { operation, source } => {
                write!(f, "Database error during {operation}: {source}")
            }
            Self::Validation { field, reason } => {
                write!(f, "Validation error for field '{field}': {reason}")
            }
            Self::Other { message, source } => {
                write!(f, "{message}: {source}")
            }
        }
    }
}

impl std::error::Error for LspBridgeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::Database { source, .. } => Some(source.as_ref()),
            Self::Other { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

/// Result type alias for LSPbridge operations
pub type LspResult<T> = Result<T, LspBridgeError>;

/// Helper trait for adding context to errors
pub trait ErrorContext<T> {
    fn io_context(self, operation: &str, path: Option<PathBuf>) -> LspResult<T>;
    fn config_context(self, message: &str, path: Option<PathBuf>) -> LspResult<T>;
    fn json_context(self, context: &str) -> LspResult<T>;
}

impl<T> ErrorContext<T> for Result<T, std::io::Error> {
    fn io_context(self, operation: &str, path: Option<PathBuf>) -> LspResult<T> {
        self.map_err(|e| LspBridgeError::Io {
            path,
            operation: operation.to_string(),
            source: e,
        })
    }
    
    fn config_context(self, _message: &str, _path: Option<PathBuf>) -> LspResult<T> {
        unreachable!("config_context should not be called on io::Error results")
    }
    
    fn json_context(self, _context: &str) -> LspResult<T> {
        unreachable!("json_context should not be called on io::Error results")
    }
}

impl<T> ErrorContext<T> for Result<T, serde_json::Error> {
    fn io_context(self, _operation: &str, _path: Option<PathBuf>) -> LspResult<T> {
        unreachable!("io_context should not be called on serde_json::Error results")
    }
    
    fn config_context(self, _message: &str, _path: Option<PathBuf>) -> LspResult<T> {
        unreachable!("config_context should not be called on serde_json::Error results")
    }
    
    fn json_context(self, context: &str) -> LspResult<T> {
        self.map_err(|e| LspBridgeError::Json {
            context: context.to_string(),
            source: e,
        })
    }
}

// Conversion from anyhow::Error
impl From<anyhow::Error> for LspBridgeError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other {
            message: err.to_string(),
            source: err.into(),
        }
    }
}

// Allow using ? with io::Error
impl From<std::io::Error> for LspBridgeError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            path: None,
            operation: "unspecified".to_string(),
            source: err,
        }
    }
}

// Allow using ? with serde_json::Error
impl From<serde_json::Error> for LspBridgeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json {
            context: "unspecified".to_string(),
            source: err,
        }
    }
}