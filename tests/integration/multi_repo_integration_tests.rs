//! Integration tests for multi-repository features

use lsp_bridge::{
    multi_repo::{
        DiagnosticAggregator,
        CrossRepoAnalyzer,
        // TODO: Enable when collaboration is implemented
        // TeamDatabase, TeamMember,
        RepositoryInfo, RepositoryRegistry,
        // TODO: Enable when MultiRepoContext is implemented
        // MultiRepoContext,
    },
    core::{
        Diagnostic, DiagnosticSeverity, Position, Range,
        // types::{DiagnosticTag, RelatedInformation},
    },
};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use uuid::Uuid;

/// Create a test diagnostic for a specific repository
fn create_repo_diagnostic(repo_name: &str, file: &str, line: u32, message: &str) -> Diagnostic {
    Diagnostic {
        id: Uuid::new_v4().to_string(),
        file: format!("{}/{}", repo_name, file),
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 10 },
        },
        severity: DiagnosticSeverity::Error,
        message: message.to_string(),
        code: Some("TEST001".to_string()),
        source: "test".to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

#[tokio::test]
#[ignore] // TODO: Re-enable when multi_repo modules are available
async fn test_repository_registry() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let registry_path = temp_dir.path().join("registry.json");
    
    // Create and save registry
    let mut registry = RepositoryRegistry::load_or_create(&registry_path).await?;
    
    let repo1 = RepositoryInfo {
        id: Uuid::new_v4().to_string(),
        name: "frontend".to_string(),
        path: PathBuf::from("/workspace/frontend"),
        remote_url: None,
        primary_language: Some("typescript".to_string()),
        build_system: Some("npm".to_string()),
        is_monorepo_member: false,
        monorepo_id: None,
        tags: vec!["web".to_string()],
        active: true,
        last_diagnostic_run: Some(chrono::Utc::now()),
        metadata: serde_json::json!({}),
    };
    
    let repo2 = RepositoryInfo {
        id: Uuid::new_v4().to_string(),
        name: "backend".to_string(),
        path: PathBuf::from("/workspace/backend"),  
        remote_url: None,
        primary_language: Some("rust".to_string()),
        build_system: Some("cargo".to_string()),
        is_monorepo_member: false,
        monorepo_id: None,
        tags: vec!["api".to_string()],
        active: true,
        last_diagnostic_run: Some(chrono::Utc::now()),
        metadata: serde_json::json!({}),
    };
    
    registry.add_repository(repo1.clone()).await?;
    registry.add_repository(repo2.clone()).await?;
    
    // Reload registry to test persistence
    let loaded_registry = RepositoryRegistry::load_or_create(&registry_path).await?;
    
    assert_eq!(loaded_registry.list_repositories().len(), 2);
    assert!(loaded_registry.get_repository("frontend").is_some());
    assert!(loaded_registry.get_repository("backend").is_some());
    
    Ok(())
}

#[tokio::test]
#[ignore] // TODO: Re-enable when multi_repo modules are available
async fn test_diagnostic_aggregation() -> Result<(), Box<dyn std::error::Error>> {
    let aggregator = DiagnosticAggregator::new(2); // Max 2 concurrent repos
    
    let repos = vec![
        RepositoryInfo {
            id: Uuid::new_v4().to_string(),
            name: "repo1".to_string(),
            path: PathBuf::from("/workspace/repo1"),
            language: "rust".to_string(),
            dependencies: vec![],
            last_analyzed: chrono::Utc::now(),
        },
        RepositoryInfo {
            id: Uuid::new_v4().to_string(),
            name: "repo2".to_string(),
            path: PathBuf::from("/workspace/repo2"),
            language: "typescript".to_string(),
            dependencies: vec![],
            last_analyzed: chrono::Utc::now(),
        },
    ];
    
    // In a real scenario, this would fetch diagnostics from each repo
    // For testing, we'll simulate the aggregation
    let result = aggregator.analyze_repositories(repos).await?;
    
    assert_eq!(result.total_repositories, 2);
    assert!(result.summary.repositories.contains_key("repo1"));
    assert!(result.summary.repositories.contains_key("repo2"));
    
    Ok(())
}

#[tokio::test]
#[ignore] // TODO: Re-enable when multi_repo modules are available
async fn test_cross_repo_analysis() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = CrossRepoAnalyzer::new(true); // Enable type sharing
    
    let diagnostics_repo1 = vec![
        create_repo_diagnostic("frontend", "api.ts", 10, "Cannot find name 'UserType'"),
        create_repo_diagnostic("frontend", "components.tsx", 20, "Property 'id' does not exist on type 'User'"),
    ];
    
    let diagnostics_repo2 = vec![
        create_repo_diagnostic("backend", "models.rs", 5, "cannot find type `UserType` in this scope"),
        create_repo_diagnostic("backend", "handlers.rs", 15, "field `id` of struct `User` is private"),
    ];
    
    let mut all_diagnostics = vec![];
    all_diagnostics.extend(diagnostics_repo1);
    all_diagnostics.extend(diagnostics_repo2);
    
    let patterns = analyzer.find_cross_repo_patterns(&all_diagnostics)?;
    
    // Should find pattern about missing UserType in both repos
    assert!(!patterns.is_empty());
    let user_type_pattern = patterns.iter()
        .find(|p| p.pattern_type.contains("Missing Type"));
    assert!(user_type_pattern.is_some());
    
    Ok(())
}

#[tokio::test]
#[ignore] // TODO: Re-enable when multi_repo modules are available
async fn test_collaboration_manager() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("team.db");
    
    let mut manager = CollaborationManager::new(Some(db_path)).await?;
    
    // Add team members
    let alice = TeamMember {
        id: Uuid::new_v4().to_string(),
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        expertise: vec!["rust".to_string(), "backend".to_string()],
        active: true,
    };
    
    let bob = TeamMember {
        id: Uuid::new_v4().to_string(),
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
        expertise: vec!["typescript".to_string(), "frontend".to_string()],
        active: true,
    };
    
    manager.add_team_member(alice.clone()).await?;
    manager.add_team_member(bob.clone()).await?;
    
    // Test assignment
    let diagnostic = create_repo_diagnostic("backend", "auth.rs", 42, "lifetime may not live long enough");
    let assignment = manager.assign_diagnostic(
        "backend-repo".to_string(),
        "auth.rs".to_string(),
        &diagnostic,
        &alice.id,
    ).await?;
    
    assert_eq!(assignment.assignee_id, alice.id);
    
    // Get assignments for Alice
    let alice_assignments = manager.get_member_assignments(&alice.id).await?;
    assert_eq!(alice_assignments.len(), 1);
    
    // Get team workload
    let workload = manager.get_team_workload().await?;
    assert_eq!(workload.len(), 2); // Both members in workload
    assert_eq!(workload.get(&alice.id).unwrap().open_assignments, 1);
    assert_eq!(workload.get(&bob.id).unwrap().open_assignments, 0);
    
    Ok(())
}

#[tokio::test]
#[ignore] // TODO: Re-enable when multi_repo modules are available
async fn test_multi_repo_context_integration() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config = lsp_bridge::core::config::UnifiedConfig {
        cache: lsp_bridge::core::config::CacheConfig {
            cache_dir: temp_dir.path().join("cache"),
            max_cache_size_mb: 100,
            cache_ttl_seconds: 3600,
            enable_compression: false,
        },
        ..Default::default()
    };
    
    // Create multi-repo context
    let context = MultiRepoContext::new(config.into()).await?;
    
    // Register repositories
    let frontend = RepositoryInfo {
        id: Uuid::new_v4().to_string(),
        name: "web-app".to_string(),
        path: PathBuf::from("/workspace/web-app"),
        language: "typescript".to_string(),
        dependencies: vec!["api-client".to_string()],
        last_analyzed: chrono::Utc::now(),
    };
    
    let backend = RepositoryInfo {
        id: Uuid::new_v4().to_string(),
        name: "api-server".to_string(),
        path: PathBuf::from("/workspace/api-server"),
        language: "rust".to_string(),
        dependencies: vec![],
        last_analyzed: chrono::Utc::now(),
    };
    
    context.registry.register_repository(frontend)?;
    context.registry.register_repository(backend)?;
    
    // Verify registration
    let repos = context.registry.list_repositories();
    assert_eq!(repos.len(), 2);
    
    // Test aggregation would normally process real diagnostics
    let result = context.aggregator.analyze_repositories(repos).await?;
    assert_eq!(result.total_repositories, 2);
    
    Ok(())
}

#[tokio::test]
#[ignore] // TODO: Re-enable when project structure module is available
async fn test_monorepo_detection() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    
    // Create monorepo structure
    std::fs::create_dir_all(temp_dir.path().join("packages/frontend"))?;
    std::fs::create_dir_all(temp_dir.path().join("packages/backend"))?;
    std::fs::create_dir_all(temp_dir.path().join("packages/shared"))?;
    
    // Add package.json files
    std::fs::write(
        temp_dir.path().join("package.json"),
        r#"{"name": "monorepo", "workspaces": ["packages/*"]}"#
    )?;
    
    std::fs::write(
        temp_dir.path().join("packages/frontend/package.json"),
        r#"{"name": "frontend", "dependencies": {"shared": "workspace:*"}}"#
    )?;
    
    std::fs::write(
        temp_dir.path().join("packages/backend/package.json"),
        r#"{"name": "backend", "dependencies": {"shared": "workspace:*"}}"#
    )?;
    
    // Test monorepo detection
    let is_monorepo = lsp_bridge::project::structure::detect_monorepo(temp_dir.path())?;
    assert!(is_monorepo);
    
    // Test workspace detection
    let workspaces = lsp_bridge::project::structure::find_workspaces(temp_dir.path())?;
    assert_eq!(workspaces.len(), 3);
    assert!(workspaces.iter().any(|w| w.ends_with("frontend")));
    assert!(workspaces.iter().any(|w| w.ends_with("backend")));
    assert!(workspaces.iter().any(|w| w.ends_with("shared")));
    
    Ok(())
}

#[tokio::test]
#[ignore] // TODO: Re-enable when multi_repo modules are available
async fn test_cross_repo_type_resolution() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = CrossRepoAnalyzer::new(true);
    
    // Simulate diagnostics from different repos referring to same types
    let frontend_diagnostics = vec![
        Diagnostic {
            id: Uuid::new_v4().to_string(),
            file: "frontend/src/api.ts".to_string(),
            range: Range {
                start: Position { line: 10, character: 15 },
                end: Position { line: 10, character: 25 },
            },
            severity: DiagnosticSeverity::Error,
            message: "Cannot find name 'ApiResponse'.".to_string(),
            code: Some("2304".to_string()),
            source: "typescript".to_string(),
            related_information: None,
            tags: None,
            data: None,
        },
    ];
    
    let backend_diagnostics = vec![
        Diagnostic {
            id: Uuid::new_v4().to_string(),
            file: "backend/src/types.rs".to_string(),
            range: Range {
                start: Position { line: 5, character: 10 },
                end: Position { line: 5, character: 21 },
            },
            severity: DiagnosticSeverity::Warning,
            message: "type `ApiResponse` is never used".to_string(),
            code: Some("dead_code".to_string()),
            source: "rust-analyzer".to_string(),
            related_information: None,
            tags: Some(vec![DiagnosticTag::Unnecessary]),
            data: None,
        },
    ];
    
    let mut all_diagnostics = vec![];
    all_diagnostics.extend(frontend_diagnostics);
    all_diagnostics.extend(backend_diagnostics);
    
    let patterns = analyzer.find_cross_repo_patterns(&all_diagnostics)?;
    
    // Should identify the connection between missing type in frontend and unused type in backend
    assert!(!patterns.is_empty());
    
    // Get specific recommendations
    let recommendations = analyzer.get_fix_recommendations(&patterns);
    assert!(!recommendations.is_empty());
    
    // Should recommend exporting the type from backend
    let export_recommendation = recommendations.iter()
        .find(|r| r.description.contains("export") || r.description.contains("share"));
    assert!(export_recommendation.is_some());
    
    Ok(())
}