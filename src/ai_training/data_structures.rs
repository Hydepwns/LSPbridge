use crate::core::semantic_context::SemanticContext;
use crate::core::types::Diagnostic;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPair {
    pub id: String,
    pub before_code: String,
    pub after_code: String,
    pub diagnostics: Vec<Diagnostic>,
    pub context: SemanticContext,
    pub fix_description: String,
    pub confidence: FixConfidence,
    pub language: String,
    pub file_path: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct FixConfidence {
    pub score: f32, // 0.0 to 1.0
    pub category: ConfidenceCategory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ConfidenceCategory {
    Certain,   // 0.9-1.0: Safe to auto-apply
    Probable,  // 0.7-0.9: Likely correct
    Possible,  // 0.5-0.7: May be correct
    Uncertain, // 0.0-0.5: Needs review
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDataset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub pairs: Vec<TrainingPair>,
    pub statistics: DatasetStatistics,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStatistics {
    pub total_pairs: usize,
    pub languages: HashMap<String, usize>,
    pub diagnostic_types: HashMap<String, usize>,
    pub confidence_distribution: HashMap<String, usize>,
    pub avg_context_size: usize,
    pub difficulty_levels: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticPattern {
    pub pattern_id: String,
    pub diagnostic_type: String,
    pub language: String,
    pub error_pattern: String,
    pub fix_pattern: String,
    pub examples: Vec<TrainingPair>,
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualFix {
    pub diagnostic: Diagnostic,
    pub original_code: String,
    pub fixed_code: String,
    pub context: SemanticContext,
    pub dependencies_updated: Vec<String>,
    pub side_effects: Vec<String>,
    pub verification_status: VerificationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationStatus {
    NotVerified,
    CompileSuccess,
    TestsPass,
    ManuallyVerified,
    AutoVerified,
}

impl FixConfidence {
    pub fn new(score: f32) -> Self {
        let category = match score {
            s if s >= 0.9 => ConfidenceCategory::Certain,
            s if s >= 0.7 => ConfidenceCategory::Probable,
            s if s >= 0.5 => ConfidenceCategory::Possible,
            _ => ConfidenceCategory::Uncertain,
        };

        Self { score, category }
    }

    pub fn is_auto_applicable(&self) -> bool {
        self.category == ConfidenceCategory::Certain
    }
}

impl TrainingPair {
    pub fn new(
        before_code: String,
        after_code: String,
        diagnostics: Vec<Diagnostic>,
        context: SemanticContext,
        language: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            before_code,
            after_code,
            diagnostics,
            context,
            fix_description: String::new(),
            confidence: FixConfidence::new(0.5),
            language,
            file_path: String::new(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = FixConfidence::new(confidence);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.fix_description = description;
        self
    }

    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }
}

impl TrainingDataset {
    pub fn new(name: String, description: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            pairs: Vec::new(),
            statistics: DatasetStatistics::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn add_pair(&mut self, pair: TrainingPair) {
        self.pairs.push(pair);
        self.update_statistics();
        self.updated_at = Utc::now();
    }

    pub fn update_statistics(&mut self) {
        let mut languages = HashMap::new();
        let mut diagnostic_types = HashMap::new();
        let mut confidence_dist = HashMap::new();
        let mut total_context_size = 0;

        for pair in &self.pairs {
            *languages.entry(pair.language.clone()).or_insert(0) += 1;

            for diag in &pair.diagnostics {
                *diagnostic_types.entry(diag.source.clone()).or_insert(0) += 1;
            }

            let conf_key = format!("{:?}", pair.confidence.category);
            *confidence_dist.entry(conf_key).or_insert(0) += 1;

            total_context_size += pair.context.surrounding_code.len();
        }

        self.statistics = DatasetStatistics {
            total_pairs: self.pairs.len(),
            languages,
            diagnostic_types,
            confidence_distribution: confidence_dist,
            avg_context_size: if self.pairs.is_empty() {
                0
            } else {
                total_context_size / self.pairs.len()
            },
            difficulty_levels: HashMap::new(),
        };
    }

    pub fn filter_by_confidence(&self, min_confidence: f32) -> Vec<&TrainingPair> {
        self.pairs
            .iter()
            .filter(|pair| pair.confidence.score >= min_confidence)
            .collect()
    }

    pub fn filter_by_language(&self, language: &str) -> Vec<&TrainingPair> {
        self.pairs
            .iter()
            .filter(|pair| pair.language == language)
            .collect()
    }
}

impl Default for DatasetStatistics {
    fn default() -> Self {
        Self {
            total_pairs: 0,
            languages: HashMap::new(),
            diagnostic_types: HashMap::new(),
            confidence_distribution: HashMap::new(),
            avg_context_size: 0,
            difficulty_levels: HashMap::new(),
        }
    }
}
