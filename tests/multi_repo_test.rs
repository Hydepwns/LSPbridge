//! Tests for multi-repository support

use lsp_bridge::multi_repo::{
    collaboration::{TeamDatabase, TeamMember, TeamRole},
    monorepo::WorkspaceType,
    CrossRepoAnalyzer, DiagnosticAggregator, MonorepoDetector, RepositoryInfo, RepositoryRegistry,
};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio;

#[tokio::test]
async fn test_repository_registry() {
    let temp_dir = TempDir::new().unwrap();
    let registry_path = temp_dir.path().join("registry.db");

    let registry = RepositoryRegistry::load_or_create(&registry_path)
        .await
        .unwrap();

    // Register a repository
    let repo_info = RepositoryInfo {
        id: "test-repo-1".to_string(),
        name: "Test Repository".to_string(),
        path: PathBuf::from("/test/repo"),
        remote_url: Some("https://github.com/test/repo".to_string()),
        primary_language: Some("rust".to_string()),
        build_system: Some("cargo".to_string()),
        is_monorepo_member: false,
        monorepo_id: None,
        tags: vec!["test".to_string()],
        active: true,
        last_diagnostic_run: None,
        metadata: serde_json::json!({}),
    };

    registry.register(repo_info.clone()).await.unwrap();

    // Retrieve repository
    let retrieved = registry.get("test-repo-1").await.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "Test Repository");
    assert_eq!(retrieved.primary_language.as_deref(), Some("rust"));
}

#[tokio::test]
async fn test_monorepo_detection() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a simple npm workspace structure
    std::fs::write(
        root.join("package.json"),
        r#"{
            "name": "test-monorepo",
            "workspaces": ["packages/*"]
        }"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("packages/app")).unwrap();
    std::fs::write(
        root.join("packages/app/package.json"),
        r#"{
            "name": "@test/app",
            "version": "1.0.0"
        }"#,
    )
    .unwrap();

    // Detect monorepo
    let layout = MonorepoDetector::detect(root).await.unwrap();
    assert!(layout.is_some());

    let layout = layout.unwrap();
    assert_eq!(layout.workspace_type, WorkspaceType::NpmWorkspace);
    assert_eq!(layout.subprojects.len(), 1);
    assert_eq!(layout.subprojects[0].name, "@test/app");
}

#[tokio::test]
async fn test_diagnostic_aggregation() {
    let aggregator = DiagnosticAggregator::new(2);

    // Create test repositories
    let repos = vec![
        RepositoryInfo {
            id: "repo1".to_string(),
            name: "Repository 1".to_string(),
            path: PathBuf::from("/repo1"),
            remote_url: None,
            primary_language: Some("rust".to_string()),
            build_system: Some("cargo".to_string()),
            is_monorepo_member: false,
            monorepo_id: None,
            tags: vec![],
            active: true,
            last_diagnostic_run: None,
            metadata: serde_json::json!({}),
        },
        RepositoryInfo {
            id: "repo2".to_string(),
            name: "Repository 2".to_string(),
            path: PathBuf::from("/repo2"),
            remote_url: None,
            primary_language: Some("typescript".to_string()),
            build_system: Some("npm".to_string()),
            is_monorepo_member: false,
            monorepo_id: None,
            tags: vec![],
            active: true,
            last_diagnostic_run: None,
            metadata: serde_json::json!({}),
        },
    ];

    // Analyze repositories (will return empty results since we're not connecting to real LSP)
    let results = aggregator.analyze_repositories(repos).await.unwrap();
    assert_eq!(results.len(), 0); // No diagnostics in test environment
}

#[tokio::test]
async fn test_team_collaboration() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("team.db");

    let team_db = TeamDatabase::connect(&db_path).await.unwrap();

    // Add team member
    let member = TeamMember {
        id: "user1".to_string(),
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        role: TeamRole::Developer,
        active: true,
        last_activity: None,
    };

    team_db.add_member(member.clone()).await.unwrap();

    // Retrieve member
    let retrieved = team_db.get_member("user1").await.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "Test User");
    assert_eq!(retrieved.email, "test@example.com");
}

#[tokio::test]
async fn test_cross_repo_analyzer() {
    let analyzer = CrossRepoAnalyzer::new(true);

    // Create a test registry
    let temp_dir = TempDir::new().unwrap();
    let registry_path = temp_dir.path().join("registry.db");
    let registry = RepositoryRegistry::load_or_create(&registry_path)
        .await
        .unwrap();

    // Analyze type references (will be empty without actual repos)
    let type_refs = analyzer.analyze_type_references(&registry).await.unwrap();
    assert_eq!(type_refs.len(), 0); // No types in test environment
}
