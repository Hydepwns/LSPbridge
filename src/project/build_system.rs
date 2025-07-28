use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildSystem {
    Cargo,  // Rust
    Npm,    // Node.js
    Yarn,   // Node.js
    Pnpm,   // Node.js
    Poetry, // Python
    Pip,    // Python
    Maven,  // Java
    Gradle, // Java
    Go,     // Go
    Make,   // Generic
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub system: BuildSystem,
    pub root_path: PathBuf,
    pub config_files: Vec<PathBuf>,
    pub commands: BuildCommands,
    pub dependencies: Vec<String>,
    pub dev_dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCommands {
    pub build: Option<String>,
    pub test: Option<String>,
    pub lint: Option<String>,
    pub format: Option<String>,
    pub run: Option<String>,
    pub clean: Option<String>,
    pub custom: HashMap<String, String>,
}

impl Default for BuildCommands {
    fn default() -> Self {
        Self {
            build: None,
            test: None,
            lint: None,
            format: None,
            run: None,
            clean: None,
            custom: HashMap::new(),
        }
    }
}

pub struct BuildSystemDetector;

impl BuildSystemDetector {
    pub fn detect(project_root: &Path) -> Result<BuildConfig> {
        // Check for various build system files
        if project_root.join("Cargo.toml").exists() {
            Self::detect_cargo(project_root)
        } else if project_root.join("package.json").exists() {
            Self::detect_node(project_root)
        } else if project_root.join("pyproject.toml").exists() {
            Self::detect_poetry(project_root)
        } else if project_root.join("requirements.txt").exists()
            || project_root.join("setup.py").exists()
        {
            Self::detect_pip(project_root)
        } else if project_root.join("pom.xml").exists() {
            Self::detect_maven(project_root)
        } else if project_root.join("build.gradle").exists()
            || project_root.join("build.gradle.kts").exists()
        {
            Self::detect_gradle(project_root)
        } else if project_root.join("go.mod").exists() {
            Self::detect_go(project_root)
        } else if project_root.join("Makefile").exists() {
            Self::detect_make(project_root)
        } else {
            Ok(BuildConfig {
                system: BuildSystem::Unknown,
                root_path: project_root.to_path_buf(),
                config_files: vec![],
                commands: BuildCommands::default(),
                dependencies: vec![],
                dev_dependencies: vec![],
            })
        }
    }

    fn detect_cargo(project_root: &Path) -> Result<BuildConfig> {
        let cargo_toml_path = project_root.join("Cargo.toml");
        let content = fs::read_to_string(&cargo_toml_path).context("Failed to read Cargo.toml")?;

        let toml: toml::Value = toml::from_str(&content).context("Failed to parse Cargo.toml")?;

        let mut commands = BuildCommands::default();
        commands.build = Some("cargo build".to_string());
        commands.test = Some("cargo test".to_string());
        commands.lint = Some("cargo clippy".to_string());
        commands.format = Some("cargo fmt".to_string());
        commands.run = Some("cargo run".to_string());
        commands.clean = Some("cargo clean".to_string());

        // Extract custom commands from aliases
        if let Some(alias_section) = toml.get("alias") {
            if let Some(aliases) = alias_section.as_table() {
                for (name, value) in aliases {
                    if let Some(cmd) = value.as_str() {
                        commands
                            .custom
                            .insert(name.clone(), format!("cargo {}", cmd));
                    }
                }
            }
        }

        // Extract dependencies
        let mut dependencies = vec![];
        let mut dev_dependencies = vec![];

        if let Some(deps) = toml.get("dependencies") {
            if let Some(deps_table) = deps.as_table() {
                dependencies.extend(deps_table.keys().cloned());
            }
        }

        if let Some(dev_deps) = toml.get("dev-dependencies") {
            if let Some(dev_deps_table) = dev_deps.as_table() {
                dev_dependencies.extend(dev_deps_table.keys().cloned());
            }
        }

        Ok(BuildConfig {
            system: BuildSystem::Cargo,
            root_path: project_root.to_path_buf(),
            config_files: vec![cargo_toml_path],
            commands,
            dependencies,
            dev_dependencies,
        })
    }

    fn detect_node(project_root: &Path) -> Result<BuildConfig> {
        let package_json_path = project_root.join("package.json");
        let content =
            fs::read_to_string(&package_json_path).context("Failed to read package.json")?;

        let package_json: serde_json::Value =
            serde_json::from_str(&content).context("Failed to parse package.json")?;

        // Determine the package manager
        let system = if project_root.join("yarn.lock").exists() {
            BuildSystem::Yarn
        } else if project_root.join("pnpm-lock.yaml").exists() {
            BuildSystem::Pnpm
        } else {
            BuildSystem::Npm
        };

        let mut commands = BuildCommands::default();

        // Extract scripts
        if let Some(scripts) = package_json.get("scripts") {
            if let Some(scripts_obj) = scripts.as_object() {
                for (name, value) in scripts_obj {
                    if let Some(cmd) = value.as_str() {
                        match name.as_str() {
                            "build" => {
                                commands.build =
                                    Some(format!("{} run build", Self::node_cmd(&system)))
                            }
                            "test" => {
                                commands.test =
                                    Some(format!("{} run test", Self::node_cmd(&system)))
                            }
                            "lint" => {
                                commands.lint =
                                    Some(format!("{} run lint", Self::node_cmd(&system)))
                            }
                            "format" => {
                                commands.format =
                                    Some(format!("{} run format", Self::node_cmd(&system)))
                            }
                            "start" | "dev" => {
                                commands.run =
                                    Some(format!("{} run {}", Self::node_cmd(&system), name))
                            }
                            "clean" => {
                                commands.clean =
                                    Some(format!("{} run clean", Self::node_cmd(&system)))
                            }
                            _ => {
                                commands.custom.insert(
                                    name.clone(),
                                    format!("{} run {}", Self::node_cmd(&system), name),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Extract dependencies
        let mut dependencies = vec![];
        let mut dev_dependencies = vec![];

        if let Some(deps) = package_json.get("dependencies") {
            if let Some(deps_obj) = deps.as_object() {
                dependencies.extend(deps_obj.keys().cloned());
            }
        }

        if let Some(dev_deps) = package_json.get("devDependencies") {
            if let Some(dev_deps_obj) = dev_deps.as_object() {
                dev_dependencies.extend(dev_deps_obj.keys().cloned());
            }
        }

        let mut config_files = vec![package_json_path];
        if system == BuildSystem::Yarn && project_root.join("yarn.lock").exists() {
            config_files.push(project_root.join("yarn.lock"));
        } else if system == BuildSystem::Pnpm && project_root.join("pnpm-lock.yaml").exists() {
            config_files.push(project_root.join("pnpm-lock.yaml"));
        } else if project_root.join("package-lock.json").exists() {
            config_files.push(project_root.join("package-lock.json"));
        }

        Ok(BuildConfig {
            system,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies,
            dev_dependencies,
        })
    }

    fn detect_poetry(project_root: &Path) -> Result<BuildConfig> {
        let pyproject_path = project_root.join("pyproject.toml");
        let content =
            fs::read_to_string(&pyproject_path).context("Failed to read pyproject.toml")?;

        let toml: toml::Value =
            toml::from_str(&content).context("Failed to parse pyproject.toml")?;

        let mut commands = BuildCommands::default();
        commands.build = Some("poetry build".to_string());
        commands.test = Some("poetry run pytest".to_string());
        commands.lint = Some("poetry run flake8".to_string());
        commands.format = Some("poetry run black .".to_string());
        commands.run = Some("poetry run python".to_string());
        commands.clean = Some("poetry env remove python".to_string());

        // Extract scripts from [tool.poetry.scripts]
        if let Some(tool) = toml.get("tool") {
            if let Some(poetry) = tool.get("poetry") {
                if let Some(scripts) = poetry.get("scripts") {
                    if let Some(scripts_table) = scripts.as_table() {
                        for (name, _) in scripts_table {
                            commands
                                .custom
                                .insert(name.clone(), format!("poetry run {}", name));
                        }
                    }
                }
            }
        }

        // Extract dependencies
        let mut dependencies = vec![];
        let mut dev_dependencies = vec![];

        if let Some(tool) = toml.get("tool") {
            if let Some(poetry) = tool.get("poetry") {
                if let Some(deps) = poetry.get("dependencies") {
                    if let Some(deps_table) = deps.as_table() {
                        dependencies.extend(deps_table.keys().filter(|k| *k != "python").cloned());
                    }
                }

                if let Some(dev_deps) = poetry.get("dev-dependencies") {
                    if let Some(dev_deps_table) = dev_deps.as_table() {
                        dev_dependencies.extend(dev_deps_table.keys().cloned());
                    }
                }
            }
        }

        let mut config_files = vec![pyproject_path];
        if project_root.join("poetry.lock").exists() {
            config_files.push(project_root.join("poetry.lock"));
        }

        Ok(BuildConfig {
            system: BuildSystem::Poetry,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies,
            dev_dependencies,
        })
    }

    fn detect_pip(project_root: &Path) -> Result<BuildConfig> {
        let mut commands = BuildCommands::default();
        commands.test = Some("python -m pytest".to_string());
        commands.lint = Some("python -m flake8".to_string());
        commands.format = Some("python -m black .".to_string());
        commands.run = Some("python".to_string());

        let mut config_files = vec![];
        let mut dependencies = vec![];

        // Check for requirements.txt
        let requirements_path = project_root.join("requirements.txt");
        if requirements_path.exists() {
            config_files.push(requirements_path.clone());

            // Parse requirements.txt
            if let Ok(content) = fs::read_to_string(&requirements_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        // Extract package name (before any version specifier)
                        let package_name = line
                            .split(&['=', '>', '<', '!', '~', ';'][..])
                            .next()
                            .unwrap_or(line)
                            .trim();
                        dependencies.push(package_name.to_string());
                    }
                }
            }
        }

        // Check for setup.py
        if project_root.join("setup.py").exists() {
            config_files.push(project_root.join("setup.py"));
            commands.build = Some("python setup.py build".to_string());
        }

        Ok(BuildConfig {
            system: BuildSystem::Pip,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies,
            dev_dependencies: vec![],
        })
    }

    fn detect_maven(project_root: &Path) -> Result<BuildConfig> {
        let pom_path = project_root.join("pom.xml");

        let mut commands = BuildCommands::default();
        commands.build = Some("mvn compile".to_string());
        commands.test = Some("mvn test".to_string());
        commands.run = Some("mvn exec:java".to_string());
        commands.clean = Some("mvn clean".to_string());
        commands
            .custom
            .insert("package".to_string(), "mvn package".to_string());
        commands
            .custom
            .insert("install".to_string(), "mvn install".to_string());

        Ok(BuildConfig {
            system: BuildSystem::Maven,
            root_path: project_root.to_path_buf(),
            config_files: vec![pom_path],
            commands,
            dependencies: vec![], // TODO: Parse pom.xml for dependencies
            dev_dependencies: vec![],
        })
    }

    fn detect_gradle(project_root: &Path) -> Result<BuildConfig> {
        let mut config_files = vec![];
        if project_root.join("build.gradle").exists() {
            config_files.push(project_root.join("build.gradle"));
        }
        if project_root.join("build.gradle.kts").exists() {
            config_files.push(project_root.join("build.gradle.kts"));
        }

        let mut commands = BuildCommands::default();
        let gradle_cmd = if project_root.join("gradlew").exists() {
            "./gradlew"
        } else {
            "gradle"
        };

        commands.build = Some(format!("{} build", gradle_cmd));
        commands.test = Some(format!("{} test", gradle_cmd));
        commands.run = Some(format!("{} run", gradle_cmd));
        commands.clean = Some(format!("{} clean", gradle_cmd));

        Ok(BuildConfig {
            system: BuildSystem::Gradle,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies: vec![], // TODO: Parse build.gradle for dependencies
            dev_dependencies: vec![],
        })
    }

    fn detect_go(project_root: &Path) -> Result<BuildConfig> {
        let go_mod_path = project_root.join("go.mod");

        let mut commands = BuildCommands::default();
        commands.build = Some("go build".to_string());
        commands.test = Some("go test ./...".to_string());
        commands.lint = Some("go vet ./...".to_string());
        commands.format = Some("go fmt ./...".to_string());
        commands.run = Some("go run .".to_string());
        commands.clean = Some("go clean".to_string());

        let mut config_files = vec![go_mod_path];
        if project_root.join("go.sum").exists() {
            config_files.push(project_root.join("go.sum"));
        }

        Ok(BuildConfig {
            system: BuildSystem::Go,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies: vec![], // TODO: Parse go.mod for dependencies
            dev_dependencies: vec![],
        })
    }

    fn detect_make(project_root: &Path) -> Result<BuildConfig> {
        let makefile_path = project_root.join("Makefile");

        let mut commands = BuildCommands::default();

        // Try to parse common targets from Makefile
        if let Ok(content) = fs::read_to_string(&makefile_path) {
            for line in content.lines() {
                if let Some(target) = line.strip_suffix(':') {
                    let target = target.trim();
                    if !target.starts_with('.') && !target.contains(' ') {
                        match target {
                            "build" | "all" => commands.build = Some(format!("make {}", target)),
                            "test" | "check" => commands.test = Some(format!("make {}", target)),
                            "lint" => commands.lint = Some(format!("make {}", target)),
                            "format" | "fmt" => commands.format = Some(format!("make {}", target)),
                            "run" | "start" => commands.run = Some(format!("make {}", target)),
                            "clean" => commands.clean = Some(format!("make {}", target)),
                            _ => {
                                commands
                                    .custom
                                    .insert(target.to_string(), format!("make {}", target));
                            }
                        }
                    }
                }
            }
        }

        // Set defaults if not found
        commands.build.get_or_insert("make".to_string());
        commands.clean.get_or_insert("make clean".to_string());

        Ok(BuildConfig {
            system: BuildSystem::Make,
            root_path: project_root.to_path_buf(),
            config_files: vec![makefile_path],
            commands,
            dependencies: vec![],
            dev_dependencies: vec![],
        })
    }

    fn node_cmd(system: &BuildSystem) -> &'static str {
        match system {
            BuildSystem::Yarn => "yarn",
            BuildSystem::Pnpm => "pnpm",
            _ => "npm",
        }
    }
}

impl BuildConfig {
    pub fn get_command(&self, command_type: &str) -> Option<&str> {
        match command_type {
            "build" => self.commands.build.as_deref(),
            "test" => self.commands.test.as_deref(),
            "lint" => self.commands.lint.as_deref(),
            "format" => self.commands.format.as_deref(),
            "run" => self.commands.run.as_deref(),
            "clean" => self.commands.clean.as_deref(),
            custom => self.commands.custom.get(custom).map(|s| s.as_str()),
        }
    }

    pub fn all_commands(&self) -> Vec<(&str, &str)> {
        let mut commands = vec![];

        if let Some(cmd) = &self.commands.build {
            commands.push(("build", cmd.as_str()));
        }
        if let Some(cmd) = &self.commands.test {
            commands.push(("test", cmd.as_str()));
        }
        if let Some(cmd) = &self.commands.lint {
            commands.push(("lint", cmd.as_str()));
        }
        if let Some(cmd) = &self.commands.format {
            commands.push(("format", cmd.as_str()));
        }
        if let Some(cmd) = &self.commands.run {
            commands.push(("run", cmd.as_str()));
        }
        if let Some(cmd) = &self.commands.clean {
            commands.push(("clean", cmd.as_str()));
        }

        for (name, cmd) in &self.commands.custom {
            commands.push((name.as_str(), cmd.as_str()));
        }

        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_cargo() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = "1.0"

[dev-dependencies]
mockito = "0.31"

[alias]
ci = "check --all-features"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

        let config = BuildSystemDetector::detect(temp_dir.path()).unwrap();
        assert_eq!(config.system, BuildSystem::Cargo);
        assert_eq!(config.commands.build, Some("cargo build".to_string()));
        assert!(config.dependencies.contains(&"serde".to_string()));
        assert!(config.dev_dependencies.contains(&"mockito".to_string()));
        assert_eq!(
            config.commands.custom.get("ci"),
            Some(&"cargo check --all-features".to_string())
        );
    }

    #[test]
    fn test_detect_npm() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = r#"{
    "name": "test-project",
    "version": "1.0.0",
    "scripts": {
        "build": "webpack",
        "test": "jest",
        "dev": "nodemon server.js",
        "custom-task": "echo custom"
    },
    "dependencies": {
        "express": "^4.17.1",
        "lodash": "^4.17.21"
    },
    "devDependencies": {
        "jest": "^27.0.0"
    }
}"#;
        fs::write(temp_dir.path().join("package.json"), package_json).unwrap();

        let config = BuildSystemDetector::detect(temp_dir.path()).unwrap();
        assert_eq!(config.system, BuildSystem::Npm);
        assert_eq!(config.commands.build, Some("npm run build".to_string()));
        assert_eq!(config.commands.run, Some("npm run dev".to_string()));
        assert!(config.dependencies.contains(&"express".to_string()));
        assert!(config.dev_dependencies.contains(&"jest".to_string()));
        assert_eq!(
            config.commands.custom.get("custom-task"),
            Some(&"npm run custom-task".to_string())
        );
    }

    #[test]
    fn test_detect_yarn() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("package.json"), r#"{"name": "test"}"#).unwrap();
        fs::write(temp_dir.path().join("yarn.lock"), "").unwrap();

        let config = BuildSystemDetector::detect(temp_dir.path()).unwrap();
        assert_eq!(config.system, BuildSystem::Yarn);
    }

    #[test]
    fn test_detect_poetry() {
        let temp_dir = TempDir::new().unwrap();
        let pyproject = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.9"
requests = "^2.26.0"

[tool.poetry.dev-dependencies]
pytest = "^6.2.5"

[tool.poetry.scripts]
serve = "myapp:serve"
"#;
        fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

        let config = BuildSystemDetector::detect(temp_dir.path()).unwrap();
        assert_eq!(config.system, BuildSystem::Poetry);
        assert_eq!(config.commands.build, Some("poetry build".to_string()));
        assert!(config.dependencies.contains(&"requests".to_string()));
        assert!(config.dev_dependencies.contains(&"pytest".to_string()));
        assert_eq!(
            config.commands.custom.get("serve"),
            Some(&"poetry run serve".to_string())
        );
    }

    #[test]
    fn test_unknown_build_system() {
        let temp_dir = TempDir::new().unwrap();
        let config = BuildSystemDetector::detect(temp_dir.path()).unwrap();
        assert_eq!(config.system, BuildSystem::Unknown);
        assert!(config.commands.build.is_none());
    }
}
