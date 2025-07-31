//! Utility modules for format conversion

pub mod range_converter;
pub mod severity_converter;

pub use range_converter::RangeConverter;
pub use severity_converter::SeverityConverter;

use uuid::Uuid;

/// Generate a unique ID for a diagnostic
pub fn generate_id(source: &str, _index: usize) -> String {
    format!("{}_{}", source, Uuid::new_v4())
}

/// Normalize file paths across different formats
pub fn normalize_file_path(file_path: &str) -> String {
    if file_path.is_empty() {
        return file_path.to_string();
    }

    let mut path = file_path.to_string();

    // Remove file:// prefix if present
    if path.starts_with("file://") {
        path = path[7..].to_string();
    }

    // Normalize path separators
    path = path.replace('\\', "/");

    path
}