use crate::core::{Diagnostic, SemanticContext};
use regex::Regex;

pub struct TypeInferenceHelper;

impl TypeInferenceHelper {
    pub fn new() -> Self {
        Self
    }

    /// Infer property type from diagnostic context
    pub fn infer_property_type(
        &self,
        diagnostic: &Diagnostic,
        property: &str,
        context: Option<&SemanticContext>,
    ) -> String {
        // First, try to extract type from the diagnostic message itself
        // Many TypeScript errors include type information
        let message = &diagnostic.message;
        
        // Pattern: "Property 'x' is missing in type '{...}' but required in type '{...}'"
        if let Some(cap) = Regex::new(&format!(r#"['"]{}"['"].*?:\s*([^;,}}]+)"#, regex::escape(property)))
            .ok()
            .and_then(|re| re.captures(message))
        {
            if let Some(type_match) = cap.get(1) {
                let type_str = type_match.as_str().trim();
                if !type_str.is_empty() && type_str != "any" {
                    return type_str.to_string();
                }
            }
        }
        
        // Try to infer from usage context
        if let Some(ctx) = context {
            // Check if property is used in function context
            if let Some(func_ctx) = &ctx.function_context {
                // Look for patterns like "property = value" or "property: type"
                if let Some(cap) = Regex::new(&format!(r"{}\s*[=:]\s*([^;,\n]+)", regex::escape(property)))
                    .ok()
                    .and_then(|re| re.captures(&func_ctx.body))
                {
                    if let Some(value_match) = cap.get(1) {
                        let value = value_match.as_str().trim();
                        return self.infer_type_from_value(value);
                    }
                }
            }
        }
        
        // Common property name patterns
        match property {
            s if s.ends_with("Id") || s.ends_with("ID") => "string".to_string(),
            s if s.starts_with("is") || s.starts_with("has") || s.starts_with("should") => "boolean".to_string(),
            s if s.ends_with("Count") || s.ends_with("Index") || s.ends_with("Size") => "number".to_string(),
            s if s.ends_with("Date") || s.ends_with("Time") => "Date".to_string(),
            s if s.ends_with("s") && !s.ends_with("ss") => "unknown[]".to_string(), // Likely plural
            _ => "unknown".to_string(),
        }
    }
    
    /// Infer type from a value expression
    pub fn infer_type_from_value(&self, value: &str) -> String {
        let trimmed = value.trim();
        
        // String literals
        if (trimmed.starts_with('"') && trimmed.ends_with('"')) ||
           (trimmed.starts_with('\'') && trimmed.ends_with('\'')) ||
           (trimmed.starts_with('`') && trimmed.ends_with('`')) {
            return "string".to_string();
        }
        
        // Boolean literals
        if trimmed == "true" || trimmed == "false" {
            return "boolean".to_string();
        }
        
        // Number literals
        if trimmed.parse::<f64>().is_ok() {
            return "number".to_string();
        }
        
        // Array literals
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            return "unknown[]".to_string();
        }
        
        // Object literals
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            return "Record<string, unknown>".to_string();
        }
        
        // Function expressions
        if trimmed.contains("=>") || trimmed.starts_with("function") {
            return "Function".to_string();
        }
        
        // null/undefined
        if trimmed == "null" {
            return "null".to_string();
        }
        if trimmed == "undefined" {
            return "undefined".to_string();
        }
        
        // Default fallback
        "unknown".to_string()
    }
}