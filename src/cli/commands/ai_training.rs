use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

use crate::ai_training::{
    AIExportFormat, AITrainingAction, AnnotationTool, DifficultyLevel, ErrorInjector,
    ExportFormat as AIFormat, FixQuality, TrainingDataset, TrainingExporter, TrainingPair,
};
use crate::cli::args::OutputFormat;
use crate::cli::commands::Command;
use crate::core::{DiagnosticResult, DiagnosticSeverity};

pub struct AITrainingCommand {
    action: AITrainingAction,
}

impl AITrainingCommand {
    pub fn new(action: AITrainingAction) -> Self {
        Self { action }
    }
}

#[async_trait]
impl Command for AITrainingCommand {
    async fn execute(&self) -> Result<()> {
        match &self.action {
            AITrainingAction::Export {
                output,
                format,
                high_confidence_only,
                max_tokens,
                language,
            } => {
                self.export_training_data(
                    output,
                    format,
                    *high_confidence_only,
                    *max_tokens,
                    language.clone(),
                )
                .await
            }
            AITrainingAction::Synthetic {
                input,
                output,
                language,
                difficulty,
                count,
                gradient,
            } => {
                self.generate_synthetic_data(
                    input,
                    output,
                    language,
                    difficulty.as_ref(),
                    *count,
                    *gradient,
                )
                .await
            }
            AITrainingAction::Annotate {
                dataset,
                annotator,
                auto_quality,
                output,
            } => {
                self.annotate_dataset(dataset, annotator, auto_quality.as_ref(), output)
                    .await
            }
            AITrainingAction::Report { dataset, format } => {
                self.generate_report(dataset, format).await
            }
        }
    }
}

impl AITrainingCommand {
    async fn export_training_data(
        &self,
        output: &PathBuf,
        format: &AIExportFormat,
        high_confidence_only: bool,
        max_tokens: Option<usize>,
        language: Option<String>,
    ) -> Result<()> {
        // Get current diagnostics from stdin or a mock source
        // For now, create an empty result as this would normally come from LSP
        let diagnostics = DiagnosticResult::new();

        // Convert diagnostics to training pairs
        let mut dataset = TrainingDataset::new(
            "Diagnostic Training Data".to_string(),
            "Training data generated from current diagnostics".to_string(),
        );

        // Convert diagnostics to training pairs (simplified for now)
        // In a real implementation, we'd need to extract before/after code from fixes
        for (file_path, file_diagnostics) in diagnostics.diagnostics {
            for diag in file_diagnostics {
                // Skip if filtering by confidence
                if high_confidence_only && diag.severity != DiagnosticSeverity::Error {
                    continue;
                }

                // Skip if filtering by language
                if let Some(ref lang_filter) = language {
                    let file_lang = file_path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("");
                    if !file_lang.contains(lang_filter) {
                        continue;
                    }
                }

                // Create a training pair from the diagnostic
                // This is simplified - in reality we'd need the actual fix
                let pair = TrainingPair::new(
                    format!("// Code with error at line {}", diag.range.start.line),
                    format!("// Fixed code"),
                    vec![diag.clone()],
                    crate::core::semantic_context::SemanticContext::default(),
                    detect_language(&file_path),
                );

                dataset.add_pair(pair);
            }
        }

        // Export the dataset
        let export_format = match format {
            AIExportFormat::JsonLines => AIFormat::JsonLines,
            AIExportFormat::Parquet => AIFormat::Parquet,
            AIExportFormat::HuggingFace => AIFormat::HuggingFace,
            AIExportFormat::OpenAI => AIFormat::OpenAI,
            AIExportFormat::Custom => AIFormat::Custom,
        };

        let mut exporter = TrainingExporter::new(export_format);
        if let Some(tokens) = max_tokens {
            exporter = exporter.with_max_tokens(tokens);
        }

        exporter.export_dataset(&dataset, output).await?;

        println!(
            "✅ Exported {} training pairs to {}",
            dataset.pairs.len(),
            output.display()
        );

        Ok(())
    }

    async fn generate_synthetic_data(
        &self,
        input: &PathBuf,
        output: &PathBuf,
        language: &str,
        difficulty: Option<&DifficultyLevel>,
        count: usize,
        gradient: bool,
    ) -> Result<()> {
        let injector = ErrorInjector::new();

        if gradient {
            // Read base code
            let base_code = fs::read_to_string(input).await?;

            // Generate gradient dataset
            let dataset = injector.generate_gradient_dataset(&base_code, language, count / 4)?;

            // Save dataset
            let json = serde_json::to_string_pretty(&dataset)?;
            fs::write(output, json).await?;

            println!(
                "✅ Generated gradient dataset with {} examples",
                dataset.pairs.len()
            );
        } else {
            // Read base code
            let base_code = fs::read_to_string(input).await?;

            // Convert difficulty level
            let diff = difficulty.map(|d| match d {
                DifficultyLevel::Beginner => crate::ai_training::DifficultyLevel::Beginner,
                DifficultyLevel::Intermediate => crate::ai_training::DifficultyLevel::Intermediate,
                DifficultyLevel::Advanced => crate::ai_training::DifficultyLevel::Advanced,
                DifficultyLevel::Expert => crate::ai_training::DifficultyLevel::Expert,
            });

            // Generate synthetic errors
            let pairs = injector.inject_errors(&base_code, language, diff, count)?;

            // Create dataset
            let mut dataset = TrainingDataset::new(
                format!("{} Synthetic Dataset", language),
                "Synthetic training data with injected errors".to_string(),
            );

            for pair in pairs {
                dataset.add_pair(pair);
            }

            // Save dataset
            let json = serde_json::to_string_pretty(&dataset)?;
            fs::write(output, json).await?;

            println!(
                "✅ Generated {} synthetic training examples",
                dataset.pairs.len()
            );
        }

        Ok(())
    }

    async fn annotate_dataset(
        &self,
        dataset: &PathBuf,
        annotator: &str,
        auto_quality: Option<&FixQuality>,
        output: &PathBuf,
    ) -> Result<()> {
        // Load dataset
        let json = fs::read_to_string(dataset).await?;
        let mut training_dataset: TrainingDataset = serde_json::from_str(&json)?;

        let mut tool = AnnotationTool::new();
        tool.start_session(annotator.to_string(), training_dataset.id.clone());

        if let Some(quality_threshold) = auto_quality {
            // Auto-annotate
            let annotations = tool.batch_annotate(&mut training_dataset, *quality_threshold)?;
            println!("✅ Auto-annotated {} training pairs", annotations.len());
        } else {
            // Interactive manual annotation
            self.interactive_annotation(&mut tool, &mut training_dataset)?;
        }

        // Save annotated dataset
        let json = serde_json::to_string_pretty(&training_dataset)?;
        fs::write(output, json).await?;

        println!("✅ Saved annotated dataset to {}", output.display());

        Ok(())
    }

    fn interactive_annotation(
        &self,
        tool: &mut AnnotationTool,
        training_dataset: &mut TrainingDataset,
    ) -> Result<()> {
        use std::io::{self, Write};

        println!("🔍 Starting manual annotation session...");
        println!(
            "Dataset: {} ({} pairs)",
            training_dataset.name,
            training_dataset.pairs.len()
        );
        println!();

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        // Iterate through unannotated pairs
        let mut annotated_count = 0;
        let total_pairs = training_dataset.pairs.len();

        for (idx, pair) in training_dataset.pairs.iter_mut().enumerate() {
            // Check if already annotated
            if pair.metadata.contains_key("annotation") {
                continue;
            }

            // Clear screen for better readability
            print!("\x1B[2J\x1B[1;1H");

            // Display progress
            println!("📊 Progress: {}/{} pairs annotated", annotated_count, total_pairs);
            println!("📍 Current: {}/{}\n", idx + 1, total_pairs);

            // Display pair information
            println!("🔧 Fix Description: {}", pair.fix_description);
            println!("📝 Language: {}", pair.language);
            println!("📁 File: {}", pair.file_path);
            println!(
                "⚡ Confidence: {:.2} ({:?})",
                pair.confidence.score, pair.confidence.category
            );
            println!();

            // Display diagnostics
            if !pair.diagnostics.is_empty() {
                println!("🚨 Diagnostics:");
                for diag in &pair.diagnostics {
                    println!("  - [{}] {}", diag.severity, diag.message);
                }
                println!();
            }

            // Display code diff
            println!("📄 Before:");
            println!("```{}", pair.language);
            let before_lines: Vec<&str> = pair.before_code.lines().collect();
            for (i, line) in before_lines.iter().enumerate() {
                println!("{:4} | {}", i + 1, line);
            }
            println!("```\n");

            println!("✅ After:");
            println!("```{}", pair.language);
            let after_lines: Vec<&str> = pair.after_code.lines().collect();
            for (i, line) in after_lines.iter().enumerate() {
                println!("{:4} | {}", i + 1, line);
            }
            println!("```\n");

            // Show annotation options
            println!("🏷️  Quality Assessment:");
            println!("  [1] Perfect    - Fixes all issues correctly");
            println!("  [2] Good       - Fixes main issue but may have minor issues");
            println!("  [3] Acceptable - Works but not ideal");
            println!("  [4] Poor       - Has problems");
            println!("  [5] Incorrect  - Doesn't fix the issue");
            println!("  [s] Skip       - Skip this pair");
            println!("  [q] Quit       - Save and exit");
            println!();

            // Get user input
            print!("Enter your choice: ");
            stdout.flush()?;

            let mut input = String::new();
            stdin.read_line(&mut input)?;
            let choice = input.trim().to_lowercase();

            // Create a default verification result for manual annotations
            let verification = crate::ai_training::annotation::VerificationResult {
                compiles: true,  // Assume manual review verified compilation
                tests_pass: None, // Unknown without running tests
                linter_warnings: vec![],
                performance_impact: None,
                side_effects: vec![],
            };

            match choice.as_str() {
                "1" => {
                    tool.annotate_pair(
                        pair,
                        FixQuality::Perfect,
                        "Manual annotation".to_string(),
                        vec![],
                        verification.clone(),
                    )?;
                    annotated_count += 1;
                    println!("✓ Annotated as Perfect");
                }
                "2" => {
                    tool.annotate_pair(
                        pair,
                        FixQuality::Good,
                        "Manual annotation".to_string(),
                        vec![],
                        verification.clone(),
                    )?;
                    annotated_count += 1;
                    println!("✓ Annotated as Good");
                }
                "3" => {
                    tool.annotate_pair(
                        pair,
                        FixQuality::Acceptable,
                        "Manual annotation".to_string(),
                        vec![],
                        verification.clone(),
                    )?;
                    annotated_count += 1;
                    println!("✓ Annotated as Acceptable");
                }
                "4" => {
                    tool.annotate_pair(
                        pair,
                        FixQuality::Poor,
                        "Manual annotation".to_string(),
                        vec![],
                        verification.clone(),
                    )?;
                    annotated_count += 1;
                    println!("✓ Annotated as Poor");
                }
                "5" => {
                    tool.annotate_pair(
                        pair,
                        FixQuality::Incorrect,
                        "Manual annotation".to_string(),
                        vec![],
                        verification.clone(),
                    )?;
                    annotated_count += 1;
                    println!("✓ Annotated as Incorrect");
                }
                "s" => {
                    println!("⏭️  Skipped");
                    continue;
                }
                "q" => {
                    println!("💾 Saving and exiting...");
                    break;
                }
                _ => {
                    println!("❌ Invalid choice. Please try again.");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            }

            // Brief pause to show confirmation
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        // Complete the session
        if let Ok(session) = tool.complete_session() {
            println!("\n✅ Annotation session completed!");
            println!("   Total annotations: {}", session.annotations.len());
            if let Some(completed_at) = session.completed_at {
                println!("   Duration: {:?}", completed_at - session.started_at);
            }
        }

        Ok(())
    }

    async fn generate_report(&self, dataset: &PathBuf, format: &OutputFormat) -> Result<()> {
        // Load dataset
        let json = fs::read_to_string(dataset).await?;
        let training_dataset: TrainingDataset = serde_json::from_str(&json)?;

        let tool = AnnotationTool::new();
        let report = tool.get_annotation_report(&training_dataset)?;

        // Format report
        let output = match format {
            OutputFormat::Json => serde_json::to_string_pretty(&report)?,
            OutputFormat::Markdown => format_annotation_report_markdown(&report, &training_dataset),
            OutputFormat::Claude => format_annotation_report_claude(&report, &training_dataset),
        };

        println!("{}", output);

        Ok(())
    }
}

fn detect_language(path: &PathBuf) -> String {
    use crate::core::constants::languages;
    
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("ts") | Some("tsx") => languages::TYPESCRIPT.to_string(),
        Some("js") | Some("jsx") => languages::JAVASCRIPT.to_string(),
        Some("rs") => languages::RUST.to_string(),
        Some("py") => languages::PYTHON.to_string(),
        Some("go") => languages::GO.to_string(),
        Some("java") => languages::JAVA.to_string(),
        Some("cpp") | Some("cc") | Some("cxx") => languages::CPP.to_string(),
        Some("c") => languages::C.to_string(),
        Some("cs") => "csharp".to_string(),
        Some("rb") => "ruby".to_string(),
        Some("php") => "php".to_string(),
        Some("swift") => "swift".to_string(),
        Some("kt") => "kotlin".to_string(),
        _ => "unknown".to_string(),
    }
}

fn format_annotation_report_markdown(
    report: &crate::ai_training::annotation::AnnotationReport,
    dataset: &TrainingDataset,
) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    let _ = writeln!(&mut output, "# Annotation Report: {}\n", dataset.name);
    let _ = writeln!(&mut output, "## Summary");
    let _ = writeln!(&mut output, "- **Total Pairs**: {}", report.total_pairs);
    let _ = writeln!(&mut output, "- **Annotated**: {}", report.annotated_count);
    let _ = writeln!(
        &mut output,
        "- **Completion**: {:.1}%",
        (report.annotated_count as f64 / report.total_pairs as f64) * 100.0
    );
    let _ = writeln!(&mut output, "- **Sessions**: {}", report.sessions.len());
    let _ = writeln!(&mut output, "- **Unique Annotators**: {}", report.unique_annotators);
    let _ = writeln!(&mut output);

    let _ = writeln!(&mut output, "## Quality Distribution");
    let _ = writeln!(
        &mut output,
        "- Perfect: {} ({:.1}%)",
        report.quality_distribution.get(&FixQuality::Perfect).unwrap_or(&0),
        (report.quality_distribution.get(&FixQuality::Perfect).unwrap_or(&0) as f64
            / report.annotated_count as f64)
            * 100.0
    );
    let _ = writeln!(
        &mut output,
        "- Good: {} ({:.1}%)",
        report.quality_distribution.get(&FixQuality::Good).unwrap_or(&0),
        (report.quality_distribution.get(&FixQuality::Good).unwrap_or(&0) as f64
            / report.annotated_count as f64)
            * 100.0
    );
    let _ = writeln!(
        &mut output,
        "- Acceptable: {} ({:.1}%)",
        report.quality_distribution.get(&FixQuality::Acceptable).unwrap_or(&0),
        (report.quality_distribution.get(&FixQuality::Acceptable).unwrap_or(&0) as f64
            / report.annotated_count as f64)
            * 100.0
    );
    let _ = writeln!(
        &mut output,
        "- Poor: {} ({:.1}%)",
        report.quality_distribution.get(&FixQuality::Poor).unwrap_or(&0),
        (report.quality_distribution.get(&FixQuality::Poor).unwrap_or(&0) as f64
            / report.annotated_count as f64)
            * 100.0
    );
    let _ = writeln!(
        &mut output,
        "- Incorrect: {} ({:.1}%)",
        report.quality_distribution.get(&FixQuality::Incorrect).unwrap_or(&0),
        (report.quality_distribution.get(&FixQuality::Incorrect).unwrap_or(&0) as f64
            / report.annotated_count as f64)
            * 100.0
    );

    output
}

fn format_annotation_report_claude(
    report: &crate::ai_training::annotation::AnnotationReport,
    dataset: &TrainingDataset,
) -> String {
    format_annotation_report_markdown(report, dataset) // Same format for now
}