use anyhow::{Context, Result};
use std::path::Path;
use std::fs;

use crate::project::build_system::types::{BuildCommands, BuildConfig, BuildSystem};
use super::{BuildSystemDetector, utils};

pub struct MavenDetector;

impl BuildSystemDetector for MavenDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Maven
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "pom.xml")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let pom_path = utils::get_file_path(project_root, "pom.xml");
        let content = utils::read_file(&pom_path)?;

        let mut commands = BuildCommands::default();
        
        // Determine if using Maven wrapper
        let mvn_cmd = if utils::has_file(project_root, "mvnw") {
            "./mvnw"
        } else {
            "mvn"
        };

        commands.build = Some(format!("{mvn_cmd} compile"));
        commands.test = Some(format!("{mvn_cmd} test"));
        commands.run = Some(format!("{mvn_cmd} exec:java"));
        commands.clean = Some(format!("{mvn_cmd} clean"));
        commands.custom.insert("package".to_string(), format!("{mvn_cmd} package"));
        commands.custom.insert("install".to_string(), format!("{mvn_cmd} install"));
        commands.custom.insert("verify".to_string(), format!("{mvn_cmd} verify"));
        commands.custom.insert("dependency-tree".to_string(), format!("{mvn_cmd} dependency:tree"));

        // Check for common Maven plugins in pom.xml
        if content.contains("maven-checkstyle-plugin") || content.contains("spotbugs-maven-plugin") {
            commands.lint = Some(format!("{mvn_cmd} verify"));
        }

        if content.contains("fmt-maven-plugin") || content.contains("formatter-maven-plugin") {
            commands.format = Some(format!("{mvn_cmd} fmt:format"));
        } else if content.contains("spotless-maven-plugin") {
            commands.format = Some(format!("{mvn_cmd} spotless:apply"));
        }

        // Extract dependencies using XML parser
        let (dependencies, dev_dependencies) = parse_maven_dependencies(&pom_path)?;

        let mut config_files = vec![pom_path];
        
        // Check for Maven wrapper
        if utils::has_file(project_root, "mvnw") {
            config_files.push(utils::get_file_path(project_root, "mvnw"));
        }
        if utils::has_file(project_root, ".mvn/wrapper/maven-wrapper.properties") {
            config_files.push(utils::get_file_path(project_root, ".mvn/wrapper/maven-wrapper.properties"));
        }
        
        // Check for settings
        if utils::has_file(project_root, ".mvn/settings.xml") {
            config_files.push(utils::get_file_path(project_root, ".mvn/settings.xml"));
        }

        Ok(BuildConfig {
            system: BuildSystem::Maven,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies,
            dev_dependencies,
        })
    }
}

pub struct GradleDetector;

impl BuildSystemDetector for GradleDetector {
    fn build_system(&self) -> BuildSystem {
        BuildSystem::Gradle
    }

    fn can_detect(&self, project_root: &Path) -> bool {
        utils::has_file(project_root, "build.gradle") ||
        utils::has_file(project_root, "build.gradle.kts")
    }

    fn detect(&self, project_root: &Path) -> Result<BuildConfig> {
        let mut commands = BuildCommands::default();
        let mut config_files = vec![];

        // Determine build file type
        let is_kotlin_dsl = utils::has_file(project_root, "build.gradle.kts");
        if is_kotlin_dsl {
            config_files.push(utils::get_file_path(project_root, "build.gradle.kts"));
        } else {
            config_files.push(utils::get_file_path(project_root, "build.gradle"));
        }

        // Determine if using Gradle wrapper
        let gradle_cmd = if utils::has_file(project_root, "gradlew") {
            "./gradlew"
        } else {
            "gradle"
        };

        commands.build = Some(format!("{gradle_cmd} build"));
        commands.test = Some(format!("{gradle_cmd} test"));
        commands.run = Some(format!("{gradle_cmd} run"));
        commands.clean = Some(format!("{gradle_cmd} clean"));
        commands.custom.insert("assemble".to_string(), format!("{gradle_cmd} assemble"));
        commands.custom.insert("check".to_string(), format!("{gradle_cmd} check"));
        commands.custom.insert("dependencies".to_string(), format!("{gradle_cmd} dependencies"));
        commands.custom.insert("tasks".to_string(), format!("{gradle_cmd} tasks"));

        // Read build file to check for plugins
        let build_file_path = if is_kotlin_dsl {
            utils::get_file_path(project_root, "build.gradle.kts")
        } else {
            utils::get_file_path(project_root, "build.gradle")
        };
        
        if let Ok(content) = utils::read_file(&build_file_path) {
            // Check for common plugins
            if content.contains("checkstyle") || content.contains("spotbugs") || content.contains("pmd") {
                commands.lint = Some(format!("{gradle_cmd} check"));
            }

            if content.contains("spotless") {
                commands.format = Some(format!("{gradle_cmd} spotlessApply"));
                commands.custom.insert("format-check".to_string(), format!("{gradle_cmd} spotlessCheck"));
            }

            // Extract dependencies (simplified)
            // Real implementation would need to parse Groovy/Kotlin
        }

        // Check for settings file
        if utils::has_file(project_root, "settings.gradle") {
            config_files.push(utils::get_file_path(project_root, "settings.gradle"));
        } else if utils::has_file(project_root, "settings.gradle.kts") {
            config_files.push(utils::get_file_path(project_root, "settings.gradle.kts"));
        }

        // Check for gradle.properties
        if utils::has_file(project_root, "gradle.properties") {
            config_files.push(utils::get_file_path(project_root, "gradle.properties"));
        }

        // Check for Gradle wrapper
        if utils::has_file(project_root, "gradlew") {
            config_files.push(utils::get_file_path(project_root, "gradlew"));
        }
        if utils::has_file(project_root, "gradle/wrapper/gradle-wrapper.properties") {
            config_files.push(utils::get_file_path(project_root, "gradle/wrapper/gradle-wrapper.properties"));
        }

        // For multi-module projects
        if utils::has_file(project_root, "buildSrc/build.gradle") {
            config_files.push(utils::get_file_path(project_root, "buildSrc/build.gradle"));
        } else if utils::has_file(project_root, "buildSrc/build.gradle.kts") {
            config_files.push(utils::get_file_path(project_root, "buildSrc/build.gradle.kts"));
        }

        Ok(BuildConfig {
            system: BuildSystem::Gradle,
            root_path: project_root.to_path_buf(),
            config_files,
            commands,
            dependencies: vec![], // Would need proper parsing
            dev_dependencies: vec![], // Would need proper parsing
        })
    }
}

/// Parse Maven dependencies from pom.xml
fn parse_maven_dependencies(pom_path: &Path) -> Result<(Vec<String>, Vec<String>)> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();
    
    let pom_content = fs::read_to_string(pom_path)
        .context("Failed to read pom.xml")?;
    
    let mut reader = Reader::from_str(&pom_content);
    reader.trim_text(true);
    
    let mut buf = Vec::new();
    let mut in_dependencies = false;
    let mut in_dependency = false;
    let mut in_test_scope = false;
    let mut current_group_id = String::new();
    let mut current_artifact_id = String::new();
    let mut current_element = String::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                let name_str = std::str::from_utf8(name.as_ref())
                    .unwrap_or_default()
                    .to_string();
                current_element = name_str.clone();
                
                match name_str.as_str() {
                    "dependencies" => in_dependencies = true,
                    "dependency" if in_dependencies => {
                        in_dependency = true;
                        in_test_scope = false;
                        current_group_id.clear();
                        current_artifact_id.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                let name_str = std::str::from_utf8(name.as_ref())
                    .unwrap_or_default();
                
                match name_str {
                    "dependencies" => in_dependencies = false,
                    "dependency" if in_dependencies => {
                        in_dependency = false;
                        if !current_group_id.is_empty() && !current_artifact_id.is_empty() {
                            let dep = format!("{current_group_id}:{current_artifact_id}");
                            if in_test_scope {
                                dev_dependencies.push(dep);
                            } else {
                                dependencies.push(dep);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if in_dependency => {
                let text = e.unescape().unwrap_or_default().to_string();
                match current_element.as_str() {
                    "groupId" => current_group_id = text,
                    "artifactId" => current_artifact_id = text,
                    "scope" => {
                        if text == "test" {
                            in_test_scope = true;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("Error parsing pom.xml: {}", e)),
            _ => {}
        }
        buf.clear();
    }
    
    Ok((dependencies, dev_dependencies))
}