use crate::core::constants::{languages, lsp_constants};
use crate::core::types::{Diagnostic, DiagnosticSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Confidence score for a fix (0.0 to 1.0)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ConfidenceScore(f32);

impl ConfidenceScore {
    pub fn new(score: f32) -> Self {
        Self(score.clamp(0.0, 1.0))
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    pub fn is_auto_applicable(&self, threshold: &ConfidenceThreshold) -> bool {
        self.0 >= threshold.auto_apply
    }

    pub fn is_suggestable(&self, threshold: &ConfidenceThreshold) -> bool {
        self.0 >= threshold.suggest
    }
}

/// Configurable confidence thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceThreshold {
    /// Minimum confidence to auto-apply a fix (default: 0.9)
    pub auto_apply: f32,
    /// Minimum confidence to suggest a fix (default: 0.5)
    pub suggest: f32,
    /// Never apply fixes below this threshold (default: 0.3)
    pub minimum: f32,
}

impl Default for ConfidenceThreshold {
    fn default() -> Self {
        Self {
            auto_apply: 0.9,
            suggest: 0.5,
            minimum: 0.3,
        }
    }
}

/// Factors that influence confidence scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceFactors {
    /// How well-known is this error pattern (0.0-1.0)
    pub pattern_recognition: f32,
    /// How complex is the fix (0.0-1.0, higher = simpler)
    pub fix_complexity: f32,
    /// Historical success rate for similar fixes (0.0-1.0)
    pub historical_success: f32,
    /// Risk of breaking other code (0.0-1.0, higher = lower risk)
    pub safety_score: f32,
    /// Language-specific confidence (0.0-1.0)
    pub language_confidence: f32,
    /// Whether fix comes from LSP code action (0.0-1.0)
    pub lsp_confidence: f32,
}

/// Fix confidence scorer
pub struct FixConfidenceScorer {
    /// Historical success rates by error pattern
    pattern_success_rates: HashMap<String, f32>,
    /// Language-specific confidence modifiers
    language_modifiers: HashMap<String, f32>,
    /// User-configured thresholds
    thresholds: ConfidenceThreshold,
}

impl FixConfidenceScorer {
    pub fn new() -> Self {
        let mut pattern_success_rates = HashMap::new();
        // Common TypeScript patterns
        pattern_success_rates.insert("TS2322".to_string(), 0.85); // Type assignment
        pattern_success_rates.insert("TS2339".to_string(), 0.75); // Property doesn't exist
        pattern_success_rates.insert("TS2345".to_string(), 0.80); // Argument type mismatch
        pattern_success_rates.insert("TS1005".to_string(), 0.95); // Missing syntax

        // Common Rust patterns
        pattern_success_rates.insert("E0308".to_string(), 0.80); // Type mismatch
        pattern_success_rates.insert("E0384".to_string(), 0.90); // Cannot assign twice
        pattern_success_rates.insert("E0382".to_string(), 0.70); // Use after move
        pattern_success_rates.insert("E0596".to_string(), 0.85); // Cannot borrow as mutable

        let mut language_modifiers = HashMap::new();
        language_modifiers.insert(languages::TYPESCRIPT.to_string(), 0.9);
        language_modifiers.insert(languages::JAVASCRIPT.to_string(), 0.85);
        language_modifiers.insert(languages::RUST.to_string(), 0.95);
        language_modifiers.insert(languages::PYTHON.to_string(), 0.80);
        language_modifiers.insert(languages::GO.to_string(), 0.85);

        Self {
            pattern_success_rates,
            language_modifiers,
            thresholds: ConfidenceThreshold::default(),
        }
    }

    pub fn with_thresholds(mut self, thresholds: ConfidenceThreshold) -> Self {
        self.thresholds = thresholds;
        self
    }

    pub fn score_fix(
        &self,
        diagnostic: &Diagnostic,
        fix_text: &str,
        has_lsp_action: bool,
    ) -> (ConfidenceScore, ConfidenceFactors) {
        let factors = self.calculate_factors(diagnostic, fix_text, has_lsp_action);
        let score = self.calculate_weighted_score(&factors);

        (ConfidenceScore::new(score), factors)
    }

    fn calculate_factors(
        &self,
        diagnostic: &Diagnostic,
        fix_text: &str,
        has_lsp_action: bool,
    ) -> ConfidenceFactors {
        // Pattern recognition score
        let pattern_recognition = if let Some(code) = &diagnostic.code {
            self.pattern_success_rates.get(code).copied().unwrap_or(0.5)
        } else {
            0.3
        };

        // Fix complexity (simple heuristics)
        let fix_complexity = match fix_text.len() {
            0..=20 => 0.9,    // Very simple fix
            21..=50 => 0.8,   // Simple fix
            51..=100 => 0.6,  // Moderate fix
            101..=200 => 0.4, // Complex fix
            _ => 0.2,         // Very complex fix
        };

        // Historical success (would be loaded from persistent storage)
        let historical_success = pattern_recognition; // For now, use pattern rate

        // Safety score based on severity and fix type
        let safety_score = match diagnostic.severity {
            DiagnosticSeverity::Error => 0.7,
            DiagnosticSeverity::Warning => 0.8,
            DiagnosticSeverity::Information => 0.9,
            DiagnosticSeverity::Hint => 0.95,
        };

        // Language confidence
        let language = detect_language_from_file(&diagnostic.file);
        let language_confidence = self
            .language_modifiers
            .get(&language)
            .copied()
            .unwrap_or(0.5);

        // LSP confidence boost
        let lsp_confidence = if has_lsp_action { 0.95 } else { 0.5 };

        ConfidenceFactors {
            pattern_recognition,
            fix_complexity,
            historical_success,
            safety_score,
            language_confidence,
            lsp_confidence,
        }
    }

    fn calculate_weighted_score(&self, factors: &ConfidenceFactors) -> f32 {
        // Weighted average with different importance for each factor
        let weights = [
            (factors.pattern_recognition, 0.25),
            (factors.fix_complexity, 0.15),
            (factors.historical_success, 0.20),
            (factors.safety_score, 0.15),
            (factors.language_confidence, 0.10),
            (factors.lsp_confidence, 0.15),
        ];

        let total_weight: f32 = weights.iter().map(|(_, w)| w).sum();
        let weighted_sum: f32 = weights.iter().map(|(v, w)| v * w).sum();

        weighted_sum / total_weight
    }

    pub fn update_success_rate(&mut self, pattern: &str, success: bool) {
        let current = self
            .pattern_success_rates
            .get(pattern)
            .copied()
            .unwrap_or(0.5);
        // Simple exponential moving average
        let alpha = 0.1;
        let new_rate = if success {
            current * (1.0 - alpha) + alpha
        } else {
            current * (1.0 - alpha)
        };
        self.pattern_success_rates
            .insert(pattern.to_string(), new_rate);
    }
}

impl Default for FixConfidenceScorer {
    fn default() -> Self {
        Self::new()
    }
}

fn detect_language_from_file(file_path: &str) -> String {
    if file_path.ends_with(".ts") || file_path.ends_with(".tsx") {
        languages::TYPESCRIPT.to_string()
    } else if file_path.ends_with(".js") || file_path.ends_with(".jsx") {
        languages::JAVASCRIPT.to_string()
    } else if file_path.ends_with(".rs") {
        languages::RUST.to_string()
    } else if file_path.ends_with(".py") {
        languages::PYTHON.to_string()
    } else if file_path.ends_with(".go") {
        languages::GO.to_string()
    } else {
        lsp_constants::UNKNOWN.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Position, Range};

    #[test]
    fn test_confidence_score_bounds() {
        let score1 = ConfidenceScore::new(1.5);
        assert_eq!(score1.value(), 1.0);

        let score2 = ConfidenceScore::new(-0.5);
        assert_eq!(score2.value(), 0.0);

        let score3 = ConfidenceScore::new(0.75);
        assert_eq!(score3.value(), 0.75);
    }

    #[test]
    fn test_threshold_checks() {
        let score = ConfidenceScore::new(0.85);
        let threshold = ConfidenceThreshold::default();

        assert!(!score.is_auto_applicable(&threshold));
        assert!(score.is_suggestable(&threshold));

        let high_score = ConfidenceScore::new(0.95);
        assert!(high_score.is_auto_applicable(&threshold));
    }

    #[test]
    fn test_confidence_scoring() {
        let scorer = FixConfidenceScorer::new();

        let diagnostic = Diagnostic::new(
            "test.ts".to_string(),
            Range {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 1,
                    character: 10,
                },
            },
            DiagnosticSeverity::Error,
            "Type 'string' is not assignable to type 'number'".to_string(),
            languages::TYPESCRIPT.to_string(),
        );

        let (score, factors) = scorer.score_fix(&diagnostic, "number", true);

        assert!(score.value() > 0.5); // Should have decent confidence
        assert!(factors.lsp_confidence > 0.9); // LSP action should boost confidence
    }
}
