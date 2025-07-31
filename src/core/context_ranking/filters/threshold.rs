use crate::core::context_ranking::types::ContextElement;

pub struct ThresholdFilter;

impl ThresholdFilter {
    /// Filter elements by minimum priority score
    pub fn filter_by_priority(elements: &[ContextElement], min_priority: f32) -> Vec<ContextElement> {
        elements
            .iter()
            .filter(|e| e.priority_score >= min_priority)
            .cloned()
            .collect()
    }

    /// Filter elements by maximum token cost
    pub fn filter_by_token_cost(elements: &[ContextElement], max_tokens: usize) -> Vec<ContextElement> {
        elements
            .iter()
            .filter(|e| e.estimated_tokens <= max_tokens)
            .cloned()
            .collect()
    }

    /// Get threshold recommendations based on element distribution
    pub fn recommend_thresholds(elements: &[ContextElement]) -> ThresholdRecommendations {
        if elements.is_empty() {
            return ThresholdRecommendations::default();
        }

        let mut scores: Vec<f32> = elements.iter().map(|e| e.priority_score).collect();
        scores.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        let percentile_95 = Self::percentile(&scores, 0.95);
        let percentile_80 = Self::percentile(&scores, 0.80);
        let percentile_50 = Self::percentile(&scores, 0.50);

        ThresholdRecommendations {
            essential_threshold: percentile_95.max(0.8),
            supplementary_threshold: percentile_80.max(0.4),
            optional_threshold: percentile_50.max(0.2),
        }
    }

    fn percentile(sorted_values: &[f32], percentile: f32) -> f32 {
        if sorted_values.is_empty() {
            return 0.0;
        }

        let index = ((sorted_values.len() - 1) as f32 * percentile) as usize;
        sorted_values[index]
    }
}

#[derive(Debug, Default)]
pub struct ThresholdRecommendations {
    pub essential_threshold: f32,
    pub supplementary_threshold: f32,
    pub optional_threshold: f32,
}