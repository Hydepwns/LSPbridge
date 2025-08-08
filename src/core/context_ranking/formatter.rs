use crate::core::context_ranking::types::{ContextContent, ContextElement, RankedContext};

/// Generate a formatted context summary for AI consumption
pub fn format_context_for_ai(ranked_context: &RankedContext) -> String {
    let mut output = String::new();

    output.push_str("# Code Context for Error Analysis\n\n");

    // Essential context
    if !ranked_context.budget_context.essential_context.is_empty() {
        output.push_str("## Essential Context\n\n");
        for element in &ranked_context.budget_context.essential_context {
            output.push_str(&format_context_element(element));
            output.push('\n');
        }
    }

    // Supplementary context
    if !ranked_context
        .budget_context
        .supplementary_context
        .is_empty()
    {
        output.push_str("## Additional Context\n\n");
        for element in &ranked_context.budget_context.supplementary_context {
            output.push_str(&format_context_element(element));
            output.push('\n');
        }
    }

    // Budget summary
    output.push_str(&format!(
        "## Context Summary\n- Tokens used: {}/{}\n- Elements included: {}\n- Elements excluded: {}\n",
        ranked_context.budget_context.tokens_used,
        ranked_context.budget_context.tokens_used + ranked_context.budget_context.tokens_remaining,
        ranked_context.budget_context.essential_context.len() + ranked_context.budget_context.supplementary_context.len(),
        ranked_context.budget_context.excluded_context.len()
    ));

    output
}

fn format_context_element(element: &ContextElement) -> String {
    let mut output = String::new();

    match &element.content {
        ContextContent::Function(func) => {
            output.push_str(&format!("### Function: {}\n", func.name));
            output.push_str(&format!("**Signature:** `{}`\n", func.signature));
            output.push_str(&format!(
                "**Lines:** {}-{}\n",
                func.start_line, func.end_line
            ));
            output.push_str(&format!(
                "**Relevance:** {}\n",
                element.relevance_explanation
            ));
            output.push_str("```\n");
            output.push_str(&func.body);
            output.push_str("\n```\n");
        }
        ContextContent::Class(class) => {
            output.push_str(&format!("### Class: {}\n", class.name));
            output.push_str(&format!(
                "**Lines:** {}-{}\n",
                class.start_line, class.end_line
            ));
            output.push_str(&format!(
                "**Relevance:** {}\n",
                element.relevance_explanation
            ));
            output.push_str("```\n");
            output.push_str(&class.definition);
            output.push_str("\n```\n");
        }
        ContextContent::Import(import) => {
            output.push_str("### Import\n");
            output.push_str(&format!("**Statement:** `{}`\n", import.statement));
            output.push_str(&format!("**Source:** {}\n", import.source));
            output.push_str(&format!(
                "**Symbols:** {}\n",
                import.imported_names.join(", ")
            ));
            output.push_str(&format!(
                "**Relevance:** {}\n",
                element.relevance_explanation
            ));
        }
        ContextContent::Type(type_def) => {
            output.push_str(&format!("### Type: {}\n", type_def.name));
            output.push_str("**Kind:** Type\n");
            output.push_str(&format!(
                "**Relevance:** {}\n",
                element.relevance_explanation
            ));
            output.push_str("```\n");
            output.push_str(&type_def.definition);
            output.push_str("\n```\n");
        }
        ContextContent::Variable(var) => {
            output.push_str(&format!("### Variable: {}\n", var.name));
            if let Some(type_annotation) = &var.type_annotation {
                output.push_str(&format!("**Type:** {type_annotation}\n"));
            }
            if let Some(init) = &var.value {
                output.push_str(&format!("**Initial value:** {init}\n"));
            }
            output.push_str(&format!(
                "**Relevance:** {}\n",
                element.relevance_explanation
            ));
        }
        ContextContent::Calls(calls) => {
            output.push_str("### Call Hierarchy\n");
            if !calls.callees.is_empty() {
                output.push_str("**Outgoing calls:**\n");
                for call in &calls.callees {
                    output.push_str(&format!(
                        "- {} (line {})\n",
                        call.function_name, call.line
                    ));
                }
            }
            if !calls.callers.is_empty() {
                output.push_str("**Incoming calls:**\n");
                for call in &calls.callers {
                    output.push_str(&format!(
                        "- {} (line {})\n",
                        call.function_name, call.line
                    ));
                }
            }
            output.push_str(&format!(
                "**Relevance:** {}\n",
                element.relevance_explanation
            ));
        }
        ContextContent::Dependency(dep) => {
            output.push_str("### Dependency\n");
            output.push_str(&format!("**File:** {}\n", dep.file_path));
            output.push_str(&format!("**Type:** {:?}\n", dep.dependency_type));
            output.push_str(&format!("**Symbols:** {}\n", dep.imported_symbols.join(", ")));
            output.push_str(&format!(
                "**Relevance:** {}\n",
                element.relevance_explanation
            ));
        }
    }

    output
}