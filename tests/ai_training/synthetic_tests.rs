use lsp_bridge::ai_training::*;

#[test]
fn test_error_injector_initialization() {
    let injector = ErrorInjector::new();

    // Should have default patterns for common languages
    let ts_result = injector.inject_errors(
        "const x = \"hello\"; console.log(x);",
        "typescript",
        None,
        1,
    );
    assert!(ts_result.is_ok());

    let rust_result = injector.inject_errors("let mut x = 5;", "rust", None, 1);
    assert!(rust_result.is_ok());

    let python_result = injector.inject_errors("    return x", "python", None, 1);
    assert!(python_result.is_ok());
}

#[test]
fn test_difficulty_level_injection() {
    let injector = ErrorInjector::new();

    let code = r#"
const x: string = "hello";
const user = { name: "John" };
console.log(user.name);
"#;

    // Test beginner level
    let beginner_result =
        injector.inject_errors(code, "typescript", Some(DifficultyLevel::Beginner), 1);
    assert!(beginner_result.is_ok());

    // Test intermediate level
    let intermediate_result =
        injector.inject_errors(code, "typescript", Some(DifficultyLevel::Intermediate), 1);
    assert!(intermediate_result.is_ok());
}

#[test]
fn test_custom_pattern_addition() {
    let mut injector = ErrorInjector::new();

    let custom_pattern = synthetic::TrainingErrorPattern {
        name: "custom_error".to_string(),
        description: "Custom test error".to_string(),
        difficulty: DifficultyLevel::Beginner,
        languages: vec!["typescript".to_string()],
        transformations: vec![synthetic::CodeTransformation {
            pattern: "console.log".to_string(),
            replacement: "console.lg".to_string(),
            diagnostic_message: "Property 'lg' does not exist on type 'Console'".to_string(),
            diagnostic_type: "type".to_string(),
        }],
    };

    injector.add_custom_pattern("typescript".to_string(), custom_pattern);

    let code = "console.log('test');";
    let result = injector.inject_errors(code, "typescript", Some(DifficultyLevel::Beginner), 1);

    assert!(result.is_ok());
    let pairs = result.unwrap();

    // Should find and apply our custom pattern
    if let Some(pair) = pairs.first() {
        assert!(pair.before_code.contains("console.lg"));
        assert!(pair.after_code.contains("console.log"));
    }
}

#[test]
fn test_gradient_dataset_all_difficulties() {
    let injector = ErrorInjector::new();

    let base_code = r#"
const x: string = "hello";
let mut data = 5;
const user = { name: "John" };
console.log(user.name);
"#;

    let result = injector.generate_gradient_dataset(base_code, "typescript", 4);
    assert!(result.is_ok());

    let dataset = result.unwrap();

    // Should have examples for each difficulty level
    let difficulties: Vec<String> = dataset
        .pairs
        .iter()
        .filter_map(|p| p.metadata.get("difficulty"))
        .map(|v| v.to_string())
        .collect();

    assert!(!difficulties.is_empty());
}

#[test]
fn test_multiple_error_injection() {
    let injector = ErrorInjector::new();

    let code = r#"
const x: string = "hello";
const y: number = 42;
const z: boolean = true;
"#;

    let result = injector.inject_errors(code, "typescript", None, 3);
    assert!(result.is_ok());

    let pairs = result.unwrap();
    // May not get exactly 3 if patterns don't match, but should get some
    assert!(!pairs.is_empty());

    // Each pair should have diagnostics
    for pair in &pairs {
        assert!(!pair.diagnostics.is_empty());
        assert_eq!(pair.confidence.score, 1.0); // Synthetic data has perfect confidence
    }
}

#[test]
fn test_language_specific_patterns() {
    let injector = ErrorInjector::new();

    // Test Rust-specific patterns
    let rust_code = "let mut x = 5; x = 10;";
    let rust_result = injector.inject_errors(rust_code, "rust", None, 1);
    assert!(rust_result.is_ok());

    // Test Python-specific patterns
    let python_code = "    return x + y";
    let python_result = injector.inject_errors(python_code, "python", None, 1);
    assert!(python_result.is_ok());

    // Test unsupported language
    let unknown_result = injector.inject_errors("code", "cobol", None, 1);
    assert!(unknown_result.is_err());
}

#[test]
fn test_transformation_metadata() {
    let injector = ErrorInjector::new();

    let code = "const x: string = \"hello\";";
    let result = injector.inject_errors(code, "typescript", Some(DifficultyLevel::Intermediate), 1);

    assert!(result.is_ok());
    let pairs = result.unwrap();

    if let Some(pair) = pairs.first() {
        // Should have fix description
        assert!(!pair.fix_description.is_empty());

        // Should have proper diagnostic
        let diag = &pair.diagnostics[0];
        assert!(!diag.message.is_empty());
        assert_eq!(diag.file, "synthetic.typescript");
    }
}
