use lsp_bridge::project::{BuildSystem, build_system::detectors::{detect_build_system, monorepo}};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_lerna_detection() {
    let temp_dir = TempDir::new().unwrap();
    let lerna_json = r#"{
        "version": "0.0.0",
        "packages": ["packages/*"],
        "npmClient": "npm"
    }"#;
    
    fs::write(temp_dir.path().join("lerna.json"), lerna_json).unwrap();
    fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
    
    let config = detect_build_system(temp_dir.path()).unwrap();
    assert_eq!(config.system, BuildSystem::Lerna, "Expected Lerna to be detected");
    assert!(config.commands.build.unwrap().contains("lerna"));
}

#[test]
fn test_nx_detection() {
    let temp_dir = TempDir::new().unwrap();
    let nx_json = r#"{
        "npmScope": "myorg",
        "affected": {
            "defaultBase": "main"
        }
    }"#;
    
    fs::write(temp_dir.path().join("nx.json"), nx_json).unwrap();
    fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
    
    let config = detect_build_system(temp_dir.path()).unwrap();
    assert_eq!(config.system, BuildSystem::Nx, "Expected Nx to be detected");
    
    assert!(config.commands.build.unwrap().contains("nx"));
}

#[test]
fn test_rush_detection() {
    let temp_dir = TempDir::new().unwrap();
    let rush_json = r#"{
        "rushVersion": "5.0.0",
        "projects": []
    }"#;
    
    fs::write(temp_dir.path().join("rush.json"), rush_json).unwrap();
    
    let config = detect_build_system(temp_dir.path()).unwrap();
    assert_eq!(config.system, BuildSystem::Rush, "Expected Rush to be detected");
    
    assert!(config.commands.build.unwrap().contains("rush"));
}

#[test]
fn test_yarn_workspaces_detection() {
    let temp_dir = TempDir::new().unwrap();
    let package_json = r#"{
        "name": "monorepo",
        "private": true,
        "workspaces": ["packages/*"],
        "scripts": {
            "build": "yarn workspaces run build"
        }
    }"#;
    
    fs::write(temp_dir.path().join("package.json"), package_json).unwrap();
    fs::write(temp_dir.path().join("yarn.lock"), "").unwrap();
    
    let config = detect_build_system(temp_dir.path()).unwrap();
    assert_eq!(config.system, BuildSystem::YarnWorkspaces, "Expected Yarn Workspaces to be detected");
}

#[test]
fn test_pnpm_workspaces_detection() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_yaml = r#"packages:
  - 'packages/*'
  - 'apps/*'"#;
    
    fs::write(temp_dir.path().join("pnpm-workspace.yaml"), workspace_yaml).unwrap();
    fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
    
    let config = detect_build_system(temp_dir.path()).unwrap();
    assert_eq!(config.system, BuildSystem::PnpmWorkspaces, "Expected pnpm Workspaces to be detected");
    
    assert!(config.commands.build.unwrap().contains("pnpm"));
}

#[test]
fn test_npm_workspaces_detection() {
    let temp_dir = TempDir::new().unwrap();
    let package_json = r#"{
        "name": "monorepo",
        "workspaces": ["packages/*"]
    }"#;
    
    fs::write(temp_dir.path().join("package.json"), package_json).unwrap();
    fs::write(temp_dir.path().join("package-lock.json"), "{}").unwrap();
    
    let config = detect_build_system(temp_dir.path()).unwrap();
    assert_eq!(config.system, BuildSystem::NpmWorkspaces, "Expected npm Workspaces to be detected");
    
    assert!(config.commands.build.unwrap().contains("npm"));
}

#[test]
fn test_workspace_with_nested_packages() {
    let temp_dir = TempDir::new().unwrap();
    let package_json = r#"{
        "name": "monorepo",
        "workspaces": {
            "packages": ["packages/*", "tools/*"]
        }
    }"#;
    
    fs::write(temp_dir.path().join("package.json"), package_json).unwrap();
    fs::write(temp_dir.path().join("yarn.lock"), "").unwrap();
    
    let config = detect_build_system(temp_dir.path()).unwrap();
    assert_eq!(config.system, BuildSystem::YarnWorkspaces, "Expected Yarn Workspaces to be detected");
}

#[test]
fn test_monorepo_precedence() {
    // Test that monorepo detection takes precedence over regular package managers
    let temp_dir = TempDir::new().unwrap();
    
    // Create both lerna.json and regular npm files
    let lerna_json = r#"{
        "version": "0.0.0",
        "packages": ["packages/*"]
    }"#;
    
    fs::write(temp_dir.path().join("lerna.json"), lerna_json).unwrap();
    fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
    fs::write(temp_dir.path().join("package-lock.json"), "{}").unwrap();
    
    let config = detect_build_system(temp_dir.path()).unwrap();
    assert_eq!(config.system, BuildSystem::Lerna, "Lerna should take precedence");
}

#[test]
fn test_detect_workspace_packages() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a lerna.json with packages
    let lerna_json = r#"{
        "version": "0.0.0",
        "packages": ["packages/core", "packages/cli"]
    }"#;
    
    fs::write(temp_dir.path().join("lerna.json"), lerna_json).unwrap();
    
    let packages = monorepo::detect_workspace_packages(temp_dir.path()).unwrap();
    assert_eq!(packages.len(), 2);
    assert!(packages[0].ends_with("packages/core"));
    assert!(packages[1].ends_with("packages/cli"));
}