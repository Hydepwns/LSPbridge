use anyhow::{Context, Result};
use std::path::Path;

use crate::project::build_system::types::{BuildCommands, BuildConfig, BuildSystem};
use super::{BuildSystemDetector, utils};

pub struct PoetryDetector;

impl BuildSystemDetector for PoetryDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Poetry
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "pyproject.toml") && 
        utils::has_file(project_root, "poetry.lock")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let pyproject_path = utils::get_file_path(project_root, "pyproject.toml");
        let content = utils::read_file(&pyproject_path)?;
        let pyproject: toml::Value = toml::from_str(&content)
            .context("Failed to parse pyproject.toml")?;

        let mut commands = BuildCommands::default();
        commands.build = Some("poetry build".to_string());
        commands.test = Some("poetry run pytest".to_string());
        commands.run = Some("poetry run python".to_string());
        commands.custom.insert("install".to_string(), "poetry install".to_string());
        commands.custom.insert("shell".to_string(), "poetry shell".to_string());
        commands.custom.insert("update".to_string(), "poetry update".to_string());

        // Check for common Python tools in dev dependencies
        if let Some(dev_deps) = pyproject
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("dev-dependencies"))
            .and_then(|d| d.as_table()) 
        {
            if dev_deps.contains_key("black") {
                commands.format = Some("poetry run black .".to_string());
            } else if dev_deps.contains_key("autopep8") {
                commands.format = Some("poetry run autopep8 --in-place --recursive .".to_string());
            }

            if dev_deps.contains_key("flake8") {
                commands.lint = Some("poetry run flake8".to_string());
            } else if dev_deps.contains_key("pylint") {
                commands.lint = Some("poetry run pylint".to_string());
            } else if dev_deps.contains_key("ruff") {
                commands.lint = Some("poetry run ruff check".to_string());
                if commands.format.is_none() {
                    commands.format = Some("poetry run ruff format".to_string());
                }
            }

            if dev_deps.contains_key("mypy") {
                commands.custom.insert("typecheck".to_string(), "poetry run mypy .".to_string());
            }
        }

        // Extract dependencies
        let mut dependencies = vec![];
        let mut dev_dependencies = vec![];

        if let Some(deps) = pyproject
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("dependencies"))
            .and_then(|d| d.as_table())
        {
            for (name, _) in deps {
                if name != "python" { // Skip Python version constraint
                    dependencies.push(name.clone());
                }
            }
        }

        if let Some(deps) = pyproject
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("dev-dependencies"))
            .and_then(|d| d.as_table())
        {
            for (name, _) in deps {
                dev_dependencies.push(name.clone());
            }
        }

        let mut config_files = vec![pyproject_path];
        if utils::has_file(project_root, "poetry.lock") {
            config_files.push(utils::get_file_path(project_root, "poetry.lock"));
        }
        if utils::has_file(project_root, "poetry.toml") {
            config_files.push(utils::get_file_path(project_root, "poetry.toml"));
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
}

pub struct PipDetector;

impl BuildSystemDetector for PipDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Pip
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "requirements.txt") ||
        utils::has_file(project_root, "setup.py") ||
        utils::has_file(project_root, "setup.cfg") ||
        (utils::has_file(project_root, "pyproject.toml") && 
         !utils::has_file(project_root, "poetry.lock"))
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let mut commands = BuildCommands::default();
        let mut config_files = vec![];

        // Determine the package management approach
        let has_setup_py = utils::has_file(project_root, "setup.py");
        let has_pyproject = utils::has_file(project_root, "pyproject.toml");
        let has_requirements = utils::has_file(project_root, "requirements.txt");

        if has_setup_py {
            config_files.push(utils::get_file_path(project_root, "setup.py"));
            commands.build = Some("python setup.py build".to_string());
            commands.custom.insert("install".to_string(), "pip install -e .".to_string());
            commands.custom.insert("dist".to_string(), "python setup.py sdist bdist_wheel".to_string());
        }

        if has_pyproject && !has_setup_py {
            config_files.push(utils::get_file_path(project_root, "pyproject.toml"));
            commands.build = Some("python -m build".to_string());
            commands.custom.insert("install".to_string(), "pip install -e .".to_string());
        }

        if has_requirements {
            config_files.push(utils::get_file_path(project_root, "requirements.txt"));
            if commands.custom.get("install").is_none() {
                commands.custom.insert("install".to_string(), "pip install -r requirements.txt".to_string());
            }
        }

        if utils::has_file(project_root, "requirements-dev.txt") {
            config_files.push(utils::get_file_path(project_root, "requirements-dev.txt"));
            commands.custom.insert("install-dev".to_string(), "pip install -r requirements-dev.txt".to_string());
        }

        if utils::has_file(project_root, "setup.cfg") {
            config_files.push(utils::get_file_path(project_root, "setup.cfg"));
        }

        // Common Python commands
        commands.test = Some("python -m pytest".to_string());
        commands.run = Some("python".to_string());
        
        // Check for common tools in the project
        if utils::has_file(project_root, ".flake8") {
            commands.lint = Some("flake8".to_string());
            config_files.push(utils::get_file_path(project_root, ".flake8"));
        } else if utils::has_file(project_root, ".pylintrc") {
            commands.lint = Some("pylint".to_string());
            config_files.push(utils::get_file_path(project_root, ".pylintrc"));
        } else if utils::has_file(project_root, "ruff.toml") || utils::has_file(project_root, ".ruff.toml") {
            commands.lint = Some("ruff check".to_string());
            commands.format = Some("ruff format".to_string());
            if utils::has_file(project_root, "ruff.toml") {
                config_files.push(utils::get_file_path(project_root, "ruff.toml"));
            } else {
                config_files.push(utils::get_file_path(project_root, ".ruff.toml"));
            }
        }

        if utils::has_file(project_root, ".black") || utils::has_file(project_root, "pyproject.toml") {
            commands.format = commands.format.or(Some("black .".to_string()));
        }

        if utils::has_file(project_root, "mypy.ini") || utils::has_file(project_root, ".mypy.ini") {
            commands.custom.insert("typecheck".to_string(), "mypy .".to_string());
            if utils::has_file(project_root, "mypy.ini") {
                config_files.push(utils::get_file_path(project_root, "mypy.ini"));
            } else {
                config_files.push(utils::get_file_path(project_root, ".mypy.ini"));
            }
        }

        // Look for other common Python config files
        if utils::has_file(project_root, "tox.ini") {
            config_files.push(utils::get_file_path(project_root, "tox.ini"));
            commands.custom.insert("tox".to_string(), "tox".to_string());
        }

        if utils::has_file(project_root, "Makefile") {
            // Check if it has Python-specific targets
            if let Ok(content) = utils::read_file(&utils::get_file_path(project_root, "Makefile")) {
                if content.contains("test:") || content.contains("lint:") || content.contains("install:") {
                    config_files.push(utils::get_file_path(project_root, "Makefile"));
                }
            }
        }

        // Extract dependencies from requirements.txt
        let mut dependencies = vec![];
        let mut dev_dependencies = vec![];

        if has_requirements {
            if let Ok(content) = utils::read_file(&utils::get_file_path(project_root, "requirements.txt")) {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') && !line.starts_with('-') {
                        if let Some(pkg_name) = line.split(&['=', '>', '<', '!', '~', ';'][..]).next() {
                            dependencies.push(pkg_name.trim().to_string());
                        }
                    }
                }
            }
        }

        if utils::has_file(project_root, "requirements-dev.txt") {
            if let Ok(content) = utils::read_file(&utils::get_file_path(project_root, "requirements-dev.txt")) {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') && !line.starts_with('-') {
                        if let Some(pkg_name) = line.split(&['=', '>', '<', '!', '~', ';'][..]).next() {
                            dev_dependencies.push(pkg_name.trim().to_string());
                        }
                    }
                }
            }
        }

        Ok(BuildConfig {
            system: BuildSystem::Pip,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies,
            dev_dependencies,
        })
    }
}