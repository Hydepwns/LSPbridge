use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

use crate::cli::args::OutputFormat;
use crate::cli::commands::Command;
use crate::core::{Diagnostic, DiagnosticResult, DiagnosticSeverity};
use crate::quick_fix::{
    ConfidenceThreshold, FixApplicationEngine, FixConfidenceScorer, FixEdit, FixVerifier,
    QuickFixAction, RollbackManager,
};

pub struct QuickFixCommand {
    action: QuickFixAction,
}

impl QuickFixCommand {
    pub fn new(action: QuickFixAction) -> Self {
        Self { action }
    }
}

#[async_trait]
impl Command for QuickFixCommand {
    async fn execute(&self) -> Result<()> {
        match &self.action {
            QuickFixAction::Apply {
                threshold,
                errors_only,
                verify_tests,
                verify_build,
                backup,
                dry_run,
                files,
            } => {
                self.apply_fixes(
                    *threshold,
                    *errors_only,
                    *verify_tests,
                    *verify_build,
                    *backup,
                    *dry_run,
                    files.clone(),
                )
                .await
            }
            QuickFixAction::Rollback { session_id, list } => {
                self.rollback_fixes(session_id.clone(), *list).await
            }
            QuickFixAction::Analyze { detailed, format } => {
                self.analyze_fixes(*detailed, format).await
            }
        }
    }
}

impl QuickFixCommand {
    async fn apply_fixes(
        &self,
        threshold: f64,
        errors_only: bool,
        verify_tests: bool,
        verify_build: bool,
        backup: bool,
        dry_run: bool,
        files: Option<String>,
    ) -> Result<()> {
        // Get current diagnostics
        let diagnostics = DiagnosticResult::new(); // Would normally capture from LSP

        // Set up confidence scorer
        let scorer = FixConfidenceScorer::new();
        let confidence_threshold = ConfidenceThreshold {
            auto_apply: threshold as f32,
            suggest: (threshold * 0.7) as f32,
            minimum: 0.3,
        };

        // Set up fix engine
        let engine = FixApplicationEngine::new().with_backups(backup);

        // Set up verifier if needed
        let verifier = if verify_tests || verify_build {
            Some(
                FixVerifier::new()
                    .with_tests(verify_tests)
                    .with_build_check(verify_build)
                    .with_lsp_validation(true), // Enable LSP validation
            )
        } else {
            None
        };

        // Set up rollback manager
        let rollback_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lspbridge")
            .join("rollback");
        let mut rollback_manager = RollbackManager::new(rollback_dir);
        rollback_manager.init().await?;

        let mut fixes_to_apply = Vec::new();
        let mut all_backups = Vec::new();

        // Analyze each diagnostic
        for (file_path, file_diagnostics) in diagnostics.diagnostics {
            // Filter by file pattern if specified
            if let Some(ref pattern) = files {
                if !file_path.to_string_lossy().contains(pattern) {
                    continue;
                }
            }

            for diag in file_diagnostics {
                // Filter by severity if needed
                if errors_only && diag.severity != DiagnosticSeverity::Error {
                    continue;
                }

                // For demo purposes, create a simple fix
                // In real implementation, would get from LSP code actions
                if let Some(fix_edit) = create_demo_fix(&diag) {
                    let (confidence, _factors) =
                        scorer.score_fix(&diag, &fix_edit.new_text, false);

                    if dry_run {
                        println!(
                            "Would fix: {} (confidence: {:.2})",
                            diag.message,
                            confidence.value()
                        );
                        if confidence.is_auto_applicable(&confidence_threshold) {
                            println!("  ✓ Auto-applicable");
                        } else {
                            println!("  ⚠ Requires confirmation");
                        }
                    } else if confidence.is_auto_applicable(&confidence_threshold) {
                        fixes_to_apply.push((fix_edit, confidence));
                    }
                }
            }
        }

        if dry_run {
            println!(
                "\nTotal fixes that would be applied: {}",
                fixes_to_apply.len()
            );
            return Ok(());
        }

        // Apply fixes
        println!("Applying {} fixes...", fixes_to_apply.len());
        let results = engine
            .apply_fixes_with_confidence(&fixes_to_apply, &confidence_threshold)
            .await?;

        // Collect backups for rollback
        for (result, _) in &results {
            if let Some(ref backup) = result.backup {
                all_backups.push(backup.clone());
            }
        }

        // Save rollback state
        if !all_backups.is_empty() {
            let rollback_state = RollbackManager::create_state(
                all_backups,
                format!("Applied {} fixes", results.len()),
            );
            let session_id = rollback_state.session_id.clone();
            rollback_manager.save_state(rollback_state).await?;
            println!("✅ Fixes applied. Rollback session: {session_id}");
        }

        // Verify if requested
        if let Some(verifier) = verifier {
            println!("🔍 Verifying fixes...");
            let mut verification_results = Vec::new();
            
            for ((fix_edit, _confidence), (result, original_diag)) in fixes_to_apply.iter().zip(&results) {
                if result.success {
                    println!("  Verifying fix for: {}", fix_edit.file_path.display());
                    
                    // Create a dummy diagnostic for verification
                    // In a real implementation, we'd pass the original diagnostic
                    let dummy_diagnostic = create_verification_diagnostic(fix_edit);
                    
                    match verifier.verify_fix(&dummy_diagnostic, result).await {
                        Ok(verification) => {
                            verification_results.push(verification.clone());
                            
                            if verification.issue_resolved {
                                println!("    ✅ Issue resolved successfully");
                            } else {
                                println!("    ❌ Issue may not be fully resolved");
                            }
                            
                            if !verification.new_issues.is_empty() {
                                println!("    ⚠️  {} new issues detected", verification.new_issues.len());
                            }
                            
                            if !verification.build_status.success {
                                println!("    🔨 Build failed after fix");
                                for error in &verification.build_status.errors {
                                    println!("      Error: {}", error);
                                }
                            }
                            
                            if let Some(test_results) = &verification.test_results {
                                if test_results.failed > 0 {
                                    println!("    🧪 {} tests failed", test_results.failed);
                                } else {
                                    println!("    🧪 All {} tests passed", test_results.passed);
                                }
                            }
                        }
                        Err(e) => {
                            println!("    ❌ Verification failed: {}", e);
                        }
                    }
                }
            }
            
            // Verification summary
            let successful_verifications = verification_results.iter()
                .filter(|v| v.issue_resolved && v.build_status.success)
                .count();
            let failed_verifications = verification_results.len() - successful_verifications;
            
            println!("\n🔍 Verification Summary:");
            println!("  ✅ Successfully verified: {}", successful_verifications);
            if failed_verifications > 0 {
                println!("  ❌ Failed verification: {}", failed_verifications);
            }
        }

        // Summary
        let successful = results.iter().filter(|(r, _)| r.success).count();
        let failed = results.len() - successful;
        println!("\n📊 Summary:");
        println!("  ✓ Successfully applied: {successful}");
        if failed > 0 {
            println!("  ✗ Failed: {failed}");
        }

        Ok(())
    }

    async fn rollback_fixes(&self, session_id: Option<String>, list: bool) -> Result<()> {
        let rollback_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lspbridge")
            .join("rollback");
        let mut rollback_manager = RollbackManager::new(rollback_dir);
        rollback_manager.init().await?;

        if list {
            let states = rollback_manager.list_states().await?;
            if states.is_empty() {
                println!("No rollback sessions available");
            } else {
                println!("Available rollback sessions:");
                for state in states {
                    println!(
                        "  {} - {} ({}{})",
                        state.session_id,
                        state.description,
                        state.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        if state.rolled_back {
                            " - already rolled back"
                        } else {
                            ""
                        }
                    );
                }
            }
        } else {
            match session_id {
                Some(id) => {
                    rollback_manager.rollback(&id).await?;
                    println!("✅ Rolled back session: {id}");
                }
                None => {
                    rollback_manager.rollback_latest().await?;
                    println!("✅ Rolled back latest session");
                }
            }
        }

        Ok(())
    }

    async fn analyze_fixes(&self, detailed: bool, format: &OutputFormat) -> Result<()> {
        let diagnostics = DiagnosticResult::new(); // Would normally capture from LSP
        let scorer = FixConfidenceScorer::new();

        let mut analysis_results = Vec::new();

        for (_file_path, file_diagnostics) in diagnostics.diagnostics {
            for diag in file_diagnostics {
                if let Some(fix_edit) = create_demo_fix(&diag) {
                    let (confidence, factors) = scorer.score_fix(&diag, &fix_edit.new_text, false);
                    analysis_results.push((diag, confidence, factors));
                }
            }
        }

        // Format output
        match format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&analysis_results)?;
                println!("{json}");
            }
            OutputFormat::Markdown => {
                println!("# Fix Confidence Analysis\n");
                for (diag, confidence, factors) in &analysis_results {
                    println!("## {}", diag.message);
                    println!("- **File**: {}", diag.file);
                    println!("- **Confidence**: {:.2}", confidence.value());
                    if detailed {
                        println!("- **Factors**:");
                        println!(
                            "  - Pattern recognition: {:.2}",
                            factors.pattern_recognition
                        );
                        println!("  - Fix complexity: {:.2}", factors.fix_complexity);
                        println!("  - Safety score: {:.2}", factors.safety_score);
                        println!(
                            "  - Language confidence: {:.2}",
                            factors.language_confidence
                        );
                    }
                    println!();
                }
            }
            _ => {
                // Table format
                println!(
                    "{:<50} {:<10} {:<10}",
                    "Diagnostic", "Confidence", "Auto-Apply"
                );
                println!("{}", "-".repeat(72));
                for (diag, confidence, _) in &analysis_results {
                    let auto = if confidence.value() >= 0.9 { "Yes" } else { "No" };
                    println!(
                        "{:<50} {:<10.2} {:<10}",
                        diag.message.chars().take(47).collect::<String>(),
                        confidence.value(),
                        auto
                    );
                }
            }
        }

        Ok(())
    }
}

fn create_demo_fix(diagnostic: &Diagnostic) -> Option<FixEdit> {
    // This is a simplified demo - real implementation would use LSP code actions
    match diagnostic.code.as_deref() {
        Some("TS2322") => {
            // Type mismatch - simple demo fix
            Some(FixEdit {
                file_path: PathBuf::from(&diagnostic.file),
                range: diagnostic.range.clone(),
                new_text: "fixed_type".to_string(),
                description: Some("Fix type mismatch".to_string()),
            })
        }
        Some("missing_semicolon") => Some(FixEdit {
            file_path: PathBuf::from(&diagnostic.file),
            range: diagnostic.range.clone(),
            new_text: ";".to_string(),
            description: Some("Add missing semicolon".to_string()),
        }),
        _ => None,
    }
}

fn create_verification_diagnostic(fix_edit: &FixEdit) -> Diagnostic {
    use uuid::Uuid;
    use crate::core::{Position, Range};
    
    Diagnostic {
        id: Uuid::new_v4().to_string(),
        file: fix_edit.file_path.to_string_lossy().to_string(),
        range: fix_edit.range.clone(),
        severity: DiagnosticSeverity::Error,
        message: fix_edit.description.as_deref().unwrap_or("Unknown issue").to_string(),
        code: Some("verification_test".to_string()),
        source: "quick_fix_verifier".to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}