mod export_tests;
mod synthetic_tests;

use lsp_bridge::ai_training::data_structures::ConfidenceCategory;
use lsp_bridge::ai_training::*;
use lsp_bridge::core::semantic_context::SemanticContext;
use lsp_bridge::core::types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_training_pair_creation() {
    let diagnostic = create_test_diagnostic();
    let context = SemanticContext::default();

    let pair = TrainingPair::new(
        "const x: number = \"string\"".to_string(),
        "const x: string = \"string\"".to_string(),
        vec![diagnostic],
        context,
        "typescript".to_string(),
    );

    assert!(!pair.id.is_empty());
    assert_eq!(pair.language, "typescript");
    assert_eq!(pair.diagnostics.len(), 1);
    assert_eq!(pair.confidence.score, 0.5);
}

#[tokio::test]
async fn test_training_dataset() {
    let mut dataset =
        TrainingDataset::new("Test Dataset".to_string(), "Test description".to_string());

    assert_eq!(dataset.pairs.len(), 0);

    let pair = create_test_training_pair();
    dataset.add_pair(pair);

    assert_eq!(dataset.pairs.len(), 1);
    assert_eq!(dataset.statistics.total_pairs, 1);
    assert!(dataset.statistics.languages.contains_key("typescript"));
}

#[tokio::test]
async fn test_jsonl_export() {
    let dataset = create_test_dataset();
    let exporter = TrainingExporter::new(ExportFormat::JsonLines);

    let temp_file = NamedTempFile::new().unwrap();
    let result = exporter.export_dataset(&dataset, temp_file.path()).await;

    assert!(result.is_ok());

    let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
    assert!(!content.is_empty());
    assert!(content.contains("Type 'string' is not assignable to type 'number'"));
    assert!(content.contains("typescript"));
}

#[tokio::test]
async fn test_openai_export() {
    let dataset = create_test_dataset();
    let exporter = TrainingExporter::new(ExportFormat::OpenAI);

    let temp_file = NamedTempFile::new().unwrap();
    let result = exporter.export_dataset(&dataset, temp_file.path()).await;

    assert!(result.is_ok());

    let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
    assert!(content.contains("system"));
    assert!(content.contains("user"));
    assert!(content.contains("assistant"));
}

#[tokio::test]
async fn test_error_injection() {
    let injector = ErrorInjector::new();

    let clean_code = r#"
const x: string = "hello";
const y: number = 42;
"#;

    let result = injector.inject_errors(clean_code, "typescript", None, 1);
    assert!(result.is_ok());

    let pairs = result.unwrap();
    assert!(!pairs.is_empty());

    let pair = &pairs[0];
    assert_ne!(pair.before_code, pair.after_code);
    assert!(!pair.diagnostics.is_empty());
}

#[tokio::test]
async fn test_gradient_dataset_generation() {
    let injector = ErrorInjector::new();

    let base_code = r#"
const user = { name: "John" };
const x: string = "hello";
"#;

    let result = injector.generate_gradient_dataset(base_code, "typescript", 1);
    assert!(result.is_ok());

    let dataset = result.unwrap();
    assert!(dataset.pairs.len() >= 1);

    // Check metadata includes difficulty
    for pair in &dataset.pairs {
        assert!(pair.metadata.contains_key("difficulty"));
    }
}

#[tokio::test]
async fn test_annotation_tool() {
    let mut tool = AnnotationTool::new();
    let session_id = tool.start_session("test_user".to_string(), "dataset_1".to_string());

    assert!(!session_id.is_empty());

    let mut pair = create_test_training_pair();

    let verification = annotation::VerificationResult {
        compiles: true,
        tests_pass: Some(true),
        linter_warnings: vec![],
        performance_impact: None,
        side_effects: vec![],
    };

    let result = tool.annotate_pair(
        &mut pair,
        FixQuality::Good,
        "Looks good".to_string(),
        vec!["reviewed".to_string()],
        verification,
    );

    assert!(result.is_ok());
    let annotation = result.unwrap();
    assert_eq!(annotation.quality, FixQuality::Good);
    assert!(annotation.confidence_adjustment > 0.0);
}

#[tokio::test]
async fn test_annotation_report() {
    let mut dataset = create_test_dataset();

    // Add some metadata to simulate annotations
    for pair in &mut dataset.pairs {
        pair.add_metadata(
            "fix_quality".to_string(),
            serde_json::json!(FixQuality::Good),
        );
    }

    let tool = AnnotationTool::new();
    let report = tool.get_annotation_report(&dataset);

    assert!(report.is_ok());
    let report = report.unwrap();
    assert_eq!(report.total_annotated, 1);
}

#[tokio::test]
async fn test_confidence_scoring() {
    let confidence = FixConfidence::new(0.95);
    assert_eq!(confidence.category, ConfidenceCategory::Certain);
    assert!(confidence.is_auto_applicable());

    let confidence = FixConfidence::new(0.4);
    assert_eq!(confidence.category, ConfidenceCategory::Uncertain);
    assert!(!confidence.is_auto_applicable());
}

#[tokio::test]
async fn test_dataset_filtering() {
    let mut dataset = create_test_dataset();

    // Add more pairs with different languages and confidence
    let pair2 = TrainingPair::new(
        "def hello(): pass".to_string(),
        "def hello():\n    pass".to_string(),
        vec![],
        SemanticContext::default(),
        "python".to_string(),
    )
    .with_confidence(0.8);

    dataset.add_pair(pair2);

    // Test language filtering
    let ts_pairs = dataset.filter_by_language("typescript");
    assert_eq!(ts_pairs.len(), 1);

    let py_pairs = dataset.filter_by_language("python");
    assert_eq!(py_pairs.len(), 1);

    // Test confidence filtering
    let high_conf_pairs = dataset.filter_by_confidence(0.7);
    assert_eq!(high_conf_pairs.len(), 2); // Both pairs have confidence > 0.7
}

// Helper functions
fn create_test_diagnostic() -> Diagnostic {
    Diagnostic::new(
        "test.ts".to_string(),
        Range {
            start: Position {
                line: 10,
                character: 5,
            },
            end: Position {
                line: 10,
                character: 15,
            },
        },
        DiagnosticSeverity::Error,
        "Type 'string' is not assignable to type 'number'".to_string(),
        "typescript".to_string(),
    )
}

fn create_test_training_pair() -> TrainingPair {
    TrainingPair::new(
        "const x: number = \"string\"".to_string(),
        "const x: string = \"string\"".to_string(),
        vec![create_test_diagnostic()],
        SemanticContext::default(),
        "typescript".to_string(),
    )
    .with_confidence(0.95)
}

fn create_test_dataset() -> TrainingDataset {
    let mut dataset = TrainingDataset::new(
        "Test Dataset".to_string(),
        "Test dataset for unit tests".to_string(),
    );

    dataset.add_pair(create_test_training_pair());
    dataset
}
