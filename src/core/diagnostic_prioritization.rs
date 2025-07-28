use super::diagnostic_grouping::DiagnosticGroup;
use super::types::{Diagnostic, DiagnosticSeverity};
use crate::analyzers::{LanguageAnalyzer, RustAnalyzer, TypeScriptAnalyzer};
use crate::simple_builder;
use std::collections::HashMap;

/// A prioritized diagnostic with scoring information
#[derive(Debug, Clone)]
pub struct PrioritizedDiagnostic {
    /// The diagnostic group
    pub group: DiagnosticGroup,
    /// Overall priority score (0.0 - 100.0, higher is more important)
    pub priority_score: f32,
    /// Estimated complexity to fix (1-5)
    pub fix_complexity: u8,
    /// Estimated impact radius (how many other errors this might fix)
    pub impact_radius: u32,
    /// Breakdown of scoring factors
    pub score_breakdown: ScoreBreakdown,
}

// Apply builder pattern to ScoreBreakdown
simple_builder! {
    #[derive(Debug, Clone)]
    pub struct ScoreBreakdown {
        pub severity_score: f32 = 0.0,
        pub location_score: f32 = 0.0,
        pub impact_score: f32 = 0.0,
        pub complexity_score: f32 = 0.0,
        pub category_score: f32 = 0.0,
    }
}

/// Service for prioritizing diagnostics
pub struct DiagnosticPrioritizer {
    analyzers: HashMap<String, Box<dyn LanguageAnalyzer>>,
}

impl DiagnosticPrioritizer {
    pub fn new() -> Self {
        let mut analyzers: HashMap<String, Box<dyn LanguageAnalyzer>> = HashMap::new();
        analyzers.insert(
            "typescript".to_string(),
            Box::new(TypeScriptAnalyzer::new()),
        );
        analyzers.insert("rust".to_string(), Box::new(RustAnalyzer::new()));

        Self { analyzers }
    }

    /// Prioritize diagnostic groups based on importance
    pub fn prioritize(&self, groups: Vec<DiagnosticGroup>) -> Vec<PrioritizedDiagnostic> {
        let mut prioritized: Vec<PrioritizedDiagnostic> = groups
            .into_iter()
            .map(|group| self.score_diagnostic_group(group))
            .collect();

        // Sort by priority score (highest first)
        prioritized.sort_by(|a, b| b.priority_score.partial_cmp(&a.priority_score).unwrap());

        prioritized
    }

    fn score_diagnostic_group(&self, group: DiagnosticGroup) -> PrioritizedDiagnostic {
        let primary = &group.primary;

        // Get language-specific analysis
        let analyzer = self.get_analyzer(primary);
        let analysis = analyzer
            .as_ref()
            .map(|a| a.analyze_diagnostic(primary, None))
            .unwrap_or_default();

        // Calculate various scoring factors
        let severity_score = self.calculate_severity_score(primary);
        let location_score = self.calculate_location_score(primary);
        let impact_score = self.calculate_impact_score(&group, analysis.is_cascading);
        let complexity_score = self.calculate_complexity_score(analysis.fix_complexity);
        let category_score = self.calculate_category_score(&analysis.category);

        // Calculate overall priority score
        let priority_score = (severity_score * 0.35)
            + (location_score * 0.15)
            + (impact_score * 0.25)
            + (complexity_score * 0.15)
            + (category_score * 0.10);

        // Estimate impact radius
        let impact_radius = if analysis.is_cascading {
            group.related.len() as u32 + 5 // Cascading errors likely affect more
        } else {
            group.related.len() as u32
        };

        PrioritizedDiagnostic {
            group,
            priority_score,
            fix_complexity: analysis.fix_complexity,
            impact_radius,
            score_breakdown: ScoreBreakdown {
                severity_score,
                location_score,
                impact_score,
                complexity_score,
                category_score,
            },
        }
    }

    fn get_analyzer(&self, diagnostic: &Diagnostic) -> Option<&Box<dyn LanguageAnalyzer>> {
        // Try to determine language from source
        let language = diagnostic.source.to_lowercase();

        if language.contains("typescript") || language.contains("eslint") {
            self.analyzers.get("typescript")
        } else if language.contains("rust") {
            self.analyzers.get("rust")
        } else {
            None
        }
    }

    fn calculate_severity_score(&self, diagnostic: &Diagnostic) -> f32 {
        match diagnostic.severity {
            DiagnosticSeverity::Error => 100.0,
            DiagnosticSeverity::Warning => 60.0,
            DiagnosticSeverity::Information => 30.0,
            DiagnosticSeverity::Hint => 10.0,
        }
    }

    fn calculate_location_score(&self, diagnostic: &Diagnostic) -> f32 {
        let line = diagnostic.range.start.line;

        // Prioritize errors in earlier parts of files (often more fundamental)
        if line < 50 {
            90.0
        } else if line < 200 {
            70.0
        } else if line < 500 {
            50.0
        } else {
            30.0
        }
    }

    fn calculate_impact_score(&self, group: &DiagnosticGroup, is_cascading: bool) -> f32 {
        let related_count = group.related.len();

        // Base score on number of related errors
        let base_score = match related_count {
            0 => 20.0,
            1..=2 => 40.0,
            3..=5 => 60.0,
            6..=10 => 80.0,
            _ => 100.0,
        };

        // Boost score if it's a cascading error
        if is_cascading {
            (base_score * 1.5_f32).min(100.0)
        } else {
            base_score
        }
    }

    fn calculate_complexity_score(&self, fix_complexity: u8) -> f32 {
        // Lower complexity = higher priority
        match fix_complexity {
            1 => 100.0, // Trivial fix
            2 => 80.0,  // Simple fix
            3 => 60.0,  // Moderate fix
            4 => 40.0,  // Complex fix
            5 => 20.0,  // Very complex fix
            _ => 30.0,
        }
    }

    fn calculate_category_score(&self, category: &crate::analyzers::DiagnosticCategory) -> f32 {
        use crate::analyzers::DiagnosticCategory;

        match category {
            // High priority categories
            DiagnosticCategory::SyntaxError => 100.0,
            DiagnosticCategory::ParseError => 100.0,
            DiagnosticCategory::MissingImport => 90.0,
            DiagnosticCategory::UndefinedVariable => 85.0,

            // Medium priority
            DiagnosticCategory::TypeMismatch => 70.0,
            DiagnosticCategory::MissingProperty => 65.0,
            DiagnosticCategory::BorrowChecker => 60.0,
            DiagnosticCategory::LifetimeError => 55.0,

            // Lower priority
            DiagnosticCategory::UnusedVariable => 40.0,
            DiagnosticCategory::CodeQuality => 35.0,
            DiagnosticCategory::Performance => 30.0,

            // Default
            _ => 50.0,
        }
    }

    /// Get a summary of prioritization results
    pub fn summarize_priorities(
        &self,
        prioritized: &[PrioritizedDiagnostic],
    ) -> PrioritizationSummary {
        let total_diagnostics: usize = prioritized.iter().map(|p| 1 + p.group.related.len()).sum();

        let high_priority = prioritized
            .iter()
            .filter(|p| p.priority_score >= 70.0)
            .count();

        let quick_fixes = prioritized.iter().filter(|p| p.fix_complexity <= 2).count();

        let cascading_errors = prioritized.iter().filter(|p| p.impact_radius > 5).count();

        let avg_priority = if prioritized.is_empty() {
            0.0
        } else {
            prioritized.iter().map(|p| p.priority_score).sum::<f32>() / prioritized.len() as f32
        };

        PrioritizationSummary {
            total_groups: prioritized.len(),
            total_diagnostics,
            high_priority_count: high_priority,
            quick_fix_count: quick_fixes,
            cascading_error_count: cascading_errors,
            average_priority_score: avg_priority,
        }
    }

    /// Get recommended fix order based on prioritization
    pub fn get_fix_order(
        &self,
        prioritized: &[PrioritizedDiagnostic],
        max_items: usize,
    ) -> Vec<FixRecommendation> {
        prioritized
            .iter()
            .take(max_items)
            .enumerate()
            .map(|(index, pd)| {
                let analyzer = self.get_analyzer(&pd.group.primary);
                let suggestions = analyzer
                    .map(|a| a.suggest_fix(&pd.group.primary, None))
                    .unwrap_or_default();

                FixRecommendation {
                    order: index + 1,
                    diagnostic: pd.group.primary.clone(),
                    reason: self.explain_priority(pd),
                    estimated_impact: format!(
                        "Fixing this may resolve {} related errors",
                        pd.impact_radius
                    ),
                    fix_suggestions: suggestions,
                }
            })
            .collect()
    }

    fn explain_priority(&self, pd: &PrioritizedDiagnostic) -> String {
        let mut reasons = Vec::new();

        if pd.score_breakdown.severity_score >= 90.0 {
            reasons.push("Critical error");
        }

        if pd.score_breakdown.impact_score >= 80.0 {
            reasons.push("High impact - affects many related diagnostics");
        }

        if pd.score_breakdown.complexity_score >= 80.0 {
            reasons.push("Quick fix available");
        }

        if pd.score_breakdown.location_score >= 80.0 {
            reasons.push("Early in file - likely foundational issue");
        }

        if reasons.is_empty() {
            reasons.push("Standard priority diagnostic");
        }

        reasons.join(", ")
    }
}

#[derive(Debug, Clone)]
pub struct PrioritizationSummary {
    pub total_groups: usize,
    pub total_diagnostics: usize,
    pub high_priority_count: usize,
    pub quick_fix_count: usize,
    pub cascading_error_count: usize,
    pub average_priority_score: f32,
}

#[derive(Debug, Clone)]
pub struct FixRecommendation {
    pub order: usize,
    pub diagnostic: Diagnostic,
    pub reason: String,
    pub estimated_impact: String,
    pub fix_suggestions: Vec<crate::analyzers::FixSuggestion>,
}

impl Default for DiagnosticPrioritizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Position, Range};
    use uuid::Uuid;

    fn create_test_diagnostic(severity: DiagnosticSeverity, line: u32, source: &str) -> Diagnostic {
        Diagnostic {
            id: Uuid::new_v4().to_string(),
            file: "test.ts".to_string(),
            range: Range {
                start: Position { line, character: 0 },
                end: Position {
                    line,
                    character: 10,
                },
            },
            severity,
            message: "Test error".to_string(),
            code: None,
            source: source.to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    #[test]
    fn test_prioritization_basic() {
        let prioritizer = DiagnosticPrioritizer::new();

        let groups = vec![
            DiagnosticGroup {
                primary: create_test_diagnostic(DiagnosticSeverity::Warning, 100, "typescript"),
                related: vec![],
                confidence: 1.0,
            },
            DiagnosticGroup {
                primary: create_test_diagnostic(DiagnosticSeverity::Error, 10, "typescript"),
                related: vec![
                    create_test_diagnostic(DiagnosticSeverity::Error, 11, "typescript"),
                    create_test_diagnostic(DiagnosticSeverity::Error, 12, "typescript"),
                ],
                confidence: 0.8,
            },
        ];

        let prioritized = prioritizer.prioritize(groups);

        // Error with related diagnostics should be prioritized over warning
        assert_eq!(prioritized.len(), 2);
        assert!(prioritized[0].priority_score > prioritized[1].priority_score);
        assert_eq!(
            prioritized[0].group.primary.severity,
            DiagnosticSeverity::Error
        );
    }

    #[test]
    fn test_fix_order_recommendations() {
        let prioritizer = DiagnosticPrioritizer::new();

        let groups = vec![DiagnosticGroup {
            primary: create_test_diagnostic(DiagnosticSeverity::Error, 5, "typescript"),
            related: vec![],
            confidence: 1.0,
        }];

        let prioritized = prioritizer.prioritize(groups);
        let recommendations = prioritizer.get_fix_order(&prioritized, 5);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].order, 1);
        assert!(!recommendations[0].reason.is_empty());
    }
}
