use lsp_bridge::ai_training::*;
use lsp_bridge::core::semantic_context::SemanticContext;
use lsp_bridge::core::types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_huggingface_export() {
    let dataset = create_test_dataset();
    let exporter = TrainingExporter::new(ExportFormat::HuggingFace);

    let temp_file = NamedTempFile::new().unwrap();
    let result = exporter.export_dataset(&dataset, temp_file.path()).await;

    assert!(result.is_ok());

    let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
    assert!(content.contains("Code with errors"));
    assert!(content.contains("Fixed code"));
    assert!(content.contains("Diagnostics"));
}

#[tokio::test]
async fn test_custom_export() {
    let dataset = create_test_dataset();
    let exporter = TrainingExporter::new(ExportFormat::Custom);

    let temp_file = NamedTempFile::new().unwrap();
    let result = exporter.export_dataset(&dataset, temp_file.path()).await;

    assert!(result.is_ok());

    let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
    let parsed: TrainingDataset = serde_json::from_str(&content).unwrap();

    assert_eq!(parsed.pairs.len(), dataset.pairs.len());
    assert_eq!(parsed.name, dataset.name);
}

#[tokio::test]
async fn test_export_with_context_limits() {
    let mut dataset = create_test_dataset();

    // Add large context to test token limiting
    let mut context = SemanticContext::default();
    context.surrounding_code.insert(
        "large_file.ts".to_string(),
        "x".repeat(5000), // Very large context
    );

    dataset.pairs[0].context = context;

    let exporter = TrainingExporter::new(ExportFormat::JsonLines).with_max_tokens(100);

    let temp_file = NamedTempFile::new().unwrap();
    let result = exporter.export_dataset(&dataset, temp_file.path()).await;

    assert!(result.is_ok());

    let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
    // Should be truncated
    assert!(content.contains("..."));
}

#[tokio::test]
async fn test_export_without_metadata() {
    let dataset = create_test_dataset();
    let exporter = TrainingExporter::new(ExportFormat::JsonLines).with_metadata(false);

    let temp_file = NamedTempFile::new().unwrap();
    let result = exporter.export_dataset(&dataset, temp_file.path()).await;

    assert!(result.is_ok());

    let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
    let lines: Vec<&str> = content.lines().collect();

    for line in lines {
        if !line.is_empty() {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(parsed["metadata"], serde_json::json!({}));
        }
    }
}

#[tokio::test]
async fn test_multiple_export_formats() {
    let dataset = create_test_dataset();
    let formats = vec![
        ExportFormat::JsonLines,
        ExportFormat::OpenAI,
        ExportFormat::HuggingFace,
        ExportFormat::Custom,
    ];

    for format in formats {
        let exporter = TrainingExporter::new(format);
        let temp_file = NamedTempFile::new().unwrap();
        let result = exporter.export_dataset(&dataset, temp_file.path()).await;

        assert!(result.is_ok(), "Failed to export format: {:?}", format);

        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        assert!(!content.is_empty(), "Empty export for format: {:?}", format);
    }
}

// Helper function
fn create_test_dataset() -> TrainingDataset {
    let mut dataset = TrainingDataset::new(
        "Test Export Dataset".to_string(),
        "Dataset for testing export functionality".to_string(),
    );

    let diagnostic = Diagnostic::new(
        "test.ts".to_string(),
        Range {
            start: Position {
                line: 1,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 10,
            },
        },
        DiagnosticSeverity::Error,
        "Test error".to_string(),
        "typescript".to_string(),
    );

    let pair = TrainingPair::new(
        "const x = error".to_string(),
        "const x = fixed".to_string(),
        vec![diagnostic],
        SemanticContext::default(),
        "typescript".to_string(),
    )
    .with_confidence(0.9)
    .with_description("Fixed test error".to_string());

    dataset.add_pair(pair);
    dataset
}
