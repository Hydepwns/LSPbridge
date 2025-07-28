use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ai_training::{FixConfidence, TrainingDataset, TrainingPair};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FixQuality {
    Perfect,    // Fixes all issues correctly
    Good,       // Fixes main issue but may have minor issues
    Acceptable, // Works but not ideal
    Poor,       // Has problems
    Incorrect,  // Doesn't fix the issue
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub training_pair_id: String,
    pub annotator_id: String,
    pub quality: FixQuality,
    pub confidence_adjustment: f32,
    pub notes: String,
    pub tags: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub verification: VerificationResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub compiles: bool,
    pub tests_pass: Option<bool>,
    pub linter_warnings: Vec<String>,
    pub performance_impact: Option<String>,
    pub side_effects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationSession {
    pub id: String,
    pub annotator_id: String,
    pub dataset_id: String,
    pub annotations: Vec<Annotation>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub statistics: SessionStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatistics {
    pub total_reviewed: usize,
    pub quality_distribution: HashMap<String, usize>,
    pub avg_confidence_adjustment: f32,
    pub common_issues: Vec<String>,
    pub time_per_annotation: f32,
}

pub struct AnnotationTool {
    current_session: Option<AnnotationSession>,
    quality_weights: HashMap<FixQuality, f32>,
}

impl AnnotationTool {
    pub fn new() -> Self {
        let mut quality_weights = HashMap::new();
        quality_weights.insert(FixQuality::Perfect, 1.0);
        quality_weights.insert(FixQuality::Good, 0.8);
        quality_weights.insert(FixQuality::Acceptable, 0.6);
        quality_weights.insert(FixQuality::Poor, 0.3);
        quality_weights.insert(FixQuality::Incorrect, 0.0);

        Self {
            current_session: None,
            quality_weights,
        }
    }

    pub fn start_session(&mut self, annotator_id: String, dataset_id: String) -> String {
        let session = AnnotationSession {
            id: uuid::Uuid::new_v4().to_string(),
            annotator_id,
            dataset_id,
            annotations: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
            statistics: SessionStatistics::default(),
        };

        let session_id = session.id.clone();
        self.current_session = Some(session);
        session_id
    }

    pub fn annotate_pair(
        &mut self,
        pair: &mut TrainingPair,
        quality: FixQuality,
        notes: String,
        tags: Vec<String>,
        verification: VerificationResult,
    ) -> Result<Annotation> {
        // Calculate confidence adjustment based on quality (before getting mutable ref)
        let base_adjustment = self.quality_weights.get(&quality).copied().unwrap_or(0.5);
        let verification_adjustment = self.calculate_verification_adjustment(&verification);
        let final_adjustment = (base_adjustment + verification_adjustment) / 2.0;

        let session = self
            .current_session
            .as_mut()
            .context("No active annotation session")?;

        // Update the training pair's confidence
        pair.confidence = FixConfidence::new(pair.confidence.score * final_adjustment);

        // Add quality metadata to the pair
        pair.add_metadata("fix_quality".to_string(), serde_json::json!(quality));
        pair.add_metadata(
            "verified".to_string(),
            serde_json::json!(verification.compiles),
        );

        let annotation = Annotation {
            id: uuid::Uuid::new_v4().to_string(),
            training_pair_id: pair.id.clone(),
            annotator_id: session.annotator_id.clone(),
            quality,
            confidence_adjustment: final_adjustment,
            notes,
            tags,
            timestamp: Utc::now(),
            verification,
        };

        session.annotations.push(annotation.clone());
        self.update_session_statistics();

        Ok(annotation)
    }

    pub fn batch_annotate(
        &mut self,
        dataset: &mut TrainingDataset,
        quality_threshold: FixQuality,
    ) -> Result<Vec<Annotation>> {
        let mut annotations = Vec::new();

        for pair in &mut dataset.pairs {
            // Simulate automatic quality assessment
            let quality = self.assess_quality(pair)?;

            if quality >= quality_threshold {
                let verification = self.verify_fix(pair)?;
                let annotation = self.annotate_pair(
                    pair,
                    quality,
                    "Automated annotation".to_string(),
                    vec!["auto".to_string()],
                    verification,
                )?;
                annotations.push(annotation);
            }
        }

        Ok(annotations)
    }

    pub fn complete_session(&mut self) -> Result<AnnotationSession> {
        let mut session = self
            .current_session
            .take()
            .context("No active annotation session")?;

        session.completed_at = Some(Utc::now());
        self.update_session_statistics();

        Ok(session)
    }

    pub fn get_annotation_report(&self, dataset: &TrainingDataset) -> Result<AnnotationReport> {
        let mut report = AnnotationReport::default();

        for pair in &dataset.pairs {
            if let Some(quality_value) = pair.metadata.get("fix_quality") {
                if let Ok(quality) = serde_json::from_value::<FixQuality>(quality_value.clone()) {
                    *report.quality_distribution.entry(quality).or_insert(0) += 1;
                    report.total_annotated += 1;
                }
            }
        }

        report.calculate_metrics(dataset);
        Ok(report)
    }

    fn calculate_verification_adjustment(&self, verification: &VerificationResult) -> f32 {
        let mut score = 0.5;

        if verification.compiles {
            score += 0.3;
        }

        if let Some(true) = verification.tests_pass {
            score += 0.2;
        }

        // Deduct for warnings and side effects
        score -= (verification.linter_warnings.len() as f32 * 0.05).min(0.2);
        score -= (verification.side_effects.len() as f32 * 0.1).min(0.2);

        score.clamp(0.0, 1.0)
    }

    fn assess_quality(&self, pair: &TrainingPair) -> Result<FixQuality> {
        // Simple heuristic-based quality assessment
        let quality = if pair.diagnostics.is_empty() {
            FixQuality::Perfect
        } else if pair.confidence.score > 0.9 {
            FixQuality::Good
        } else if pair.confidence.score > 0.7 {
            FixQuality::Acceptable
        } else if pair.confidence.score > 0.5 {
            FixQuality::Poor
        } else {
            FixQuality::Incorrect
        };

        Ok(quality)
    }

    fn verify_fix(&self, _pair: &TrainingPair) -> Result<VerificationResult> {
        // In a real implementation, this would compile and test the code
        Ok(VerificationResult {
            compiles: true,
            tests_pass: Some(true),
            linter_warnings: vec![],
            performance_impact: None,
            side_effects: vec![],
        })
    }

    fn update_session_statistics(&mut self) {
        if let Some(session) = &mut self.current_session {
            let mut quality_dist = HashMap::new();
            let mut total_adjustment = 0.0;

            for annotation in &session.annotations {
                let quality_str = format!("{:?}", annotation.quality);
                *quality_dist.entry(quality_str).or_insert(0) += 1;
                total_adjustment += annotation.confidence_adjustment;
            }

            let elapsed = (Utc::now() - session.started_at).num_seconds() as f32;
            let time_per = if session.annotations.is_empty() {
                0.0
            } else {
                elapsed / session.annotations.len() as f32
            };

            session.statistics = SessionStatistics {
                total_reviewed: session.annotations.len(),
                quality_distribution: quality_dist,
                avg_confidence_adjustment: if session.annotations.is_empty() {
                    0.0
                } else {
                    total_adjustment / session.annotations.len() as f32
                },
                common_issues: vec![],
                time_per_annotation: time_per,
            };
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnnotationReport {
    pub total_annotated: usize,
    pub quality_distribution: HashMap<FixQuality, usize>,
    pub confidence_metrics: ConfidenceMetrics,
    pub language_breakdown: HashMap<String, usize>,
    pub diagnostic_type_breakdown: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfidenceMetrics {
    pub avg_original: f32,
    pub avg_adjusted: f32,
    pub improvement_rate: f32,
}

impl AnnotationReport {
    fn calculate_metrics(&mut self, dataset: &TrainingDataset) {
        let mut original_sum = 0.0;
        let adjusted_sum = 0.0;
        let mut count = 0;

        for pair in &dataset.pairs {
            original_sum += pair.confidence.score;
            count += 1;

            self.language_breakdown
                .entry(pair.language.clone())
                .and_modify(|e| *e += 1)
                .or_insert(1);

            for diag in &pair.diagnostics {
                self.diagnostic_type_breakdown
                    .entry(diag.source.clone())
                    .and_modify(|e| *e += 1)
                    .or_insert(1);
            }
        }

        if count > 0 {
            self.confidence_metrics.avg_original = original_sum / count as f32;
            self.confidence_metrics.avg_adjusted = adjusted_sum / count as f32;
            self.confidence_metrics.improvement_rate =
                (self.confidence_metrics.avg_adjusted - self.confidence_metrics.avg_original).abs();
        }
    }
}

impl Default for SessionStatistics {
    fn default() -> Self {
        Self {
            total_reviewed: 0,
            quality_distribution: HashMap::new(),
            avg_confidence_adjustment: 0.0,
            common_issues: vec![],
            time_per_annotation: 0.0,
        }
    }
}

impl Default for AnnotationTool {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialOrd for FixQuality {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let self_value = match self {
            FixQuality::Perfect => 5,
            FixQuality::Good => 4,
            FixQuality::Acceptable => 3,
            FixQuality::Poor => 2,
            FixQuality::Incorrect => 1,
        };

        let other_value = match other {
            FixQuality::Perfect => 5,
            FixQuality::Good => 4,
            FixQuality::Acceptable => 3,
            FixQuality::Poor => 2,
            FixQuality::Incorrect => 1,
        };

        self_value.partial_cmp(&other_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotation_session() {
        let mut tool = AnnotationTool::new();
        let session_id = tool.start_session("test_user".to_string(), "dataset_1".to_string());
        assert!(!session_id.is_empty());

        let mut pair = TrainingPair::new(
            "broken code".to_string(),
            "fixed code".to_string(),
            vec![],
            crate::core::semantic_context::SemanticContext::default(),
            "rust".to_string(),
        );

        let verification = VerificationResult {
            compiles: true,
            tests_pass: Some(true),
            linter_warnings: vec![],
            performance_impact: None,
            side_effects: vec![],
        };

        let result = tool.annotate_pair(
            &mut pair,
            FixQuality::Good,
            "Looks good".to_string(),
            vec!["reviewed".to_string()],
            verification,
        );

        assert!(result.is_ok());
        let annotation = result.unwrap();
        assert_eq!(annotation.quality, FixQuality::Good);
    }
}
