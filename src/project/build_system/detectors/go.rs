use anyhow::{Context, Result};
use std::path::Path;

use crate::project::build_system::types::{BuildCommands, BuildConfig, BuildSystem};
use super::{BuildSystemDetector, utils};

pub struct GoDetector;

impl BuildSystemDetector for GoDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Go
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "go.mod")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let go_mod_path = utils::get_file_path(project_root, "go.mod");
        let content = utils::read_file(&go_mod_path)?;

        let mut commands = BuildCommands::default();
        commands.build = Some("go build ./...".to_string());
        commands.test = Some("go test ./...".to_string());
        commands.run = Some("go run .".to_string());
        commands.clean = Some("go clean".to_string());
        commands.format = Some("go fmt ./...".to_string());
        commands.lint = Some("go vet ./...".to_string());

        // Additional common Go commands
        commands.custom.insert("mod-download".to_string(), "go mod download".to_string());
        commands.custom.insert("mod-tidy".to_string(), "go mod tidy".to_string());
        commands.custom.insert("mod-vendor".to_string(), "go mod vendor".to_string());
        commands.custom.insert("mod-verify".to_string(), "go mod verify".to_string());
        commands.custom.insert("test-race".to_string(), "go test -race ./...".to_string());
        commands.custom.insert("test-cover".to_string(), "go test -cover ./...".to_string());
        commands.custom.insert("bench".to_string(), "go test -bench=. ./...".to_string());

        // Check for common Go tools
        if utils::has_file(project_root, ".golangci.yml") || 
           utils::has_file(project_root, ".golangci.yaml") || 
           utils::has_file(project_root, ".golangci.toml") {
            commands.lint = Some("golangci-lint run".to_string());
        }

        // Check for Makefile with Go targets
        if utils::has_file(project_root, "Makefile") {
            if let Ok(makefile_content) = utils::read_file(&utils::get_file_path(project_root, "Makefile")) {
                if makefile_content.contains("go build") || makefile_content.contains("go test") {
                    // Makefile likely has Go-specific targets
                    commands.custom.insert("make-build".to_string(), "make build".to_string());
                    commands.custom.insert("make-test".to_string(), "make test".to_string());
                }
            }
        }

        // Extract module name and dependencies
        let mut module_name = String::new();
        let mut dependencies = vec![];

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("module ") {
                module_name = line[7..].trim().to_string();
            } else if line.starts_with("require ") && line.contains("(") {
                // Multi-line require block
                continue;
            } else if line.starts_with("require ") {
                // Single line require
                if let Some(dep) = line[8..].split_whitespace().next() {
                    dependencies.push(dep.to_string());
                }
            } else if !line.starts_with("module") && !line.starts_with("go ") && 
                      !line.starts_with("require") && !line.starts_with("replace") && 
                      !line.starts_with("exclude") && !line.starts_with("//") &&
                      !line.is_empty() && !line.starts_with(")") {
                // Inside a require block
                if let Some(dep) = line.split_whitespace().next() {
                    dependencies.push(dep.to_string());
                }
            }
        }

        let mut config_files = vec![go_mod_path];
        
        if utils::has_file(project_root, "go.sum") {
            config_files.push(utils::get_file_path(project_root, "go.sum"));
        }
        
        if utils::has_file(project_root, "go.work") {
            config_files.push(utils::get_file_path(project_root, "go.work"));
            // For workspaces
            commands.custom.insert("work-sync".to_string(), "go work sync".to_string());
        }
        
        if utils::has_file(project_root, ".golangci.yml") {
            config_files.push(utils::get_file_path(project_root, ".golangci.yml"));
        } else if utils::has_file(project_root, ".golangci.yaml") {
            config_files.push(utils::get_file_path(project_root, ".golangci.yaml"));
        } else if utils::has_file(project_root, ".golangci.toml") {
            config_files.push(utils::get_file_path(project_root, ".golangci.toml"));
        }

        // Check for tools.go (common pattern for tool dependencies)
        if utils::has_file(project_root, "tools.go") {
            config_files.push(utils::get_file_path(project_root, "tools.go"));
        }

        Ok(BuildConfig {
            system: BuildSystem::Go,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies,
            dev_dependencies: vec![], // Go doesn't distinguish dev dependencies in go.mod
        })
    }
}