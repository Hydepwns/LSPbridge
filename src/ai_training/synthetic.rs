use anyhow::{Context, Result};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ai_training::{TrainingDataset, TrainingPair};
use crate::core::constants::{error_patterns, languages, metadata_keys};
use crate::core::semantic_context::SemanticContext;
use crate::core::types::{Diagnostic, Position, Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DifficultyLevel {
    Beginner,     // Simple syntax errors
    Intermediate, // Type errors, missing imports
    Advanced,     // Complex logic errors
    Expert,       // Architecture-level issues
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingErrorPattern {
    pub name: String,
    pub description: String,
    pub difficulty: DifficultyLevel,
    pub languages: Vec<String>,
    pub transformations: Vec<CodeTransformation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTransformation {
    pub pattern: String,
    pub replacement: String,
    pub diagnostic_message: String,
    pub diagnostic_type: String,
}

pub struct ErrorInjector {
    patterns: HashMap<String, Vec<TrainingErrorPattern>>,
    difficulty_weights: HashMap<DifficultyLevel, f32>,
}

impl ErrorInjector {
    pub fn new() -> Self {
        let mut injector = Self {
            patterns: HashMap::new(),
            difficulty_weights: HashMap::new(),
        };

        // Initialize default difficulty weights
        injector
            .difficulty_weights
            .insert(DifficultyLevel::Beginner, 0.4);
        injector
            .difficulty_weights
            .insert(DifficultyLevel::Intermediate, 0.3);
        injector
            .difficulty_weights
            .insert(DifficultyLevel::Advanced, 0.2);
        injector
            .difficulty_weights
            .insert(DifficultyLevel::Expert, 0.1);

        // Initialize common error patterns
        injector.init_typescript_patterns();
        injector.init_rust_patterns();
        injector.init_python_patterns();

        injector
    }

    pub fn inject_errors(
        &self,
        clean_code: &str,
        language: &str,
        difficulty: Option<DifficultyLevel>,
        count: usize,
    ) -> Result<Vec<TrainingPair>> {
        let patterns = self
            .patterns
            .get(language)
            .context(format!("No patterns available for language: {}", language))?;

        let mut training_pairs = Vec::new();
        let mut rng = thread_rng();

        // Filter patterns by difficulty if specified
        let filtered_patterns: Vec<&TrainingErrorPattern> = if let Some(diff) = difficulty {
            patterns.iter().filter(|p| p.difficulty == diff).collect()
        } else {
            patterns.iter().collect()
        };

        if filtered_patterns.is_empty() {
            anyhow::bail!("No patterns available for specified criteria");
        }

        // First, collect all patterns that can match the code
        let mut applicable_patterns = Vec::new();
        for pattern in &filtered_patterns {
            // Check if any transformation in this pattern can apply
            for transformation in &pattern.transformations {
                if clean_code.contains(&transformation.pattern) {
                    applicable_patterns.push(pattern);
                    break; // Don't add the same pattern multiple times
                }
            }
        }

        if applicable_patterns.is_empty() {
            anyhow::bail!("No applicable patterns found for the provided code");
        }

        // Now generate the requested number of training pairs from applicable patterns
        for _ in 0..count {
            let pattern = applicable_patterns
                .choose(&mut rng)
                .context("Failed to select applicable pattern")?;

            if let Some(pair) = self.apply_pattern(clean_code, pattern, language)? {
                training_pairs.push(pair);
            }
        }

        Ok(training_pairs)
    }

    pub fn generate_gradient_dataset(
        &self,
        base_code: &str,
        language: &str,
        examples_per_level: usize,
    ) -> Result<TrainingDataset> {
        let mut dataset = TrainingDataset::new(
            format!("{} Gradient Dataset", language),
            "Synthetic dataset with increasing difficulty levels".to_string(),
        );

        for difficulty in &[
            DifficultyLevel::Beginner,
            DifficultyLevel::Intermediate,
            DifficultyLevel::Advanced,
            DifficultyLevel::Expert,
        ] {
            // Try to inject errors for this difficulty level, but don't fail if no patterns exist
            match self.inject_errors(base_code, language, Some(*difficulty), examples_per_level) {
                Ok(pairs) => {
                    for mut pair in pairs {
                        pair.add_metadata(
                            metadata_keys::DIFFICULTY.to_string(),
                            serde_json::json!(difficulty),
                        );
                        dataset.add_pair(pair);
                    }
                }
                Err(_) => {
                    // No patterns available for this difficulty level, skip it
                    continue;
                }
            }
        }

        Ok(dataset)
    }

    fn apply_pattern(
        &self,
        code: &str,
        pattern: &TrainingErrorPattern,
        language: &str,
    ) -> Result<Option<TrainingPair>> {
        let mut rng = thread_rng();

        // Select a random transformation from the pattern
        let transformation = pattern
            .transformations
            .choose(&mut rng)
            .context("No transformations in pattern")?;

        // Apply the transformation
        if code.contains(&transformation.pattern) {
            let error_code = code.replace(&transformation.pattern, &transformation.replacement);

            // Find the line number of the change
            let line_num = code
                .lines()
                .position(|line| line.contains(&transformation.pattern))
                .unwrap_or(0)
                + 1;

            // Create diagnostic
            let diagnostic = Diagnostic::new(
                format!("synthetic.{}", language),
                Range {
                    start: Position {
                        line: line_num as u32,
                        character: 0,
                    },
                    end: Position {
                        line: line_num as u32,
                        character: 80,
                    },
                },
                crate::core::types::DiagnosticSeverity::Error,
                transformation.diagnostic_message.clone(),
                language.to_string(),
            );

            let pair = TrainingPair::new(
                error_code,
                code.to_string(),
                vec![diagnostic],
                SemanticContext::default(),
                language.to_string(),
            )
            .with_confidence(1.0) // Synthetic data has perfect confidence
            .with_description(format!("Fix {}: {}", pattern.name, pattern.description));

            Ok(Some(pair))
        } else {
            Ok(None)
        }
    }

    fn init_typescript_patterns(&mut self) {
        let patterns = vec![
            // Beginner patterns
            TrainingErrorPattern {
                name: "missing_semicolon".to_string(),
                description: "Missing semicolon".to_string(),
                difficulty: DifficultyLevel::Beginner,
                languages: vec![languages::TYPESCRIPT.to_string()],
                transformations: vec![CodeTransformation {
                    pattern: "\";".to_string(),
                    replacement: "\"".to_string(),
                    diagnostic_message: error_patterns::MISSING_SEMICOLON.to_string(),
                    diagnostic_type: "syntax".to_string(),
                }],
            },
            TrainingErrorPattern {
                name: "typo_in_console".to_string(),
                description: "Typo in console method".to_string(),
                difficulty: DifficultyLevel::Beginner,
                languages: vec![languages::TYPESCRIPT.to_string()],
                transformations: vec![CodeTransformation {
                    pattern: "console.log".to_string(),
                    replacement: "console.lg".to_string(),
                    diagnostic_message:
                        "Property 'lg' does not exist on type 'Console'. Did you mean 'log'?"
                            .to_string(),
                    diagnostic_type: "type".to_string(),
                }],
            },
            // Intermediate patterns
            TrainingErrorPattern {
                name: "type_mismatch".to_string(),
                description: "Type annotation mismatch".to_string(),
                difficulty: DifficultyLevel::Intermediate,
                languages: vec![languages::TYPESCRIPT.to_string()],
                transformations: vec![CodeTransformation {
                    pattern: "const x: string =".to_string(),
                    replacement: "const x: number =".to_string(),
                    diagnostic_message: "Type 'string' is not assignable to type 'number'"
                        .to_string(),
                    diagnostic_type: "type".to_string(),
                }],
            },
            // Advanced patterns
            TrainingErrorPattern {
                name: "undefined_property".to_string(),
                description: "Accessing undefined property".to_string(),
                difficulty: DifficultyLevel::Advanced,
                languages: vec![languages::TYPESCRIPT.to_string()],
                transformations: vec![CodeTransformation {
                    pattern: "user.name".to_string(),
                    replacement: "user.firstName".to_string(),
                    diagnostic_message: "Property 'firstName' does not exist on type 'User'"
                        .to_string(),
                    diagnostic_type: "type".to_string(),
                }],
            },
        ];

        self.patterns
            .insert(languages::TYPESCRIPT.to_string(), patterns);
    }

    fn init_rust_patterns(&mut self) {
        let patterns = vec![
            // Beginner patterns
            TrainingErrorPattern {
                name: "missing_mut".to_string(),
                description: "Missing mut keyword".to_string(),
                difficulty: DifficultyLevel::Beginner,
                languages: vec![languages::RUST.to_string()],
                transformations: vec![CodeTransformation {
                    pattern: "let mut x = 5;".to_string(),
                    replacement: "let x = 5;".to_string(),
                    diagnostic_message: "cannot assign twice to immutable variable".to_string(),
                    diagnostic_type: "E0384".to_string(),
                }],
            },
            // Intermediate patterns
            TrainingErrorPattern {
                name: "borrow_checker".to_string(),
                description: "Borrow checker violation".to_string(),
                difficulty: DifficultyLevel::Intermediate,
                languages: vec![languages::RUST.to_string()],
                transformations: vec![CodeTransformation {
                    pattern: "&data".to_string(),
                    replacement: "&mut data".to_string(),
                    diagnostic_message: error_patterns::CANNOT_BORROW_MUTABLE.to_string(),
                    diagnostic_type: "E0596".to_string(),
                }],
            },
        ];

        self.patterns.insert(languages::RUST.to_string(), patterns);
    }

    fn init_python_patterns(&mut self) {
        let patterns = vec![
            // Beginner patterns
            TrainingErrorPattern {
                name: "indentation_error".to_string(),
                description: "Incorrect indentation".to_string(),
                difficulty: DifficultyLevel::Beginner,
                languages: vec![languages::PYTHON.to_string()],
                transformations: vec![CodeTransformation {
                    pattern: "    return x".to_string(),
                    replacement: "   return x".to_string(),
                    diagnostic_message: "IndentationError: unexpected indent".to_string(),
                    diagnostic_type: "syntax".to_string(),
                }],
            },
            // Intermediate patterns
            TrainingErrorPattern {
                name: "undefined_variable".to_string(),
                description: "Using undefined variable".to_string(),
                difficulty: DifficultyLevel::Intermediate,
                languages: vec![languages::PYTHON.to_string()],
                transformations: vec![CodeTransformation {
                    pattern: "result = x + y".to_string(),
                    replacement: "result = x + z".to_string(),
                    diagnostic_message: "NameError: name 'z' is not defined".to_string(),
                    diagnostic_type: "runtime".to_string(),
                }],
            },
        ];

        self.patterns
            .insert(languages::PYTHON.to_string(), patterns);
    }

    pub fn add_custom_pattern(&mut self, language: String, pattern: TrainingErrorPattern) {
        self.patterns
            .entry(language)
            .or_insert_with(Vec::new)
            .push(pattern);
    }
}

impl Default for ErrorInjector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_injection() {
        let injector = ErrorInjector::new();

        let clean_code = r#"
const x: string = "hello";
const y: number = 42;
console.log(x, y);
"#;

        let result = injector.inject_errors(clean_code, "typescript", None, 1);
        assert!(result.is_ok());

        let pairs = result.unwrap();
        assert!(!pairs.is_empty());

        let pair = &pairs[0];
        assert_ne!(pair.before_code, pair.after_code);
        assert!(!pair.diagnostics.is_empty());
    }

    #[test]
    fn test_gradient_dataset() {
        let injector = ErrorInjector::new();

        let base_code = r#"
const user = { name: "John" };
const x: string = "hello";
console.log(user.name);
"#;

        let result = injector.generate_gradient_dataset(base_code, "typescript", 1);
        assert!(result.is_ok());

        let dataset = result.unwrap();
        assert!(dataset.pairs.len() >= 1);

        // Check that we have different difficulty levels
        let difficulties: Vec<_> = dataset
            .pairs
            .iter()
            .filter_map(|p| p.metadata.get("difficulty"))
            .collect();
        assert!(!difficulties.is_empty());
    }
}
