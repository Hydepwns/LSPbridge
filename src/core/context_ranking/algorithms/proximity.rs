use crate::core::semantic_context::VariableContext;
use crate::core::types::Diagnostic;

pub struct ProximityScorer;

impl ProximityScorer {
    /// Calculate proximity score based on distance from diagnostic location
    pub fn score_by_distance(item_line: u32, diagnostic_line: u32, max_distance: u32) -> f32 {
        let distance = (item_line as i32 - diagnostic_line as i32).abs() as u32;
        
        if distance == 0 {
            1.0
        } else if distance <= max_distance {
            // Linear decay based on distance
            1.0 - (distance as f32 / max_distance as f32) * 0.5
        } else {
            0.5 // Base score for items outside proximity range
        }
    }

    /// Score variable relevance based on scope proximity
    pub fn score_variable_proximity(var: &VariableContext, diagnostic: &Diagnostic) -> f32 {
        let diagnostic_line = diagnostic.range.start.line;
        let distance = (var.line as i32 - diagnostic_line as i32).abs();
        
        match distance {
            0..=3 => 1.0,   // Very close - same or adjacent lines
            4..=10 => 0.8,  // Close - within same code block
            11..=30 => 0.6, // Moderate - within same function typically
            31..=50 => 0.4, // Far - possibly in same scope
            _ => 0.2,       // Very far - minimal relevance
        }
    }

    /// Score based on whether item is before or after diagnostic
    pub fn score_by_direction(item_line: u32, diagnostic_line: u32) -> f32 {
        if item_line < diagnostic_line {
            // Items before the error (like variable declarations) are often more relevant
            1.1
        } else if item_line == diagnostic_line {
            1.0
        } else {
            // Items after might be less relevant
            0.9
        }
    }
}