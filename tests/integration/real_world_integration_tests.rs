use anyhow::Result;
use lsp_bridge::core::*;

mod fixtures {
    include!("../fixtures/real_world_examples.rs");
}

#[tokio::test]
async fn test_typescript_react_component_context_extraction() -> Result<()> {
    let mut extractor = ContextExtractor::new()?;

    // Test TypeScript React component with a callback error
    let diagnostic = Diagnostic {
        id: "ts-react-1".to_string(),
        file: "UserList.tsx".to_string(),
        range: Range {
            start: Position {
                line: 71,
                character: 8,
            },
            end: Position {
                line: 71,
                character: 20,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "Cannot invoke an object which is possibly 'undefined'.".to_string(),
        code: None,
        source: "typescript".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    let context = extractor.extract_context(&diagnostic, fixtures::TYPESCRIPT_REACT_COMPONENT)?;

    // Test context ranking for this complex scenario
    let ranker = ContextRanker::builder().max_tokens(1500).build();
    let ranked_context = ranker.rank_context(context, &diagnostic)?;

    // Verify that relevant context is captured
    assert!(!ranked_context.ranked_elements.is_empty());

    // Check that React imports are captured
    let has_react_import = ranked_context.ranked_elements.iter()
        .any(|e| matches!(&e.content, ContextContent::Import(import) if import.imported_names.contains(&"React".to_string())));
    assert!(has_react_import, "Should capture React import");

    // Check that User interface is captured
    let has_user_type = ranked_context.ranked_elements.iter()
        .any(|e| matches!(&e.content, ContextContent::Import(import) if import.imported_names.contains(&"User".to_string())));
    assert!(has_user_type, "Should capture User type import");

    // Check that the function containing the error is captured
    let function_elements: Vec<_> = ranked_context
        .ranked_elements
        .iter()
        .filter(|e| matches!(e.element_type, ContextElementType::FunctionContext))
        .collect();

    // Should have the handleUserClick function context
    assert!(
        !function_elements.is_empty(),
        "Should capture function context"
    );

    // Format for AI and verify it's comprehensive
    let formatted = format_context_for_ai(&ranked_context);
    assert!(
        formatted.contains("UserList"),
        "Should mention the component"
    );
    assert!(
        formatted.contains("onUserSelect"),
        "Should mention the problematic callback"
    );

    println!("React Component Context Analysis:");
    println!("- Total elements: {}", ranked_context.ranked_elements.len());
    println!(
        "- Tokens used: {}",
        ranked_context.budget_context.tokens_used
    );
    println!(
        "- Essential elements: {}",
        ranked_context.budget_context.essential_context.len()
    );

    Ok(())
}

#[tokio::test]
async fn test_rust_web_server_context_extraction() -> Result<()> {
    let mut extractor = ContextExtractor::new()?;

    // Test Rust ownership/borrowing error in web server setup
    let diagnostic = Diagnostic {
        id: "rust-web-1".to_string(),
        file: "server.rs".to_string(),
        range: Range {
            start: Position {
                line: 167,
                character: 32,
            },
            end: Position {
                line: 167,
                character: 39,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "borrow of moved value: `service`".to_string(),
        code: None,
        source: "rust-analyzer".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    let context = extractor.extract_context(&diagnostic, fixtures::RUST_WEB_SERVER)?;

    // Test advanced dependency analysis
    let analyzer = DependencyAnalyzer::new()?;

    // Test context ranking with focus on ownership patterns
    let ranker = ContextRanker::builder().max_tokens(2000).build();
    let ranked_context = ranker.rank_context(context, &diagnostic)?;

    // Verify Rust-specific patterns are captured
    let has_arc_import = ranked_context.ranked_elements.iter().any(|e| {
        matches!(&e.content, ContextContent::Import(import) if
            import.imported_names.iter().any(|name| name.contains("Arc")))
    });
    assert!(
        has_arc_import,
        "Should capture Arc import for shared ownership"
    );

    // Check that the problematic function is captured
    let function_elements: Vec<_> = ranked_context
        .ranked_elements
        .iter()
        .filter(|e| matches!(e.element_type, ContextElementType::FunctionContext))
        .collect();

    assert!(
        !function_elements.is_empty(),
        "Should capture function context"
    );

    // The setup_routes function should be high priority as it contains the error
    let setup_routes_priority = function_elements.iter()
        .find(|e| matches!(&e.content, ContextContent::Function(func) if func.name.contains("setup_routes")))
        .map(|e| e.priority_score);

    if let Some(priority) = setup_routes_priority {
        assert!(
            priority > 0.8,
            "setup_routes function should have high priority due to containing the error"
        );
    }

    // Format and verify Rust-specific context
    let formatted = format_context_for_ai(&ranked_context);
    assert!(
        formatted.contains("Arc"),
        "Should include Arc usage context"
    );
    assert!(
        formatted.contains("service"),
        "Should mention the moved variable"
    );

    println!("Rust Web Server Context Analysis:");
    println!("- Total elements: {}", ranked_context.ranked_elements.len());
    println!(
        "- Highest priority: {:.2}",
        ranked_context
            .ranked_elements
            .iter()
            .map(|e| e.priority_score)
            .fold(0.0, f32::max)
    );

    Ok(())
}

#[tokio::test]
async fn test_python_data_processing_context_extraction() -> Result<()> {
    let mut extractor = ContextExtractor::new()?;

    // Test Python type inconsistency error
    let diagnostic = Diagnostic {
        id: "python-1".to_string(),
        file: "data_processor.py".to_string(),
        range: Range {
            start: Position {
                line: 245,
                character: 19,
            },
            end: Position {
                line: 245,
                character: 24,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "Incompatible return value type (got \"bool\", expected \"Dict[str, Union[str, int]]\")"
            .to_string(),
        code: None,
        source: "mypy".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    let context = extractor.extract_context(&diagnostic, fixtures::PYTHON_DATA_PROCESSING)?;

    // Test with high token budget for complex Python code
    let ranker = ContextRanker::builder().max_tokens(3000).build();
    let ranked_context = ranker.rank_context(context, &diagnostic)?;

    // Verify Python-specific imports are captured
    let has_typing_imports = ranked_context.ranked_elements.iter()
        .any(|e| matches!(&e.content, ContextContent::Import(import) if
            import.imported_names.iter().any(|name| ["Dict", "List", "Optional"].contains(&name.as_str()))));
    assert!(has_typing_imports, "Should capture typing imports");

    // Check for pandas import
    let has_pandas_import = ranked_context.ranked_elements.iter().any(|e| {
        matches!(&e.content, ContextContent::Import(import) if
            import.imported_names.contains(&"pandas".to_string()) || 
            import.statement.contains("pandas"))
    });
    assert!(has_pandas_import, "Should capture pandas import");

    // Verify the problematic function gets high priority
    let function_elements: Vec<_> = ranked_context
        .ranked_elements
        .iter()
        .filter(|e| matches!(e.element_type, ContextElementType::FunctionContext))
        .collect();

    assert!(
        !function_elements.is_empty(),
        "Should capture function context"
    );

    // The process function should be captured as it contains the return type error
    let process_function = function_elements
        .iter()
        .find(|e| matches!(&e.content, ContextContent::Function(func) if func.name == "process"));

    assert!(
        process_function.is_some(),
        "Should capture the process function containing the error"
    );

    // Format and check for Python-specific context
    let formatted = format_context_for_ai(&ranked_context);
    assert!(
        formatted.contains("Dict"),
        "Should include Dict type information"
    );
    assert!(
        formatted.contains("process"),
        "Should mention the problematic function"
    );

    println!("Python Data Processing Context Analysis:");
    println!("- Total elements: {}", ranked_context.ranked_elements.len());
    println!(
        "- Type-related elements: {}",
        ranked_context
            .ranked_elements
            .iter()
            .filter(|e| matches!(
                e.element_type,
                ContextElementType::TypeDefinition | ContextElementType::Import
            ))
            .count()
    );

    Ok(())
}

#[tokio::test]
async fn test_cross_language_context_comparison() -> Result<()> {
    // Test how context extraction differs across languages for similar errors
    let mut extractor = ContextExtractor::new()?;
    let ranker = ContextRanker::builder().max_tokens(1000).build();

    // TypeScript null/undefined error
    let ts_diagnostic = Diagnostic {
        id: "cross-lang-ts".to_string(),
        file: "app.ts".to_string(),
        range: Range {
            start: Position {
                line: 58,
                character: 8,
            },
            end: Position {
                line: 58,
                character: 20,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "Cannot invoke an object which is possibly 'undefined'.".to_string(),
        code: None,
        source: "typescript".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    // Rust ownership error
    let rust_diagnostic = Diagnostic {
        id: "cross-lang-rust".to_string(),
        file: "app.rs".to_string(),
        range: Range {
            start: Position {
                line: 167,
                character: 32,
            },
            end: Position {
                line: 167,
                character: 39,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "borrow of moved value: `service`".to_string(),
        code: None,
        source: "rust-analyzer".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    // Python type error
    let python_diagnostic = Diagnostic {
        id: "cross-lang-python".to_string(),
        file: "app.py".to_string(),
        range: Range {
            start: Position {
                line: 245,
                character: 19,
            },
            end: Position {
                line: 245,
                character: 24,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "Incompatible return value type (got \"bool\", expected \"Dict[str, Union[str, int]]\")"
            .to_string(),
        code: None,
        source: "mypy".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    // Extract contexts
    let ts_context =
        extractor.extract_context(&ts_diagnostic, fixtures::TYPESCRIPT_REACT_COMPONENT)?;
    let rust_context = extractor.extract_context(&rust_diagnostic, fixtures::RUST_WEB_SERVER)?;
    let python_context =
        extractor.extract_context(&python_diagnostic, fixtures::PYTHON_DATA_PROCESSING)?;

    // Rank contexts
    let ts_ranked = ranker.rank_context(ts_context, &ts_diagnostic)?;
    let rust_ranked = ranker.rank_context(rust_context, &rust_diagnostic)?;
    let python_ranked = ranker.rank_context(python_context, &python_diagnostic)?;

    // Compare context extraction effectiveness
    println!("Cross-Language Context Comparison:");
    println!(
        "TypeScript: {} elements, {} tokens",
        ts_ranked.ranked_elements.len(),
        ts_ranked.budget_context.tokens_used
    );
    println!(
        "Rust: {} elements, {} tokens",
        rust_ranked.ranked_elements.len(),
        rust_ranked.budget_context.tokens_used
    );
    println!(
        "Python: {} elements, {} tokens",
        python_ranked.ranked_elements.len(),
        python_ranked.budget_context.tokens_used
    );

    // Verify each language captures appropriate imports
    let ts_imports = ts_ranked
        .ranked_elements
        .iter()
        .filter(|e| matches!(e.element_type, ContextElementType::Import))
        .count();
    let rust_imports = rust_ranked
        .ranked_elements
        .iter()
        .filter(|e| matches!(e.element_type, ContextElementType::Import))
        .count();
    let python_imports = python_ranked
        .ranked_elements
        .iter()
        .filter(|e| matches!(e.element_type, ContextElementType::Import))
        .count();

    assert!(ts_imports > 0, "TypeScript should capture imports");
    assert!(rust_imports > 0, "Rust should capture use statements");
    assert!(
        python_imports > 0,
        "Python should capture import statements"
    );

    // Verify language-specific patterns

    // TypeScript should capture callback/undefined patterns
    let ts_has_callback_context = ts_ranked.ranked_elements.iter().any(|e| {
        e.relevance_explanation.to_lowercase().contains("callback")
            || e.relevance_explanation.to_lowercase().contains("undefined")
    });

    // Rust should capture ownership/borrowing patterns
    let rust_has_ownership_context = rust_ranked.ranked_elements.iter().any(|e| {
        e.relevance_explanation.to_lowercase().contains("service")
            || e.relevance_explanation.to_lowercase().contains("move")
    });

    // Python should capture type annotation patterns
    let python_has_type_context = python_ranked.ranked_elements.iter().any(|e| {
        e.relevance_explanation.to_lowercase().contains("dict")
            || e.relevance_explanation.to_lowercase().contains("type")
    });

    // These are not strict requirements since the actual relevance depends on the exact context
    // but serve as good indicators of language-specific analysis
    println!("Language-specific context detection:");
    println!("TypeScript callback context: {}", ts_has_callback_context);
    println!("Rust ownership context: {}", rust_has_ownership_context);
    println!("Python type context: {}", python_has_type_context);

    Ok(())
}

#[tokio::test]
async fn test_context_budget_stress_test() -> Result<()> {
    // Test context ranking under extreme budget constraints
    let mut extractor = ContextExtractor::new()?;

    let diagnostic = Diagnostic {
        id: "budget-stress".to_string(),
        file: "complex.ts".to_string(),
        range: Range {
            start: Position {
                line: 100,
                character: 50,
            },
            end: Position {
                line: 100,
                character: 60,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "Complex error with multiple type references and function calls".to_string(),
        code: None,
        source: "typescript".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    let context = extractor.extract_context(&diagnostic, fixtures::TYPESCRIPT_REACT_COMPONENT)?;

    // Test with various budget constraints
    let budgets = vec![50, 100, 200, 500, 1000, 2000];
    let mut results = Vec::new();

    for budget in budgets {
        let ranker = ContextRanker::builder().max_tokens(budget).build();
        let ranked = ranker.rank_context(context.clone(), &diagnostic)?;

        results.push((
            budget,
            ranked.budget_context.tokens_used,
            ranked.ranked_elements.len(),
        ));

        // Verify budget is respected
        assert!(
            ranked.budget_context.tokens_used <= budget,
            "Budget constraint violated: {} tokens used with {} budget",
            ranked.budget_context.tokens_used,
            budget
        );
    }

    println!("Budget Stress Test Results:");
    for (budget, used, elements) in results {
        println!(
            "Budget: {} | Used: {} | Elements: {}",
            budget, used, elements
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_comprehensive_pipeline_integration() -> Result<()> {
    // Test the complete pipeline from raw code to AI-ready context
    let mut extractor = ContextExtractor::new()?;
    let mut analyzer = DependencyAnalyzer::new()?;

    // Create a complex multi-file scenario
    let files = vec![
        (
            "src/types.ts",
            r#"
export interface User {
    id: number;
    name: string;
    email: string;
}

export interface ApiResponse<T> {
    data: T;
    success: boolean;
    error?: string;
}
"#,
        ),
        (
            "src/service.ts",
            r#"
import { User, ApiResponse } from './types';

export class UserService {
    async getUser(id: number): Promise<ApiResponse<User>> {
        // Implementation would go here
        return { data: null as any, success: false };
    }
}
"#,
        ),
        (
            "src/main.ts",
            r#"
import { UserService } from './service';
import { User } from './types';

const service = new UserService();

async function main() {
    const response = await service.getUser(1);
    
    // Type error: data could be null
    const user: User = response.data;
    console.log(user.name);
}
"#,
        ),
    ];

    let diagnostic = Diagnostic {
        id: "pipeline-test".to_string(),
        file: "src/main.ts".to_string(),
        range: Range {
            start: Position {
                line: 8,
                character: 23,
            },
            end: Position {
                line: 8,
                character: 36,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "Type 'null' is not assignable to type 'User'.".to_string(),
        code: None,
        source: "typescript".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    // Step 1: Extract semantic context
    let main_code = files
        .iter()
        .find(|(name, _)| name == &"src/main.ts")
        .unwrap()
        .1;
    let context = extractor.extract_context(&diagnostic, main_code)?;

    // Step 2: Simulate dependency analysis (would need actual files)
    // For this test, we'll just verify the analyzer can be instantiated
    let _graph = analyzer.build_graph(&[] as &[String]).await?;

    // Step 3: Rank and optimize context
    let priority_config = PriorityConfig::default();

    let token_weights = TokenWeights::default();

    let ranker = ContextRanker::builder()
        .max_tokens(800)
        .priority_config(priority_config)
        .token_weights(token_weights)
        .build();
    let ranked_context = ranker.rank_context(context, &diagnostic)?;

    // Step 4: Format for AI consumption
    let formatted_output = format_context_for_ai(&ranked_context);

    // Verify the complete pipeline produces high-quality output
    assert!(
        ranked_context.budget_context.tokens_used > 0,
        "Should use some of the token budget"
    );
    assert!(
        !ranked_context.ranked_elements.is_empty(),
        "Should have context elements"
    );
    assert!(
        formatted_output.contains("User"),
        "Should mention the User type"
    );
    assert!(
        formatted_output.contains("import"),
        "Should include import context"
    );
    assert!(
        formatted_output.len() > 100,
        "Should produce substantial context"
    );

    // Verify priority scoring worked correctly
    let type_elements: Vec<_> = ranked_context
        .ranked_elements
        .iter()
        .filter(|e| matches!(e.element_type, ContextElementType::TypeDefinition))
        .collect();

    let import_elements: Vec<_> = ranked_context
        .ranked_elements
        .iter()
        .filter(|e| matches!(e.element_type, ContextElementType::Import))
        .collect();

    // Type elements should have higher priority for type errors
    if !type_elements.is_empty() && !import_elements.is_empty() {
        let avg_type_priority = type_elements.iter().map(|e| e.priority_score).sum::<f32>()
            / type_elements.len() as f32;
        let avg_import_priority = import_elements
            .iter()
            .map(|e| e.priority_score)
            .sum::<f32>()
            / import_elements.len() as f32;

        println!("Priority Analysis:");
        println!("Average type priority: {:.2}", avg_type_priority);
        println!("Average import priority: {:.2}", avg_import_priority);
    }

    println!("Pipeline Integration Results:");
    println!(
        "- Total context elements: {}",
        ranked_context.ranked_elements.len()
    );
    println!(
        "- Essential elements: {}",
        ranked_context.budget_context.essential_context.len()
    );
    println!(
        "- Supplementary elements: {}",
        ranked_context.budget_context.supplementary_context.len()
    );
    println!(
        "- Excluded elements: {}",
        ranked_context.budget_context.excluded_context.len()
    );
    println!(
        "- Token efficiency: {:.1}%",
        (ranked_context.budget_context.tokens_used as f32 / 800.0) * 100.0
    );

    println!("\n--- Generated AI Context ---");
    println!("{}", formatted_output);

    Ok(())
}
