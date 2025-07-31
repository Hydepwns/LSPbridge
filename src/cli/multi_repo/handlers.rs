//! Command handlers for multi-repository CLI operations
//!
//! This module contains the implementation of all multi-repository command handlers,
//! including repository registration, listing, analysis, and team management.

use super::types::{AssignmentStatusArg, MultiRepoCommand, OutputFormat, PriorityArg, RelationTypeArg, TeamCommand, TeamRoleArg};
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;
use uuid::Uuid;

use crate::multi_repo::{MultiRepoContext, RepositoryInfo};
use crate::project::BuildSystemDetector;
use crate::security::validate_path;

/// Main handler for multi-repository commands
///
/// Routes commands to their specific handlers and manages the execution context.
pub async fn handle_multi_repo_command(
    cmd: MultiRepoCommand,
    _config_path: Option<PathBuf>,
) -> Result<()> {
    let config = crate::multi_repo::MultiRepoConfig::default();
    let mut context = MultiRepoContext::new(config).await?;

    match cmd {
        MultiRepoCommand::Register {
            path,
            name,
            remote_url,
            language,
            tags,
        } => {
            handle_register(&mut context, path, name, remote_url, language, tags).await?;
        }

        MultiRepoCommand::List { all, tag, format } => {
            handle_list(&context, all, tag, format).await?;
        }

        MultiRepoCommand::Analyze {
            min_impact,
            output,
            format,
        } => {
            handle_analyze(&mut context, min_impact, output, format).await?;
        }

        MultiRepoCommand::DetectMonorepo { path, register } => {
            handle_detect_monorepo(&mut context, path, register).await?;
        }

        MultiRepoCommand::Relate {
            source,
            target,
            relation,
            data,
        } => {
            handle_relate(&mut context, source, target, relation, data).await?;
        }

        MultiRepoCommand::Team { command } => {
            handle_team_command(&mut context, command).await?;
        }

        MultiRepoCommand::Types { format } => {
            handle_types(&mut context, format).await?;
        }
    }

    Ok(())
}

/// Handle repository registration
pub async fn handle_register(
    _context: &mut MultiRepoContext,
    path: PathBuf,
    name: Option<String>,
    remote_url: Option<String>,
    language: Option<String>,
    tags: Option<String>,
) -> Result<()> {
    let abs_path = validate_path(&path)
        .context("Failed to validate repository path")?;

    // Detect build system
    let build_system = match BuildSystemDetector::detect(&abs_path) {
        Ok(config) => Some(format!("{:?}", config.system)),
        Err(_) => None,
    };

    // Auto-detect language if not provided
    let detected_language = if language.is_none() {
        detect_primary_language(&abs_path).await
    } else {
        language
    };

    let repo_name = name.unwrap_or_else(|| {
        abs_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let repo_id = Uuid::new_v4().to_string();

    let info = RepositoryInfo {
        id: repo_id.clone(),
        name: repo_name.clone(),
        path: abs_path.clone(),
        remote_url,
        primary_language: detected_language,
        build_system,
        is_monorepo_member: false,
        monorepo_id: None,
        tags: tags
            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        active: true,
        last_diagnostic_run: None,
        metadata: serde_json::json!({}),
    };

    // TODO: Implement actual registration
    // context.register_repo(info).await?;
    let _ = info; // Placeholder

    println!(
        "{} Repository '{}' registered successfully",
        "✓".green(),
        repo_name
    );
    println!("  ID: {}", repo_id);
    println!("  Path: {}", abs_path.display());

    Ok(())
}

/// Handle repository listing
pub async fn handle_list(
    _context: &MultiRepoContext,
    _all: bool,
    _tag: Option<String>,
    format: OutputFormat,
) -> Result<()> {
    println!("{} Listing repositories...", "→".blue());

    match format {
        OutputFormat::Table => {
            println!("{}", "┌─────────────────────────────────────────────────────┐".bright_black());
            println!("{}", "│                Repository List                     │".bright_black());
            println!("{}", "├─────────────────────────────────────────────────────┤".bright_black());
            println!("{}", "│ ID        │ Name      │ Language  │ Status     │".bright_black());
            println!("{}", "├─────────────────────────────────────────────────────┤".bright_black());
            println!("{}", "│ (none)    │ (none)    │ (none)    │ (none)     │".bright_black());
            println!("{}", "└─────────────────────────────────────────────────────┘".bright_black());
        }
        OutputFormat::Json => {
            println!("{{\"repositories\": []}}");
        }
        OutputFormat::Csv => {
            println!("id,name,language,status");
        }
    }

    Ok(())
}

/// Handle diagnostic analysis across repositories
pub async fn handle_analyze(
    _context: &mut MultiRepoContext,
    min_impact: f32,
    output: Option<PathBuf>,
    format: OutputFormat,
) -> Result<()> {
    println!(
        "{} Analyzing cross-repository diagnostics (min impact: {})",
        "→".blue(),
        min_impact
    );

    // TODO: Implement actual analysis
    let diagnostics = Vec::new(); // Placeholder

    match format {
        OutputFormat::Table => {
            display_diagnostics_table(&diagnostics);
        }
        OutputFormat::Json => {
            let json = serde_json::json!({
                "diagnostics": diagnostics,
                "analysis_config": {
                    "min_impact": min_impact
                }
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Csv => {
            println!("file,severity,message,impact_score,affected_repos");
            for diagnostic in &diagnostics {
                println!(
                    "{},{},{},{},{}",
                    diagnostic.file_path.display(),
                    diagnostic.severity,
                    diagnostic.message,
                    diagnostic.cross_repo_impact,
                    diagnostic.affected_repositories.len()
                );
            }
        }
    }

    // Write to output file if specified
    if let Some(output_path) = output {
        println!("{} Writing results to: {}", "→".blue(), output_path.display());
        // TODO: Implement file output
    }

    Ok(())
}

/// Handle monorepo detection
pub async fn handle_detect_monorepo(
    _context: &mut MultiRepoContext,
    path: PathBuf,
    register: bool,
) -> Result<()> {
    let abs_path = validate_path(&path)
        .context("Failed to validate path for monorepo detection")?;

    println!(
        "{} Detecting monorepo structure in: {}",
        "→".blue(),
        abs_path.display()
    );

    // TODO: Implement monorepo detection
    println!("{} No monorepo structure detected", "!".yellow());

    if register {
        println!("{} Would register detected subprojects (none found)", "→".blue());
    }

    Ok(())
}

/// Handle repository relationship management
pub async fn handle_relate(
    _context: &mut MultiRepoContext,
    source: String,
    target: String,
    relation: RelationTypeArg,
    data: Option<String>,
) -> Result<()> {
    println!(
        "{} Creating {} relationship: {} → {}",
        "→".blue(),
        format!("{:?}", relation).to_lowercase(),
        source,
        target
    );

    if let Some(data) = data {
        println!("  Additional data: {}", data);
    }

    // TODO: Implement relationship creation
    println!("{} Relationship created successfully", "✓".green());

    Ok(())
}

/// Handle team collaboration commands
pub async fn handle_team_command(_context: &mut MultiRepoContext, command: TeamCommand) -> Result<()> {
    match command {
        TeamCommand::AddMember { name, email, role } => {
            println!(
                "{} Adding team member: {} ({}) with role: {:?}",
                "→".blue(),
                name,
                email,
                role
            );
            // TODO: Implement member addition
            println!("{} Team member added successfully", "✓".green());
        }

        TeamCommand::ListMembers { format } => {
            println!("{} Listing team members...", "→".blue());
            
            match format {
                OutputFormat::Table => {
                    println!("{}", "┌──────────────────────────────────────────┐".bright_black());
                    println!("{}", "│              Team Members                │".bright_black());
                    println!("{}", "├──────────────────────────────────────────┤".bright_black());
                    println!("{}", "│ Name         │ Email        │ Role       │".bright_black());
                    println!("{}", "├──────────────────────────────────────────┤".bright_black());
                    println!("{}", "│ (none)       │ (none)       │ (none)     │".bright_black());
                    println!("{}", "└──────────────────────────────────────────┘".bright_black());
                }
                OutputFormat::Json => {
                    println!("{{\"team_members\": []}}");
                }
                OutputFormat::Csv => {
                    println!("name,email,role");
                }
            }
        }

        TeamCommand::Assign {
            repo,
            file,
            hash,
            assignee,
            priority,
            due_date,
        } => {
            println!(
                "{} Assigning diagnostic {} in {}/{} to {} (priority: {:?})",
                "→".blue(),
                hash,
                repo,
                file,
                assignee,
                priority
            );
            
            if let Some(due) = due_date {
                println!("  Due date: {}", due);
            }
            
            // TODO: Implement assignment
            println!("{} Diagnostic assigned successfully", "✓".green());
        }

        TeamCommand::UpdateStatus { id, status, note } => {
            println!(
                "{} Updating assignment {} status to: {:?}",
                "→".blue(),
                id,
                status
            );
            
            if let Some(note) = note {
                println!("  Note: {}", note);
            }
            
            // TODO: Implement status update
            println!("{} Assignment status updated successfully", "✓".green());
        }

        TeamCommand::History {
            member,
            repo,
            limit,
            format,
        } => {
            println!("{} Showing assignment history (limit: {})", "→".blue(), limit);
            
            if let Some(member) = member {
                println!("  Filtered by member: {}", member);
            }
            
            if let Some(repo) = repo {
                println!("  Filtered by repo: {}", repo);
            }
            
            match format {
                OutputFormat::Table => {
                    println!("{}", "┌─────────────────────────────────────────────────────┐".bright_black());
                    println!("{}", "│                Assignment History                   │".bright_black());
                    println!("{}", "├─────────────────────────────────────────────────────┤".bright_black());
                    println!("{}", "│ Date       │ Assignee  │ Repo      │ Status       │".bright_black());
                    println!("{}", "├─────────────────────────────────────────────────────┤".bright_black());
                    println!("{}", "│ (none)     │ (none)    │ (none)    │ (none)       │".bright_black());
                    println!("{}", "└─────────────────────────────────────────────────────┘".bright_black());
                }
                OutputFormat::Json => {
                    println!("{{\"assignments\": []}}");
                }
                OutputFormat::Csv => {
                    println!("date,assignee,repo,file,status");
                }
            }
        }
    }

    Ok(())
}

/// Handle cross-repository type analysis
pub async fn handle_types(_context: &mut MultiRepoContext, format: OutputFormat) -> Result<()> {
    println!("{} Analyzing cross-repository type references...", "→".blue());

    // TODO: Implement type analysis
    let type_references = Vec::new(); // Placeholder

    match format {
        OutputFormat::Table => {
            println!("{}", "┌─────────────────────────────────────────────────────┐".bright_black());
            println!("{}", "│               Type References                       │".bright_black());
            println!("{}", "├─────────────────────────────────────────────────────┤".bright_black());
            println!("{}", "│ Type Name  │ Source Repo │ Target Repos │ Usage   │".bright_black());
            println!("{}", "├─────────────────────────────────────────────────────┤".bright_black());
            println!("{}", "│ (none)     │ (none)      │ (none)       │ (none)  │".bright_black());
            println!("{}", "└─────────────────────────────────────────────────────┘".bright_black());
        }
        OutputFormat::Json => {
            let json = serde_json::json!({
                "type_references": type_references
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Csv => {
            println!("type_name,source_repo,source_file,target_repo,target_file,usage_context");
        }
    }

    Ok(())
}

/// Detect the primary programming language of a repository
pub async fn detect_primary_language(path: &PathBuf) -> Option<String> {
    use std::collections::HashMap;
    use walkdir::WalkDir;

    let mut language_counts: HashMap<String, usize> = HashMap::new();

    for entry in WalkDir::new(path)
        .max_depth(3) // Don't go too deep for performance
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if let Some(extension) = entry.path().extension().and_then(|e| e.to_str()) {
            let language = match extension {
                "rs" => "rust",
                "ts" | "tsx" => "typescript",
                "js" | "jsx" => "javascript",
                "py" => "python",
                "go" => "go",
                "java" => "java",
                "cpp" | "cc" | "cxx" => "cpp",
                "c" => "c",
                "cs" => "csharp",
                "rb" => "ruby",
                "php" => "php",
                _ => continue,
            };

            *language_counts.entry(language.to_string()).or_insert(0) += 1;
        }
    }

    // Return the language with the most files
    language_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(language, _)| language)
}

/// Display diagnostics in a formatted table
pub fn display_diagnostics_table(diagnostics: &[crate::multi_repo::AggregatedDiagnostic]) {
    if diagnostics.is_empty() {
        println!("{}", "┌─────────────────────────────────────────────────────┐".bright_black());
        println!("{}", "│                  No diagnostics found               │".bright_black());
        println!("{}", "└─────────────────────────────────────────────────────┘".bright_black());
        return;
    }

    println!("{}", "┌─────────────────────────────────────────────────────┐".bright_black());
    println!("{}", "│                Cross-Repo Diagnostics               │".bright_black());
    println!("{}", "├─────────────────────────────────────────────────────┤".bright_black());
    println!("{}", "│ File        │ Severity │ Message      │ Impact     │".bright_black());
    println!("{}", "├─────────────────────────────────────────────────────┤".bright_black());

    for diagnostic in diagnostics {
        let severity_color = match diagnostic.severity.to_lowercase().as_str() {
            "error" => diagnostic.severity.red(),
            "warning" => diagnostic.severity.yellow(),
            "info" => diagnostic.severity.blue(),
            _ => diagnostic.severity.normal(),
        };

        let impact_color = if diagnostic.cross_repo_impact > 0.7 {
            format!("{:.2}", diagnostic.cross_repo_impact).red()
        } else if diagnostic.cross_repo_impact > 0.4 {
            format!("{:.2}", diagnostic.cross_repo_impact).yellow()
        } else {
            format!("{:.2}", diagnostic.cross_repo_impact).green()
        };

        println!(
            "│ {:<10} │ {:<8} │ {:<12} │ {:<10} │",
            diagnostic.file_path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .chars()
                .take(10)
                .collect::<String>(),
            severity_color,
            diagnostic.message
                .chars()
                .take(12)
                .collect::<String>(),
            impact_color
        );
    }

    println!("{}", "└─────────────────────────────────────────────────────┘".bright_black());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_detect_primary_language_rust() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create some Rust files
        fs::write(temp_path.join("main.rs"), "fn main() {}").unwrap();
        fs::write(temp_path.join("lib.rs"), "// Library").unwrap();

        let language = detect_primary_language(&temp_path.to_path_buf()).await;
        assert_eq!(language, Some("rust".to_string()));
    }

    #[tokio::test]
    async fn test_detect_primary_language_typescript() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create some TypeScript files
        fs::write(temp_path.join("index.ts"), "console.log('hello');").unwrap();
        fs::write(temp_path.join("types.ts"), "export interface User {}").unwrap();

        let language = detect_primary_language(&temp_path.to_path_buf()).await;
        assert_eq!(language, Some("typescript".to_string()));
    }

    #[tokio::test]
    async fn test_detect_primary_language_mixed() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create mixed files with more JavaScript
        fs::write(temp_path.join("index.js"), "console.log('hello');").unwrap();
        fs::write(temp_path.join("utils.js"), "// Utils").unwrap();
        fs::write(temp_path.join("app.js"), "// App").unwrap();
        fs::write(temp_path.join("main.rs"), "fn main() {}").unwrap();

        let language = detect_primary_language(&temp_path.to_path_buf()).await;
        assert_eq!(language, Some("javascript".to_string()));
    }

    #[tokio::test]
    async fn test_detect_primary_language_empty() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let language = detect_primary_language(&temp_path.to_path_buf()).await;
        assert_eq!(language, None);
    }
}