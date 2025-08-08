//! Language-specific error code enums
//!
//! This module provides strongly-typed error codes for different language servers,
//! replacing string-based error code matching with type-safe enums.

use std::fmt;

/// TypeScript/JavaScript error codes from the TypeScript compiler
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeScriptErrorCode {
    /// Property does not exist on type
    PropertyDoesNotExist = 2339,
    /// Property does not exist on type (with suggestion)
    PropertyDoesNotExistWithSuggestion = 2551,
    /// Type is not assignable to type
    TypeNotAssignable = 2322,
    /// Argument of type X is not assignable to parameter of type Y
    ArgumentTypeNotAssignable = 2345,
    /// Cannot find name
    CannotFindName = 2304,
    /// Cannot find name (with suggestion)
    CannotFindNameWithSuggestion = 2552,
    /// Generic type requires type arguments
    GenericTypeRequiresArguments = 2314,
}

impl TypeScriptErrorCode {
    /// Parse from a string error code
    pub fn from_str(code: &str) -> Option<Self> {
        match code {
            "2339" => Some(Self::PropertyDoesNotExist),
            "2551" => Some(Self::PropertyDoesNotExistWithSuggestion),
            "2322" => Some(Self::TypeNotAssignable),
            "2345" => Some(Self::ArgumentTypeNotAssignable),
            "2304" => Some(Self::CannotFindName),
            "2552" => Some(Self::CannotFindNameWithSuggestion),
            "2314" => Some(Self::GenericTypeRequiresArguments),
            _ => None,
        }
    }
    
    /// Get the numeric code as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PropertyDoesNotExist => "2339",
            Self::PropertyDoesNotExistWithSuggestion => "2551",
            Self::TypeNotAssignable => "2322",
            Self::ArgumentTypeNotAssignable => "2345",
            Self::CannotFindName => "2304",
            Self::CannotFindNameWithSuggestion => "2552",
            Self::GenericTypeRequiresArguments => "2314",
        }
    }
}

impl fmt::Display for TypeScriptErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TS{}", self.as_str())
    }
}

/// Rust compiler error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RustErrorCode {
    // Borrow checker errors
    /// Cannot borrow as mutable more than once
    CannotBorrowAsMutableMoreThanOnce = 499,
    /// Cannot borrow as mutable because it is also borrowed as immutable
    CannotBorrowAsMutableAlsoBorrowedAsImmutable = 502,
    /// Cannot borrow as mutable because it is also borrowed as immutable (in closure)
    CannotBorrowInClosure = 503,
    /// Cannot move out of borrowed content
    CannotMoveOutOfBorrowedContent = 504,
    /// Cannot move out of value which is behind a reference
    CannotMoveOutOfReference = 505,
    
    // Lifetime errors
    /// Missing lifetime specifier
    MissingLifetimeSpecifier = 106,
    /// Lifetime mismatch
    LifetimeMismatch = 621,
    /// Lifetime mismatch in function signature
    LifetimeMismatchInSignature = 623,
    /// Higher-ranked lifetime error
    HigherRankedLifetimeError = 495,
    
    // Move errors
    /// Use of moved value
    UseOfMovedValue = 382,
    /// Cannot move out of type, a non-copy type
    CannotMoveOutOfType = 507,
    /// Cannot move out of captured variable
    CannotMoveOutOfCapturedVariable = 508,
    /// Cannot move out of type which implements Drop
    CannotMoveOutOfDrop = 509,
    
    // Type errors
    /// Mismatched types
    MismatchedTypes = 308,
    
    // Trait errors
    /// The trait bound is not satisfied
    TraitBoundNotSatisfied = 277,
}

impl RustErrorCode {
    /// Parse from a string error code (e.g., "E0308")
    pub fn from_str(code: &str) -> Option<Self> {
        // Remove "E" prefix if present
        let numeric = code.strip_prefix('E').unwrap_or(code);
        
        match numeric {
            "499" | "0499" => Some(Self::CannotBorrowAsMutableMoreThanOnce),
            "502" | "0502" => Some(Self::CannotBorrowAsMutableAlsoBorrowedAsImmutable),
            "503" | "0503" => Some(Self::CannotBorrowInClosure),
            "504" | "0504" => Some(Self::CannotMoveOutOfBorrowedContent),
            "505" | "0505" => Some(Self::CannotMoveOutOfReference),
            "106" | "0106" => Some(Self::MissingLifetimeSpecifier),
            "621" | "0621" => Some(Self::LifetimeMismatch),
            "623" | "0623" => Some(Self::LifetimeMismatchInSignature),
            "495" | "0495" => Some(Self::HigherRankedLifetimeError),
            "382" | "0382" => Some(Self::UseOfMovedValue),
            "507" | "0507" => Some(Self::CannotMoveOutOfType),
            "508" | "0508" => Some(Self::CannotMoveOutOfCapturedVariable),
            "509" | "0509" => Some(Self::CannotMoveOutOfDrop),
            "308" | "0308" => Some(Self::MismatchedTypes),
            "277" | "0277" => Some(Self::TraitBoundNotSatisfied),
            _ => None,
        }
    }
    
    /// Get the error code as a string with "E" prefix
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CannotBorrowAsMutableMoreThanOnce => "E0499",
            Self::CannotBorrowAsMutableAlsoBorrowedAsImmutable => "E0502",
            Self::CannotBorrowInClosure => "E0503",
            Self::CannotMoveOutOfBorrowedContent => "E0504",
            Self::CannotMoveOutOfReference => "E0505",
            Self::MissingLifetimeSpecifier => "E0106",
            Self::LifetimeMismatch => "E0621",
            Self::LifetimeMismatchInSignature => "E0623",
            Self::HigherRankedLifetimeError => "E0495",
            Self::UseOfMovedValue => "E0382",
            Self::CannotMoveOutOfType => "E0507",
            Self::CannotMoveOutOfCapturedVariable => "E0508",
            Self::CannotMoveOutOfDrop => "E0509",
            Self::MismatchedTypes => "E0308",
            Self::TraitBoundNotSatisfied => "E0277",
        }
    }
    
    /// Check if this is a borrow checker error
    pub fn is_borrow_error(&self) -> bool {
        matches!(
            self,
            Self::CannotBorrowAsMutableMoreThanOnce
                | Self::CannotBorrowAsMutableAlsoBorrowedAsImmutable
                | Self::CannotBorrowInClosure
                | Self::CannotMoveOutOfBorrowedContent
                | Self::CannotMoveOutOfReference
        )
    }
    
    /// Check if this is a lifetime error
    pub fn is_lifetime_error(&self) -> bool {
        matches!(
            self,
            Self::MissingLifetimeSpecifier
                | Self::LifetimeMismatch
                | Self::LifetimeMismatchInSignature
                | Self::HigherRankedLifetimeError
        )
    }
    
    /// Check if this is a move error
    pub fn is_move_error(&self) -> bool {
        matches!(
            self,
            Self::UseOfMovedValue
                | Self::CannotMoveOutOfType
                | Self::CannotMoveOutOfCapturedVariable
                | Self::CannotMoveOutOfDrop
        )
    }
}

impl fmt::Display for RustErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Python error codes (from various linters/type checkers)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PythonErrorCode {
    // Type errors (mypy)
    /// Incompatible types
    IncompatibleTypes,
    /// Missing type annotation
    MissingTypeAnnotation,
    /// Invalid type
    InvalidType,
    
    // Import errors
    /// Module not found
    ModuleNotFound,
    /// Cannot import name
    CannotImportName,
    
    // Syntax errors
    /// Invalid syntax
    InvalidSyntax,
    /// Indentation error
    IndentationError,
}

/// A unified error code enum that can represent codes from any language
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    TypeScript(TypeScriptErrorCode),
    Rust(RustErrorCode),
    Python(PythonErrorCode),
    /// Unknown or custom error code
    Custom(String),
}

impl ErrorCode {
    /// Parse an error code from a string and source
    pub fn parse(code: &str, source: &str) -> Self {
        match source {
            "typescript" | "ts" | "tsserver" => {
                TypeScriptErrorCode::from_str(code)
                    .map(ErrorCode::TypeScript)
                    .unwrap_or_else(|| ErrorCode::Custom(code.to_string()))
            }
            "rust" | "rust-analyzer" | "rustc" => {
                RustErrorCode::from_str(code)
                    .map(ErrorCode::Rust)
                    .unwrap_or_else(|| ErrorCode::Custom(code.to_string()))
            }
            "python" | "mypy" | "pylint" | "pyright" => {
                // Python doesn't have standardized numeric codes
                ErrorCode::Custom(code.to_string())
            }
            _ => ErrorCode::Custom(code.to_string()),
        }
    }
    
    /// Get the string representation of the error code
    pub fn as_str(&self) -> &str {
        match self {
            ErrorCode::TypeScript(ts) => ts.as_str(),
            ErrorCode::Rust(rust) => rust.as_str(),
            ErrorCode::Python(_) => "Python",
            ErrorCode::Custom(s) => s,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::TypeScript(ts) => write!(f, "{ts}"),
            ErrorCode::Rust(rust) => write!(f, "{rust}"),
            ErrorCode::Python(py) => write!(f, "{py:?}"),
            ErrorCode::Custom(s) => write!(f, "{s}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_typescript_error_codes() {
        assert_eq!(
            TypeScriptErrorCode::from_str("2339"),
            Some(TypeScriptErrorCode::PropertyDoesNotExist)
        );
        assert_eq!(TypeScriptErrorCode::PropertyDoesNotExist.as_str(), "2339");
        assert_eq!(
            format!("{}", TypeScriptErrorCode::PropertyDoesNotExist),
            "TS2339"
        );
    }
    
    #[test]
    fn test_rust_error_codes() {
        assert_eq!(
            RustErrorCode::from_str("E0308"),
            Some(RustErrorCode::MismatchedTypes)
        );
        assert_eq!(
            RustErrorCode::from_str("308"),
            Some(RustErrorCode::MismatchedTypes)
        );
        assert_eq!(RustErrorCode::MismatchedTypes.as_str(), "E0308");
        assert!(RustErrorCode::UseOfMovedValue.is_move_error());
        assert!(RustErrorCode::MissingLifetimeSpecifier.is_lifetime_error());
    }
    
    #[test]
    fn test_error_code_parsing() {
        let ts_code = ErrorCode::parse("2339", "typescript");
        assert!(matches!(ts_code, ErrorCode::TypeScript(_)));
        
        let rust_code = ErrorCode::parse("E0308", "rust-analyzer");
        assert!(matches!(rust_code, ErrorCode::Rust(_)));
        
        let unknown = ErrorCode::parse("ABC123", "unknown");
        assert!(matches!(unknown, ErrorCode::Custom(_)));
    }
}