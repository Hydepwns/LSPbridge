use crate::core::types::{Diagnostic, Range};
use crate::core::utils::FileUtils;
use crate::quick_fix::confidence::{ConfidenceScore, ConfidenceThreshold};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Represents a single edit to apply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixEdit {
    /// File to edit
    pub file_path: PathBuf,
    /// Range to replace
    pub range: Range,
    /// New text to insert
    pub new_text: String,
    /// Optional description of the fix
    pub description: Option<String>,
}

/// Result of applying a fix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    /// Whether the fix was successfully applied
    pub success: bool,
    /// Files that were modified
    pub modified_files: Vec<PathBuf>,
    /// Error message if failed
    pub error: Option<String>,
    /// Original content for rollback
    pub backup: Option<FileBackup>,
}

/// Backup of original file content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBackup {
    pub file_path: PathBuf,
    pub original_content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Engine for applying fixes to code
pub struct FixApplicationEngine {
    /// Whether to create backups before applying fixes
    create_backups: bool,
    /// Whether to preserve formatting
    preserve_formatting: bool,
    /// Maximum file size to process (in bytes)
    max_file_size: usize,
}

impl FixApplicationEngine {
    pub fn new() -> Self {
        Self {
            create_backups: true,
            preserve_formatting: true,
            max_file_size: 10 * 1024 * 1024, // 10MB
        }
    }

    pub fn with_backups(mut self, enabled: bool) -> Self {
        self.create_backups = enabled;
        self
    }

    pub fn with_formatting(mut self, preserve: bool) -> Self {
        self.preserve_formatting = preserve;
        self
    }

    /// Apply a single fix edit
    pub async fn apply_fix(&self, edit: &FixEdit) -> Result<FixResult> {
        // Validate file exists and is not too large
        let metadata = fs::metadata(&edit.file_path)
            .await
            .context("Failed to read file metadata")?;

        if metadata.len() > self.max_file_size as u64 {
            return Ok(FixResult {
                success: false,
                modified_files: vec![],
                error: Some(format!("File too large: {} bytes", metadata.len())),
                backup: None,
            });
        }

        // Read original content
        let original_content =
            FileUtils::read_with_context(&edit.file_path, "source file for fix").await?;

        // Create backup if enabled
        let backup = if self.create_backups {
            Some(FileBackup {
                file_path: edit.file_path.clone(),
                original_content: original_content.clone(),
                timestamp: chrono::Utc::now(),
            })
        } else {
            None
        };

        // Apply the edit
        let new_content = self.apply_edit_to_content(&original_content, edit)?;

        // Write back to file
        FileUtils::write_with_context(&edit.file_path, &new_content, "modified file").await?;

        Ok(FixResult {
            success: true,
            modified_files: vec![edit.file_path.clone()],
            error: None,
            backup,
        })
    }

    /// Apply multiple fixes, stopping on first error
    pub async fn apply_fixes(&self, edits: &[FixEdit]) -> Result<Vec<FixResult>> {
        let mut results = Vec::new();

        for edit in edits {
            let result = self.apply_fix(edit).await?;
            if !result.success {
                // Stop on first failure
                results.push(result);
                break;
            }
            results.push(result);
        }

        Ok(results)
    }

    /// Apply fixes with confidence threshold checking
    pub async fn apply_fixes_with_confidence(
        &self,
        fixes: &[(FixEdit, ConfidenceScore)],
        threshold: &ConfidenceThreshold,
    ) -> Result<Vec<(FixResult, bool)>> {
        let mut results = Vec::new();

        for (edit, confidence) in fixes {
            if confidence.is_auto_applicable(threshold) {
                let result = self.apply_fix(edit).await?;
                results.push((result, true)); // Was auto-applied
            } else if confidence.is_suggestable(threshold) {
                // Would normally prompt user here
                results.push((
                    FixResult {
                        success: false,
                        modified_files: vec![],
                        error: Some("Fix requires manual confirmation".to_string()),
                        backup: None,
                    },
                    false, // Was not auto-applied
                ));
            } else {
                // Confidence too low
                results.push((
                    FixResult {
                        success: false,
                        modified_files: vec![],
                        error: Some(format!("Confidence too low: {:.2}", confidence.value())),
                        backup: None,
                    },
                    false,
                ));
            }
        }

        Ok(results)
    }

    /// Apply edit to file content
    fn apply_edit_to_content(&self, content: &str, edit: &FixEdit) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();

        // Validate range
        if edit.range.start.line as usize > lines.len()
            || edit.range.end.line as usize > lines.len()
        {
            return Err(anyhow!("Edit range out of bounds"));
        }

        let mut result = String::new();

        // Add lines before the edit
        for (i, line) in lines.iter().enumerate() {
            let line_num = i as u32;

            if line_num < edit.range.start.line {
                result.push_str(line);
                result.push('\n');
            } else if line_num == edit.range.start.line {
                // Handle partial line replacement
                if edit.range.start.character > 0 {
                    let start_char = edit.range.start.character as usize;
                    if start_char <= line.len() {
                        result.push_str(&line[..start_char]);
                    }
                }

                // Insert new text
                result.push_str(&edit.new_text);

                // Handle end of range
                if edit.range.end.line == edit.range.start.line {
                    // Same line - add rest of line after edit
                    let end_char = edit.range.end.character as usize;
                    if end_char < line.len() {
                        result.push_str(&line[end_char..]);
                    }
                    result.push('\n');
                }
            } else if line_num > edit.range.start.line && line_num < edit.range.end.line {
                // Skip lines in the middle of the range
                continue;
            } else if line_num == edit.range.end.line && edit.range.end.line > edit.range.start.line
            {
                // Handle end of multi-line edit
                let end_char = edit.range.end.character as usize;
                if end_char < line.len() {
                    result.push_str(&line[end_char..]);
                }
                result.push('\n');
            } else {
                // Lines after the edit
                result.push_str(line);
                result.push('\n');
            }
        }

        // Remove trailing newline if original didn't have one
        if !content.ends_with('\n') && result.ends_with('\n') {
            result.pop();
        }

        Ok(result)
    }

    /// Create a fix edit from a diagnostic with suggested fix
    pub fn create_fix_from_diagnostic(
        diagnostic: &Diagnostic,
        suggested_fix: &str,
    ) -> Option<FixEdit> {
        Some(FixEdit {
            file_path: PathBuf::from(&diagnostic.file),
            range: diagnostic.range.clone(),
            new_text: suggested_fix.to_string(),
            description: Some(format!("Fix: {}", diagnostic.message)),
        })
    }
}

impl Default for FixApplicationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension methods for creating fixes from LSP code actions
impl FixEdit {
    pub fn from_lsp_text_edit(file_path: PathBuf, range: Range, new_text: String) -> Self {
        Self {
            file_path,
            range,
            new_text,
            description: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio;
    use crate::core::types::{Position, Range};

    #[tokio::test]
    async fn test_apply_simple_fix() {
        let engine = FixApplicationEngine::new();

        // Create a temporary file with test content
        let initial_content = "let x: number = \"string\";\n";
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        fs::write(&file_path, initial_content).await.unwrap();

        // Create fix
        let edit = FixEdit {
            file_path: file_path.clone(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 7,
                },
                end: Position {
                    line: 0,
                    character: 13,
                },
            },
            new_text: "string".to_string(),
            description: Some("Fix type annotation".to_string()),
        };

        // Apply fix
        let result = engine.apply_fix(&edit).await.unwrap();

        assert!(result.success);
        assert_eq!(result.modified_files.len(), 1);
        assert!(result.backup.is_some());

        // Verify content
        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "let x: string = \"string\";\n");
    }

    #[tokio::test]
    async fn test_multi_line_fix() {
        let engine = FixApplicationEngine::new();

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();

        let initial_content = "function test() {\n    console.log(x)\n}\n";
        fs::write(&file_path, initial_content).await.unwrap();

        // Fix: add semicolon
        let edit = FixEdit {
            file_path: file_path.clone(),
            range: Range {
                start: Position {
                    line: 1,
                    character: 18,
                },
                end: Position {
                    line: 1,
                    character: 18,
                },
            },
            new_text: ";".to_string(),
            description: Some("Add missing semicolon".to_string()),
        };

        let result = engine.apply_fix(&edit).await.unwrap();
        assert!(result.success);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "function test() {\n    console.log(x);\n}\n");
    }
}
