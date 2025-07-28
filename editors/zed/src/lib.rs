use std::fs;
use std::process::Command;
use zed_extension_api::{self as zed, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct ExportConfig {
    format: String,
    privacy_level: String,
    include_context: bool,
    context_lines: usize,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: "claude".to_string(),
            privacy_level: "default".to_string(),
            include_context: true,
            context_lines: 3,
        }
    }
}

struct LspBridgeExtension {
    config: ExportConfig,
}

impl LspBridgeExtension {
    fn new() -> Self {
        Self {
            config: ExportConfig::default(),
        }
    }

    fn export_diagnostics(&self, args: Vec<String>) -> Result<String> {
        let mut cmd = Command::new("lsp-bridge");
        cmd.arg("export")
            .arg("--format").arg(&self.config.format)
            .arg("--privacy").arg(&self.config.privacy_level);

        if self.config.include_context {
            cmd.arg("--include-context")
                .arg("--context-lines").arg(self.config.context_lines.to_string());
        }

        // Add any additional arguments
        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd.output()
            .map_err(|e| format!("Failed to run lsp-bridge: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!("lsp-bridge failed: {}", 
                String::from_utf8_lossy(&output.stderr)).into())
        }
    }

    fn apply_quick_fixes(&self, threshold: f32) -> Result<String> {
        let output = Command::new("lsp-bridge")
            .arg("quick-fix")
            .arg("apply")
            .arg("--threshold").arg(threshold.to_string())
            .output()
            .map_err(|e| format!("Failed to run lsp-bridge: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!("lsp-bridge quick-fix failed: {}", 
                String::from_utf8_lossy(&output.stderr)).into())
        }
    }

    fn show_history(&self) -> Result<String> {
        let output = Command::new("lsp-bridge")
            .arg("history")
            .arg("trends")
            .arg("--format").arg("json")
            .output()
            .map_err(|e| format!("Failed to run lsp-bridge: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!("lsp-bridge history failed: {}", 
                String::from_utf8_lossy(&output.stderr)).into())
        }
    }
}

#[export]
pub fn init_extension() -> Result<()> {
    let extension = LspBridgeExtension::new();
    
    // Register commands
    zed::register_command("lsp-bridge.export", move |_workspace| {
        let result = extension.export_diagnostics(vec![])?;
        
        // Save to file
        let output_path = zed::prompt_for_save_path("Save diagnostics", "diagnostics.md")?;
        fs::write(&output_path, result)?;
        
        zed::show_message(&format!("Diagnostics exported to {:?}", output_path));
        Ok(())
    });

    zed::register_command("lsp-bridge.export-clipboard", move |_workspace| {
        let result = extension.export_diagnostics(vec![])?;
        
        zed::set_clipboard_text(&result)?;
        zed::show_message("Diagnostics copied to clipboard");
        Ok(())
    });

    zed::register_command("lsp-bridge.show-history", move |_workspace| {
        let history = extension.show_history()?;
        
        // Parse and display history
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&history) {
            let health_score = data["health_score"].as_f64().unwrap_or(0.0) * 100.0;
            let error_velocity = data["error_velocity"].as_f64().unwrap_or(0.0);
            
            let message = format!(
                "Health Score: {:.0}%\nError Velocity: {:.1} errors/hour",
                health_score, error_velocity
            );
            
            zed::show_message(&message);
        }
        Ok(())
    });

    zed::register_command("lsp-bridge.apply-fixes", move |_workspace| {
        // First do a dry run
        let dry_run = Command::new("lsp-bridge")
            .arg("quick-fix")
            .arg("apply")
            .arg("--dry-run")
            .arg("--threshold").arg("0.9")
            .output()
            .map_err(|e| format!("Failed to run lsp-bridge: {}", e))?;

        if !dry_run.status.success() {
            zed::show_error("Failed to analyze quick fixes");
            return Ok(());
        }

        let dry_run_output = String::from_utf8_lossy(&dry_run.stdout);
        
        // Count available fixes
        let fix_count = dry_run_output.lines()
            .filter(|line| line.contains("Would fix:"))
            .count();

        if fix_count == 0 {
            zed::show_message("No fixes available with sufficient confidence");
            return Ok(());
        }

        // Ask user for confirmation
        let confirmed = zed::confirm(&format!(
            "Apply {} fixes with confidence >= 0.9?", 
            fix_count
        ))?;

        if confirmed {
            let result = extension.apply_quick_fixes(0.9)?;
            zed::show_message(&format!("Applied fixes: {}", result));
        }
        
        Ok(())
    });

    // Register configuration
    zed::register_setting("lsp-bridge.format", "claude", |value| {
        // Update config when setting changes
        Ok(())
    });

    zed::register_setting("lsp-bridge.privacy", "default", |value| {
        Ok(())
    });

    zed::register_setting("lsp-bridge.include_context", "true", |value| {
        Ok(())
    });

    Ok(())
}

// Status bar integration
#[export]
pub fn status_bar_item() -> Result<zed::StatusBarItem> {
    // Get current diagnostic counts
    let diagnostics = zed::get_workspace_diagnostics()?;
    let error_count = diagnostics.iter()
        .filter(|d| d.severity == zed::DiagnosticSeverity::Error)
        .count();
    let warning_count = diagnostics.iter()
        .filter(|d| d.severity == zed::DiagnosticSeverity::Warning)
        .count();

    let text = if error_count > 0 {
        format!("🔴 {} 🟡 {}", error_count, warning_count)
    } else if warning_count > 0 {
        format!("🟡 {}", warning_count)
    } else {
        "✅".to_string()
    };

    Ok(zed::StatusBarItem {
        text,
        tooltip: Some("Click to export diagnostics".to_string()),
        on_click: Some("lsp-bridge.export".to_string()),
    })
}

// Context menu integration
#[export]
pub fn context_menu_items() -> Vec<zed::MenuItem> {
    vec![
        zed::MenuItem {
            label: "Export Diagnostics".to_string(),
            command: "lsp-bridge.export".to_string(),
            when: Some("has_diagnostics".to_string()),
        },
        zed::MenuItem {
            label: "Copy Diagnostics to Clipboard".to_string(),
            command: "lsp-bridge.export-clipboard".to_string(),
            when: Some("has_diagnostics".to_string()),
        },
    ]
}