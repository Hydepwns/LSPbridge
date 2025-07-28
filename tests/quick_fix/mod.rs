use lsp_bridge::core::types::{Diagnostic, DiagnosticSeverity, Position, Range};
use lsp_bridge::quick_fix::*;
use std::path::PathBuf;
use tempfile::{NamedTempFile, TempDir};
use tokio::fs;

#[test]
fn test_confidence_scoring() {
    let scorer = FixConfidenceScorer::new();

    // Test TypeScript error with known pattern
    let diagnostic = Diagnostic {
        id: "1".to_string(),
        file: "test.ts".to_string(),
        range: Range {
            start: Position {
                line: 1,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 10,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "Type 'string' is not assignable to type 'number'".to_string(),
        code: Some("TS2322".to_string()),
        source: "typescript".to_string(),
        related_information: None,
        tags: None,
        data: None,
    };

    let (score, factors) = scorer.score_fix(&diagnostic, "number", false);

    // Should have decent confidence for known TypeScript error
    assert!(score.value() > 0.6);
    assert!(factors.pattern_recognition > 0.8);
    assert!(factors.language_confidence > 0.8);
}

#[test]
fn test_confidence_thresholds() {
    let threshold = ConfidenceThreshold::default();

    let high_score = ConfidenceScore::new(0.95);
    assert!(high_score.is_auto_applicable(&threshold));
    assert!(high_score.is_suggestable(&threshold));

    let medium_score = ConfidenceScore::new(0.7);
    assert!(!medium_score.is_auto_applicable(&threshold));
    assert!(medium_score.is_suggestable(&threshold));

    let low_score = ConfidenceScore::new(0.2);
    assert!(!low_score.is_auto_applicable(&threshold));
    assert!(!low_score.is_suggestable(&threshold));
}

#[tokio::test]
async fn test_fix_application() {
    let engine = FixApplicationEngine::new();

    // Create temporary file
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_path_buf();

    // Write initial content
    let initial_content = r#"let x: number = "string";
console.log(x);"#;
    fs::write(&file_path, initial_content).await.unwrap();

    // Create fix edit
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
    assert!(new_content.contains("let x: string"));
}

#[tokio::test]
async fn test_fix_with_confidence() {
    let engine = FixApplicationEngine::new();
    let threshold = ConfidenceThreshold::default();

    // Create temporary file
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_path_buf();
    fs::write(&file_path, "test content").await.unwrap();

    // High confidence fix - should be applied
    let high_conf_fix = (
        FixEdit {
            file_path: file_path.clone(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 4,
                },
            },
            new_text: "best".to_string(),
            description: Some("High confidence fix".to_string()),
        },
        ConfidenceScore::new(0.95),
    );

    // Low confidence fix - should not be applied
    let low_conf_fix = (
        FixEdit {
            file_path: file_path.clone(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 5,
                },
                end: Position {
                    line: 0,
                    character: 12,
                },
            },
            new_text: "changed".to_string(),
            description: Some("Low confidence fix".to_string()),
        },
        ConfidenceScore::new(0.3),
    );

    let fixes = vec![high_conf_fix, low_conf_fix];
    let results = engine
        .apply_fixes_with_confidence(&fixes, &threshold)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert!(results[0].0.success); // High confidence was applied
    assert!(results[0].1); // Was auto-applied
    assert!(!results[1].0.success); // Low confidence was not applied
    assert!(!results[1].1); // Was not auto-applied
}

#[tokio::test]
async fn test_rollback_manager() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = RollbackManager::new(temp_dir.path().to_path_buf());
    manager.init().await.unwrap();

    // Create backup
    let backup = engine::FileBackup {
        file_path: PathBuf::from("test.rs"),
        original_content: "original content".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let state = RollbackManager::create_state(vec![backup], "Test fixes applied".to_string());

    let session_id = state.session_id.clone();

    // Save state
    manager.save_state(state).await.unwrap();

    // Verify we can retrieve it
    let retrieved = manager.get_state(&session_id).await.unwrap();
    assert!(retrieved.is_some());
    assert!(!retrieved.unwrap().rolled_back);

    // Get latest state
    let latest = manager.get_latest_state().await.unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().session_id, session_id);
}

#[tokio::test]
async fn test_rollback_operation() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = RollbackManager::new(temp_dir.path().to_path_buf());
    manager.init().await.unwrap();

    // Create temporary file
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_path_buf();

    // Write modified content
    fs::write(&file_path, "modified content").await.unwrap();

    // Create backup with original content
    let backup = engine::FileBackup {
        file_path: file_path.clone(),
        original_content: "original content".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let state = RollbackManager::create_state(vec![backup], "Test rollback".to_string());

    let session_id = state.session_id.clone();
    manager.save_state(state).await.unwrap();

    // Perform rollback
    manager.rollback(&session_id).await.unwrap();

    // Verify file was restored
    let content = fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(content, "original content");

    // Verify state is marked as rolled back
    let state = manager.get_state(&session_id).await.unwrap().unwrap();
    assert!(state.rolled_back);
}

#[test]
fn test_build_status_parsing() {
    // Test language detection
    let files = vec![PathBuf::from("main.rs")];
    assert_eq!(verification::detect_language_from_files(&files), "rust");

    let files = vec![PathBuf::from("app.ts"), PathBuf::from("utils.ts")];
    assert_eq!(
        verification::detect_language_from_files(&files),
        "typescript"
    );
}

#[test]
fn test_fix_edit_creation() {
    let diagnostic = Diagnostic {
        id: "1".to_string(),
        file: "test.ts".to_string(),
        range: Range {
            start: Position {
                line: 5,
                character: 10,
            },
            end: Position {
                line: 5,
                character: 20,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "Missing semicolon".to_string(),
        code: None,
        source: "eslint".to_string(),
        related_information: None,
        tags: None,
        data: None,
    };

    let fix_edit = FixApplicationEngine::create_fix_from_diagnostic(&diagnostic, ";");
    assert!(fix_edit.is_some());

    let edit = fix_edit.unwrap();
    assert_eq!(edit.file_path, PathBuf::from("test.ts"));
    assert_eq!(edit.new_text, ";");
    assert_eq!(edit.range.start.line, 5);
}

// Helper to access private module items for testing
mod verification {
    pub use lsp_bridge::quick_fix::verification::detect_language_from_files;
}
