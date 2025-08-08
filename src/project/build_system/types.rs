use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Default)]
pub struct BuildCommands {
    pub build: Option<String>,
    pub test: Option<String>,
    pub lint: Option<String>,
    pub format: Option<String>,
    pub run: Option<String>,
    pub clean: Option<String>,
    pub custom: HashMap<String, String>,
}


impl BuildConfig {
    /// Get standard command by name
    pub fn get_command(&self, command: &str) -> Option<&str> {
        match command {
            "build" => self.commands.build.as_deref(),
            "test" => self.commands.test.as_deref(),
            "lint" => self.commands.lint.as_deref(),
            "format" => self.commands.format.as_deref(),
            "run" => self.commands.run.as_deref(),
            "clean" => self.commands.clean.as_deref(),
            custom => self.commands.custom.get(custom).map(|s| s.as_str()),
        }
    }

    /// Get all available commands
    pub fn list_commands(&self) -> Vec<String> {
        let mut commands = vec![];

        if self.commands.build.is_some() {
            commands.push("build".to_string());
        }
        if self.commands.test.is_some() {
            commands.push("test".to_string());
        }
        if self.commands.lint.is_some() {
            commands.push("lint".to_string());
        }
        if self.commands.format.is_some() {
            commands.push("format".to_string());
        }
        if self.commands.run.is_some() {
            commands.push("run".to_string());
        }
        if self.commands.clean.is_some() {
            commands.push("clean".to_string());
        }

        for custom_cmd in self.commands.custom.keys() {
            commands.push(custom_cmd.clone());
        }

        commands
    }

    /// Check if the build system supports a specific feature
    pub fn supports_feature(&self, feature: &str) -> bool {
        match (self.system, feature) {
            (BuildSystem::Cargo, "workspace") => self
                .config_files
                .iter()
                .any(|f| f.file_name() == Some("Cargo.toml".as_ref())),
            (BuildSystem::Npm | BuildSystem::Yarn | BuildSystem::Pnpm, "workspace") => self
                .config_files
                .iter()
                .any(|f| f.file_name() == Some("package.json".as_ref())),
            (BuildSystem::Poetry, "virtualenv") => true,
            _ => false,
        }
    }

    /// Get all available commands
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