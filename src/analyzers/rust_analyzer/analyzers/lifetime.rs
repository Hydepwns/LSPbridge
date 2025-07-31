use crate::analyzers::base::AnalyzerBase;
use crate::analyzers::language_analyzer::{DiagnosticAnalysis, DiagnosticCategory};
use crate::core::{Diagnostic, SemanticContext};
use regex::Regex;

pub struct LifetimeAnalyzer;

impl AnalyzerBase for LifetimeAnalyzer {}

impl LifetimeAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_lifetime_error(
        &self,
        diagnostic: &Diagnostic,
        _context: Option<&SemanticContext>,
    ) -> DiagnosticAnalysis {
        let identifiers = self.extract_identifiers(&diagnostic.message);
        let mut analysis = self.create_analysis(
            DiagnosticCategory::LifetimeError,
            0.85,
            4,
            "Lifetime constraint violation".to_string(),
            identifiers,
        );

        if diagnostic.message.contains("lifetime parameters")
            || diagnostic.message.contains("explicit lifetime")
        {
            self.add_insight(&mut analysis, "Add explicit lifetime parameters");
            self.add_insight(&mut analysis, "Example: fn foo<'a>(x: &'a str) -> &'a str");
        } else if diagnostic.message.contains("outlives") {
            self.add_insight(
                &mut analysis,
                "Add lifetime bounds to ensure proper outlives relationships",
            );
            self.add_insight(&mut analysis, "Example: where 'a: 'b");
        } else if diagnostic.message.contains("static lifetime") {
            self.add_insight(
                &mut analysis,
                "Consider if 'static lifetime is really needed",
            );
            self.add_insight(&mut analysis, "Or use Arc/Rc for shared ownership");
        }

        // Extract lifetime names
        let lifetime_pattern = Regex::new(r"'(\w+)").unwrap();
        let mut lifetimes = Vec::with_capacity(3); // Most lifetime errors involve 1-3 lifetimes
        for cap in lifetime_pattern.captures_iter(&diagnostic.message) {
            if let Some(lt) = cap.get(1) {
                lifetimes.push(lt.as_str().to_string());
            }
        }
        if !lifetimes.is_empty() {
            analysis
                .insights
                .push(format!("Lifetimes involved: {}", lifetimes.join(", ")));
        }

        analysis
    }
}