use anyhow::Result;
use lsp_bridge::core::*;
use std::path::PathBuf;

#[tokio::test]
async fn test_semantic_context_extraction_typescript() -> Result<()> {
    let mut extractor = ContextExtractor::new()?;

    // Create a realistic TypeScript code sample
    let ts_code = r#"
interface User {
    id: number;
    name: string;
    email: string;
}

class UserService {
    private users: User[] = [];
    
    constructor(private apiUrl: string) {}
    
    async createUser(userData: Partial<User>): Promise<User> {
        const newUser: User = {
            id: this.generateId(),
            name: userData.name || "Unknown",
            email: userData.email || ""
        };
        
        this.users.push(newUser);
        return newUser;
    }
    
    private generateId(): number {
        return Math.max(...this.users.map(u => u.id), 0) + 1;
    }
    
    findUserByEmail(email: string): User | undefined {
        return this.users.find(user => user.email === email);
    }
}

// Error case: trying to use a property that doesn't exist
const service = new UserService("https://api.example.com");
const user = service.createUser({ name: "John", invalid_property: true });
"#;

    // Create a diagnostic for the error
    let diagnostic = Diagnostic {
        id: "ts-1".to_string(),
        file: "test.ts".to_string(),
        range: Range {
            start: Position { line: 34, character: 47 },
            end: Position { line: 34, character: 63 },
        },
        severity: DiagnosticSeverity::Error,
        message: "Object literal may only specify known properties, and 'invalid_property' does not exist in type 'Partial<User>'.".to_string(),
        code: None,
        source: "typescript".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    let context = extractor.extract_context(&diagnostic, ts_code)?;

    // Verify function context is extracted
    assert!(context.function_context.is_none()); // This is at top level, not in a function

    // Verify class context is extracted
    assert!(context.class_context.is_none()); // Not inside class scope

    // Verify imports are extracted (none in this example)
    assert!(context.imports.is_empty());

    // Verify type definitions are extracted
    assert!(!context.type_definitions.is_empty());
    let user_interface = context.type_definitions.iter().find(|t| t.name == "User");
    assert!(user_interface.is_some());

    // Verify call hierarchy is extracted
    assert!(!context.call_hierarchy.calls_outgoing.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_semantic_context_extraction_rust() -> Result<()> {
    let mut extractor = ContextExtractor::new()?;

    let rust_code = r#"
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

struct UserRepository {
    users: HashMap<u32, User>,
    next_id: u32,
}

impl UserRepository {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
            next_id: 1,
        }
    }
    
    fn create_user(&mut self, name: String, email: String) -> Result<&User, String> {
        let user = User {
            id: self.next_id,
            name,
            email,
        };
        
        self.users.insert(self.next_id, user);
        self.next_id += 1;
        
        // Error: returning a reference to a value that will be moved
        Ok(&self.users[&(self.next_id - 1)])
    }
    
    fn find_by_email(&self, email: &str) -> Option<&User> {
        self.users.values().find(|user| user.email == email)
    }
}

fn main() {
    let mut repo = UserRepository::new();
    let user = repo.create_user("John".to_string(), "john@example.com".to_string());
    println!("{:?}", user);
}
"#;

    let diagnostic = Diagnostic {
        id: "rust-1".to_string(),
        file: "test.rs".to_string(),
        range: Range {
            start: Position {
                line: 35,
                character: 12,
            },
            end: Position {
                line: 35,
                character: 44,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "cannot return reference to temporary value".to_string(),
        code: None,
        source: "rust-analyzer".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    let context = extractor.extract_context(&diagnostic, rust_code)?;

    // Verify function context (should be inside create_user method)
    assert!(context.function_context.is_some());
    let func_ctx = context.function_context.unwrap();
    assert_eq!(func_ctx.name, "create_user");

    // Verify type definitions include the User struct
    let user_struct = context.type_definitions.iter().find(|t| t.name == "User");
    assert!(user_struct.is_some());

    // Verify imports are captured
    assert!(!context.imports.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_dependency_analysis() -> Result<()> {
    let mut analyzer = DependencyAnalyzer::new()?;

    // Create test files in memory (simulated)
    let test_files = vec![
        PathBuf::from("src/types.ts"),
        PathBuf::from("src/service.ts"),
        PathBuf::from("src/main.ts"),
    ];

    // For this test, we'll just verify the analyzer can be created
    // and basic methods work. In a real scenario, we'd need actual files.

    // Test basic functionality
    let graph = analyzer.build_graph(&test_files).await;
    assert!(graph.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_context_ranking_and_budget() -> Result<()> {
    // Create a mock semantic context with various elements
    let context = SemanticContext {
        function_context: Some(FunctionContext {
            name: "processData".to_string(),
            signature: "function processData(data: any[]): ProcessedData[]".to_string(),
            body: "function processData(data: any[]): ProcessedData[] {\n  return data.map(item => ({ ...item, processed: true }));\n}".to_string(),
            start_line: 10,
            end_line: 12,
        }),
        class_context: Some(ClassContext {
            name: "DataProcessor".to_string(),
            kind: "class".to_string(),
            definition: "class DataProcessor {\n  // ... methods\n}".to_string(),
            start_line: 5,
            end_line: 20,
        }),
        imports: vec![
            ImportContext {
                statement: "import { ProcessedData } from './types'".to_string(),
                imported_names: vec!["ProcessedData".to_string()],
                source: Some("./types".to_string()),
                line: 1,
            }
        ],
        type_definitions: vec![
            TypeDefinition {
                name: "ProcessedData".to_string(),
                definition: "interface ProcessedData { id: number; processed: boolean; }".to_string(),
                kind: "interface".to_string(),
                line: 3,
            }
        ],
        local_variables: vec![
            VariableContext {
                name: "result".to_string(),
                type_annotation: Some("ProcessedData[]".to_string()),
                initialization: Some("[]".to_string()),
                line: 11,
            }
        ],
        call_hierarchy: CallHierarchy {
            calls_outgoing: vec![
                FunctionCall {
                    function_name: "map".to_string(),
                    call_site_line: 11,
                    arguments: vec!["item => ({ ...item, processed: true })".to_string()],
                    return_type: None,
                    file_path: None,
                }
            ],
            calls_incoming: vec![],
            analysis_depth: 1,
        },
        dependencies: vec![],
        relevance_score: 0.85,
        surrounding_code: std::collections::HashMap::new(),
    };

    let diagnostic = Diagnostic {
        id: "test-1".to_string(),
        file: "test.ts".to_string(),
        range: Range {
            start: Position {
                line: 11,
                character: 10,
            },
            end: Position {
                line: 11,
                character: 20,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "Type 'ProcessedData' is not assignable to type 'unknown'".to_string(),
        code: None,
        source: "typescript".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    // Test different token budgets
    let ranker_small = ContextRanker::builder().max_tokens(100).build();
    let ranker_large = ContextRanker::builder().max_tokens(1000).build();

    let ranked_small = ranker_small.rank_context(context.clone(), &diagnostic)?;
    let ranked_large = ranker_large.rank_context(context.clone(), &diagnostic)?;

    // Verify that larger budget includes more context
    assert!(ranked_large.budget_context.tokens_used >= ranked_small.budget_context.tokens_used);

    // Verify essential context is prioritized
    assert!(!ranked_small.budget_context.essential_context.is_empty());

    // Test formatting for AI
    let formatted = format_context_for_ai(&ranked_large);
    assert!(formatted.contains("Essential Context"));
    assert!(formatted.contains("ProcessedData"));

    Ok(())
}

#[tokio::test]
async fn test_end_to_end_context_pipeline() -> Result<()> {
    // Test the complete pipeline from diagnostic to ranked context
    let typescript_code = r#"
interface ApiResponse<T> {
    data: T;
    status: number;
    message: string;
}

class ApiClient {
    private baseUrl: string;
    
    constructor(baseUrl: string) {
        this.baseUrl = baseUrl;
    }
    
    async fetchUsers(): Promise<ApiResponse<User[]>> {
        const response = await fetch(`${this.baseUrl}/users`);
        const data = await response.json();
        
        // Type error: missing required properties
        return {
            data: data.users,
            status: response.status
            // missing 'message' property
        };
    }
    
    private handleError(error: Error): void {
        console.error('API Error:', error.message);
    }
}

interface User {
    id: number;
    name: string;
    email: string;
}
"#;

    let diagnostic = Diagnostic {
        id: "end-to-end-1".to_string(),
        file: "api-client.ts".to_string(),
        range: Range {
            start: Position { line: 17, character: 16 },
            end: Position { line: 21, character: 9 },
        },
        severity: DiagnosticSeverity::Error,
        message: "Property 'message' is missing in type '{ data: any; status: number; }' but required in type 'ApiResponse<User[]>'.".to_string(),
        code: None,
        source: "typescript".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    // Step 1: Extract semantic context
    let mut extractor = ContextExtractor::new()?;
    let context = extractor.extract_context(&diagnostic, typescript_code)?;

    // Step 2: Rank and optimize context
    let ranker = ContextRanker::builder().max_tokens(500).build(); // Medium token budget
    let ranked_context = ranker.rank_context(context, &diagnostic)?;

    // Step 3: Format for AI consumption
    let formatted_output = format_context_for_ai(&ranked_context);

    // Verify the pipeline produces useful output
    assert!(formatted_output.contains("ApiResponse"));
    assert!(formatted_output.contains("fetchUsers"));
    assert!(formatted_output.contains("Context Summary"));
    assert!(ranked_context.budget_context.tokens_used > 0);
    assert!(ranked_context.ranked_elements.len() > 0);

    // Verify priority scoring works
    let type_elements: Vec<_> = ranked_context
        .ranked_elements
        .iter()
        .filter(|e| matches!(e.element_type, ContextElementType::TypeDefinition))
        .collect();

    // Should have high priority for types mentioned in error
    let api_response_element = type_elements
        .iter()
        .find(|e| e.relevance_explanation.contains("ApiResponse"));
    assert!(api_response_element.is_some());
    assert!(api_response_element.unwrap().priority_score > 0.7);

    println!("Generated context output:\n{}", formatted_output);

    Ok(())
}

#[tokio::test]
async fn test_complex_dependency_scenario() -> Result<()> {
    // Test with a more complex multi-file scenario
    let main_file = r#"
import { UserService } from './services/user-service';
import { DatabaseConnection } from './database/connection';
import { Logger } from './utils/logger';
import { User, CreateUserRequest } from './types/user';

class Application {
    private userService: UserService;
    private logger: Logger;
    
    constructor(dbConnection: DatabaseConnection) {
        this.userService = new UserService(dbConnection);
        this.logger = new Logger('Application');
    }
    
    async createUser(request: CreateUserRequest): Promise<User> {
        try {
            this.logger.info('Creating new user', { email: request.email });
            
            // Type error: passing wrong parameter type
            const user = await this.userService.create(request.email, request.name);
            
            this.logger.info('User created successfully', { userId: user.id });
            return user;
        } catch (error) {
            this.logger.error('Failed to create user', error);
            throw error;
        }
    }
}
"#;

    let diagnostic = Diagnostic {
        id: "complex-1".to_string(),
        file: "src/app.ts".to_string(),
        range: Range {
            start: Position {
                line: 18,
                character: 62,
            },
            end: Position {
                line: 18,
                character: 77,
            },
        },
        severity: DiagnosticSeverity::Error,
        message:
            "Argument of type 'string' is not assignable to parameter of type 'CreateUserRequest'."
                .to_string(),
        code: None,
        source: "typescript".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    let mut extractor = ContextExtractor::new()?;
    let context = extractor.extract_context(&diagnostic, main_file)?;

    // Test that imports are properly captured
    assert!(!context.imports.is_empty());
    assert!(context
        .imports
        .iter()
        .any(|i| i.imported_names.contains(&"UserService".to_string())));
    assert!(context
        .imports
        .iter()
        .any(|i| i.imported_names.contains(&"CreateUserRequest".to_string())));

    // Test context ranking with this complex scenario
    let ranker = ContextRanker::builder().max_tokens(800).build();
    let ranked = ranker.rank_context(context, &diagnostic)?;

    // Verify that relevant imports get high priority
    let import_elements: Vec<_> = ranked
        .ranked_elements
        .iter()
        .filter(|e| matches!(e.element_type, ContextElementType::Import))
        .collect();

    let user_service_import = import_elements
        .iter()
        .find(|e| e.relevance_explanation.contains("UserService"));
    assert!(user_service_import.is_some());

    Ok(())
}

#[tokio::test]
async fn test_budget_optimization_edge_cases() -> Result<()> {
    // Test edge cases in budget optimization
    let large_context = SemanticContext {
        function_context: Some(FunctionContext {
            name: "veryLargeFunction".to_string(),
            signature: "function veryLargeFunction()".to_string(),
            body: "x".repeat(1000), // Very large function body
            start_line: 1,
            end_line: 100,
        }),
        class_context: None,
        imports: (0..20)
            .map(|i| ImportContext {
                statement: format!("import {{ Module{} }} from './module{}'", i, i),
                imported_names: vec![format!("Module{}", i)],
                source: Some(format!("./module{}", i)),
                line: i as u32,
            })
            .collect(),
        type_definitions: vec![],
        local_variables: vec![],
        call_hierarchy: CallHierarchy::default(),
        dependencies: vec![],
        relevance_score: 0.5,
        surrounding_code: std::collections::HashMap::new(),
    };

    let diagnostic = Diagnostic {
        id: "edge-case-1".to_string(),
        file: "test.ts".to_string(),
        range: Range {
            start: Position {
                line: 50,
                character: 10,
            },
            end: Position {
                line: 50,
                character: 20,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "Some error".to_string(),
        code: None,
        source: "test".to_string(),
        tags: None,
        related_information: None,
        data: None,
    };

    // Test with very small budget
    let tiny_ranker = ContextRanker::builder().max_tokens(10).build();
    let ranked_tiny = tiny_ranker.rank_context(large_context.clone(), &diagnostic)?;

    // Should exclude most context due to budget constraints
    assert!(ranked_tiny.budget_context.excluded_context.len() > 0);
    assert!(ranked_tiny.budget_context.tokens_used <= 10);

    // Test with zero budget
    let zero_ranker = ContextRanker::builder().max_tokens(0).build();
    let ranked_zero = zero_ranker.rank_context(large_context, &diagnostic)?;

    // Should exclude everything
    assert_eq!(ranked_zero.budget_context.tokens_used, 0);
    assert!(ranked_zero.budget_context.essential_context.is_empty());
    assert!(ranked_zero.budget_context.supplementary_context.is_empty());

    Ok(())
}
