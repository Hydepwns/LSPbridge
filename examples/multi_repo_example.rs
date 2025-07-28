//! Example demonstrating multi-repository support

use lsp_bridge::multi_repo::{MultiRepoConfig, MultiRepoContext, RepositoryInfo};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize multi-repo context
    let config = MultiRepoConfig::default();
    let mut context = MultiRepoContext::new(config).await?;

    // Register some repositories
    let repo1 = RepositoryInfo {
        id: "frontend".to_string(),
        name: "Frontend App".to_string(),
        path: PathBuf::from("/projects/myapp/frontend"),
        remote_url: Some("https://github.com/myorg/frontend".to_string()),
        primary_language: Some("typescript".to_string()),
        build_system: Some("npm".to_string()),
        is_monorepo_member: false,
        monorepo_id: None,
        tags: vec!["web".to_string(), "react".to_string()],
        active: true,
        last_diagnostic_run: None,
        metadata: serde_json::json!({}),
    };

    let repo2 = RepositoryInfo {
        id: "backend".to_string(),
        name: "Backend API".to_string(),
        path: PathBuf::from("/projects/myapp/backend"),
        remote_url: Some("https://github.com/myorg/backend".to_string()),
        primary_language: Some("rust".to_string()),
        build_system: Some("cargo".to_string()),
        is_monorepo_member: false,
        monorepo_id: None,
        tags: vec!["api".to_string(), "server".to_string()],
        active: true,
        last_diagnostic_run: None,
        metadata: serde_json::json!({}),
    };

    context.register_repo(repo1).await?;
    context.register_repo(repo2).await?;

    println!("✅ Registered 2 repositories");

    // Detect monorepo structure
    let monorepo_path = PathBuf::from("/projects/mymonorepo");
    if let Some(layout) = context.detect_monorepo(&monorepo_path).await? {
        println!("\n📦 Detected monorepo:");
        println!("  Type: {:?}", layout.workspace_type);
        println!("  Subprojects: {}", layout.subprojects.len());

        for subproject in &layout.subprojects {
            println!(
                "    - {} ({})",
                subproject.name,
                subproject.relative_path.display()
            );
        }
    }

    // Analyze diagnostics across repositories
    println!("\n🔍 Analyzing diagnostics across repositories...");
    let diagnostics = context.analyze_all().await?;

    if diagnostics.is_empty() {
        println!("  No diagnostics found (LSP not connected in example)");
    } else {
        println!("  Found {} diagnostics", diagnostics.len());

        for diag in diagnostics.iter().take(5) {
            println!("\n  📍 {}", diag.repository_name);
            println!("     File: {}", diag.relative_path.display());
            println!("     Message: {}", diag.diagnostic.message);
            println!("     Impact: {:.2}", diag.cross_repo_impact);

            if !diag.related_diagnostics.is_empty() {
                println!(
                    "     Related issues in {} other repos",
                    diag.related_diagnostics.len()
                );
            }
        }
    }

    // Find cross-repository type references
    println!("\n🔗 Analyzing cross-repository type references...");
    let type_refs = context.find_cross_repo_types().await?;

    if type_refs.is_empty() {
        println!("  No cross-repo type references found");
    } else {
        for type_ref in type_refs.iter().take(3) {
            println!("\n  Type: {}", type_ref.type_name);
            println!("  Defined in: {}", type_ref.source_repo_id);
            println!(
                "  Used in {} other repositories",
                type_ref.target_repos.len()
            );
        }
    }

    Ok(())
}
