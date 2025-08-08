use anyhow::Result;
use std::path::Path;
use std::collections::HashSet;
use regex::Regex;

use crate::project::build_system::types::{BuildCommands, BuildConfig, BuildSystem};
use super::{BuildSystemDetector, utils};

pub struct MakeDetector;

impl BuildSystemDetector for MakeDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Make
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "Makefile") ||
        utils::has_file(project_root, "makefile") ||
        utils::has_file(project_root, "GNUmakefile")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        // Find the actual Makefile
        let makefile_path = if utils::has_file(project_root, "GNUmakefile") {
            utils::get_file_path(project_root, "GNUmakefile")
        } else if utils::has_file(project_root, "Makefile") {
            utils::get_file_path(project_root, "Makefile")
        } else {
            utils::get_file_path(project_root, "makefile")
        };

        let content = utils::read_file(&makefile_path)?;
        
        // Extract targets from Makefile
        let targets = extract_makefile_targets(&content);
        
        let mut commands = BuildCommands::default();
        
        // Map common target names to standard commands
        for target in &targets {
            match target.as_str() {
                "build" | "all" => {
                    commands.build = Some(format!("make {target}"));
                }
                "test" | "tests" | "check" => {
                    if commands.test.is_none() {
                        commands.test = Some(format!("make {target}"));
                    }
                }
                "lint" | "lint-check" | "checkstyle" => {
                    if commands.lint.is_none() {
                        commands.lint = Some(format!("make {target}"));
                    }
                }
                "format" | "fmt" | "lint-fix" => {
                    if commands.format.is_none() {
                        commands.format = Some(format!("make {target}"));
                    }
                }
                "run" | "start" | "serve" => {
                    if commands.run.is_none() {
                        commands.run = Some(format!("make {target}"));
                    }
                }
                "clean" => {
                    commands.clean = Some("make clean".to_string());
                }
                _ => {
                    // Add other targets as custom commands
                    if !target.starts_with('.') && !target.contains('%') {
                        commands.custom.insert(target.clone(), format!("make {target}"));
                    }
                }
            }
        }

        // If no build command found but "all" target exists, use it
        if commands.build.is_none() && targets.contains("all") {
            commands.build = Some("make all".to_string());
        } else if commands.build.is_none() {
            // Default to just "make"
            commands.build = Some("make".to_string());
        }

        // Common patterns for additional commands
        if targets.contains("install") {
            commands.custom.insert("install".to_string(), "make install".to_string());
        }
        if targets.contains("dist") || targets.contains("package") {
            let target = if targets.contains("dist") { "dist" } else { "package" };
            commands.custom.insert("package".to_string(), format!("make {target}"));
        }
        if targets.contains("doc") || targets.contains("docs") {
            let target = if targets.contains("doc") { "doc" } else { "docs" };
            commands.custom.insert("docs".to_string(), format!("make {target}"));
        }

        let mut config_files = vec![makefile_path];
        
        // Check for included makefiles
        let include_regex = Regex::new(r"(?m)^[[:space:]]*(?:-)?include[[:space:]]+(.+)$").unwrap();
        for cap in include_regex.captures_iter(&content) {
            if let Some(included) = cap.get(1) {
                let included_files: Vec<&str> = included.as_str().split_whitespace().collect();
                for file in included_files {
                    let file_path = project_root.join(file);
                    if file_path.exists() {
                        config_files.push(file_path);
                    }
                }
            }
        }

        // Check for common build configuration files that might be used with Make
        if utils::has_file(project_root, "configure") {
            config_files.push(utils::get_file_path(project_root, "configure"));
            commands.custom.insert("configure".to_string(), "./configure".to_string());
        }
        if utils::has_file(project_root, "configure.ac") {
            config_files.push(utils::get_file_path(project_root, "configure.ac"));
        }
        if utils::has_file(project_root, "Makefile.am") {
            config_files.push(utils::get_file_path(project_root, "Makefile.am"));
        }
        if utils::has_file(project_root, "CMakeLists.txt") {
            // Project might use CMake to generate Makefiles
            config_files.push(utils::get_file_path(project_root, "CMakeLists.txt"));
            commands.custom.insert("cmake-configure".to_string(), "cmake .".to_string());
        }

        Ok(BuildConfig {
            system: BuildSystem::Make,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies: vec![], // Make doesn't have a standard dependency format
            dev_dependencies: vec![],
        })
    }
}

/// Extract targets from Makefile content
fn extract_makefile_targets(content: &str) -> HashSet<String> {
    let mut targets = HashSet::new();
    let target_regex = Regex::new(r"^([a-zA-Z0-9_.-]+)\s*:").unwrap();
    
    for line in content.lines() {
        // Skip comments and empty lines
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Look for target definitions
        if let Some(caps) = target_regex.captures(line) {
            if let Some(target) = caps.get(1) {
                let target_name = target.as_str();
                // Skip special targets and pattern rules
                if !target_name.starts_with('.') && !target_name.contains('%') {
                    targets.insert(target_name.to_string());
                }
            }
        }
    }
    
    // Add some common implicit targets if they're referenced
    if content.contains("$(MAKE) all") || content.contains("${MAKE} all") {
        targets.insert("all".to_string());
    }
    if content.contains("$(MAKE) clean") || content.contains("${MAKE} clean") {
        targets.insert("clean".to_string());
    }
    
    targets
}