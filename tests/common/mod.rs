use lsp_bridge::core::types::{Diagnostic, DiagnosticSeverity, Position, Range};
use std::path::PathBuf;
use tempfile::{NamedTempFile, TempDir};
use tokio::fs;
use anyhow::Result;

/// Shared test utilities to reduce duplication across test files
pub mod test_helpers {
    use super::*;

    /// Create a simple diagnostic for testing
    pub fn create_test_diagnostic(
        message: &str,
        severity: DiagnosticSeverity,
        line: u32,
        character: u32,
    ) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position { line, character },
                end: Position { line, character: character + 10 },
            },
            severity: Some(severity),
            code: None,
            code_description: None,
            source: Some("test".to_string()),
            message: message.to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    /// Create multiple test diagnostics
    pub fn create_test_diagnostics(count: usize) -> Vec<Diagnostic> {
        (0..count)
            .map(|i| {
                create_test_diagnostic(
                    &format!("Test error {}", i),
                    DiagnosticSeverity::Error,
                    i as u32,
                    0,
                )
            })
            .collect()
    }

    /// Create test file path
    pub fn test_file_path(name: &str) -> PathBuf {
        PathBuf::from(format!("test_files/{}", name))
    }

    /// Common async test setup that creates a temporary directory
    pub async fn setup_test_dir() -> Result<TempDir> {
        Ok(TempDir::new()?)
    }

    /// Create a temporary file with specific content
    pub async fn create_test_file(content: &str) -> Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;
        fs::write(file.path(), content).await?;
        Ok(file)
    }

    /// Create a test file in a specific directory
    pub async fn create_test_file_in_dir(dir: &TempDir, name: &str, content: &str) -> Result<PathBuf> {
        let file_path = dir.path().join(name);
        fs::write(&file_path, content).await?;
        Ok(file_path)
    }

    /// Common test data for TypeScript files
    pub fn typescript_test_content() -> &'static str {
        r#"
interface User {
    name: string;
    age: number;
}

function greetUser(user: User) {
    console.log(`Hello, ${user.name}!`);
}

// This line has an error
const user: User = { name: "Alice" }; // Missing age property
greetUser(user);
"#
    }

    /// Common test data for Rust files
    pub fn rust_test_content() -> &'static str {
        r#"
struct User {
    name: String,
    age: u32,
}

fn greet_user(user: &User) {
    println!("Hello, {}!", user.name);
}

fn main() {
    let user = User {
        name: "Alice".to_string(),
        // Missing age field
    };
    greet_user(&user);
}
"#
    }

    /// Common test data for Python files
    pub fn python_test_content() -> &'static str {
        r#"
class User:
    def __init__(self, name: str, age: int):
        self.name = name
        self.age = age

def greet_user(user: User):
    print(f"Hello, {user.name}!")

# This line has an error
user = User("Alice")  # Missing age argument
greet_user(user)
"#
    }

    /// Create a test configuration with defaults
    pub fn create_test_config<T: Default>() -> T {
        T::default()
    }

    /// Macro for common test assertion patterns
    #[macro_export]
    macro_rules! assert_diagnostic_properties {
        ($diagnostic:expr, severity = $severity:expr, message_contains = $substr:expr) => {
            assert_eq!($diagnostic.severity, Some($severity));
            assert!($diagnostic.message.contains($substr));
        };
        ($diagnostic:expr, line = $line:expr, character = $character:expr) => {
            assert_eq!($diagnostic.range.start.line, $line);
            assert_eq!($diagnostic.range.start.character, $character);
        };
    }
}