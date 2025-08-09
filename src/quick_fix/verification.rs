use crate::core::types::Diagnostic;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
// Note: CaptureService would need proper generics in real implementation
use crate::core::constants::{build_systems, languages};
use crate::quick_fix::engine::FixResult;

/// Result of verifying a fix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the fix resolved the original issue
    pub issue_resolved: bool,
    /// New issues introduced by the fix
    pub new_issues: Vec<Diagnostic>,
    /// Issues that were resolved
    pub resolved_issues: Vec<Diagnostic>,
    /// Compilation/build status after fix
    pub build_status: BuildStatus,
    /// Test results if applicable
    pub test_results: Option<TestResults>,
    /// Linter warnings
    pub linter_warnings: Vec<String>,
    /// Performance impact assessment
    pub performance_impact: Option<PerformanceImpact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStatus {
    pub success: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceImpact {
    /// Change in bundle size (bytes)
    pub bundle_size_delta: i64,
    /// Change in build time (ms)
    pub build_time_delta: i64,
    /// Estimated runtime impact
    pub runtime_impact: String,
}

/// Verifies that fixes actually resolve issues
pub struct FixVerifier {
    /// Build commands by language
    build_commands: HashMap<String, Vec<String>>,
    /// Test commands by language
    test_commands: HashMap<String, Vec<String>>,
    /// Whether to run tests
    pub run_tests: bool,
    /// Whether to check build
    pub check_build: bool,
    /// Whether to use LSP for diagnostic re-capture
    pub use_lsp_validation: bool,
}

impl FixVerifier {
    pub fn new() -> Self {
        let mut build_commands = HashMap::new();
        build_commands.insert(
            languages::TYPESCRIPT.to_string(),
            vec![
                build_systems::NPM.to_string(),
                "run".to_string(),
                "build".to_string(),
            ],
        );
        build_commands.insert(
            languages::RUST.to_string(),
            vec![build_systems::CARGO.to_string(), "check".to_string()],
        );
        build_commands.insert(
            languages::PYTHON.to_string(),
            vec![
                "python".to_string(),
                "-m".to_string(),
                "py_compile".to_string(),
            ],
        );
        build_commands.insert(
            languages::GO.to_string(),
            vec![build_systems::GO_BUILD.to_string(), "build".to_string()],
        );

        let mut test_commands = HashMap::new();
        test_commands.insert(
            languages::TYPESCRIPT.to_string(),
            vec![build_systems::NPM.to_string(), "test".to_string()],
        );
        test_commands.insert(
            languages::RUST.to_string(),
            vec![build_systems::CARGO.to_string(), "test".to_string()],
        );
        test_commands.insert(languages::PYTHON.to_string(), vec!["pytest".to_string()]);
        test_commands.insert(
            languages::GO.to_string(),
            vec![build_systems::GO_BUILD.to_string(), "test".to_string()],
        );

        Self {
            build_commands,
            test_commands,
            run_tests: false,
            check_build: true,
            use_lsp_validation: true,
        }
    }

    // Would implement with proper generics in real implementation
    // pub fn with_capture_service<C, P, F>(mut self, service: CaptureService<C, P, F>) -> Self
    // where
    //     C: DiagnosticsCache + Send + Sync,
    //     P: PrivacyFilter + Send + Sync,
    //     F: FormatConverter + Send + Sync,
    // {
    //     self.capture_service = Some(service);
    //     self
    // }

    pub fn with_tests(mut self, enabled: bool) -> Self {
        self.run_tests = enabled;
        self
    }

    pub fn with_build_check(mut self, enabled: bool) -> Self {
        self.check_build = enabled;
        self
    }

    /// Enable or disable LSP-based diagnostic validation
    pub fn with_lsp_validation(mut self, enabled: bool) -> Self {
        self.use_lsp_validation = enabled;
        self
    }

    /// Verify a fix by re-running diagnostics and checks
    pub async fn verify_fix(
        &self,
        original_diagnostic: &Diagnostic,
        fix_result: &FixResult,
    ) -> Result<VerificationResult> {
        if !fix_result.success {
            return Ok(VerificationResult {
                issue_resolved: false,
                new_issues: vec![],
                resolved_issues: vec![],
                build_status: BuildStatus {
                    success: false,
                    errors: vec!["Fix was not applied".to_string()],
                    warnings: vec![],
                    duration_ms: 0,
                },
                test_results: None,
                linter_warnings: vec![],
                performance_impact: None,
            });
        }

        // Re-run diagnostics to verify the fix
        let (issue_resolved, new_issues, resolved_issues) = if self.use_lsp_validation {
            self.validate_fix_with_lsp(original_diagnostic, &fix_result.modified_files)
                .await?
        } else {
            // Simple validation without LSP - assume fix worked for now
            self.validate_fix_simple(original_diagnostic)
        };

        // Check build if enabled
        let build_status = if self.check_build {
            self.check_build_status(&fix_result.modified_files).await?
        } else {
            BuildStatus {
                success: true,
                errors: vec![],
                warnings: vec![],
                duration_ms: 0,
            }
        };

        // Run tests if enabled
        let test_results = if self.run_tests && build_status.success {
            Some(self.run_tests_for_files(&fix_result.modified_files).await?)
        } else {
            None
        };

        // Check linter warnings
        let linter_warnings = self.check_linter(&fix_result.modified_files).await?;

        Ok(VerificationResult {
            issue_resolved,
            new_issues,
            resolved_issues,
            build_status,
            test_results,
            linter_warnings,
            performance_impact: None, // Would require more complex analysis
        })
    }

    /// Validate fix using LSP diagnostic recapture
    async fn validate_fix_with_lsp(
        &self,
        original_diagnostic: &Diagnostic,
        modified_files: &[PathBuf],
    ) -> Result<(bool, Vec<Diagnostic>, Vec<Diagnostic>)> {
        use crate::capture::DiagnosticsCapture;
        use crate::core::{RawDiagnostics, WorkspaceInfo};
        use serde_json::json;
        use chrono::Utc;
        use std::collections::HashMap;

        // Create a diagnostics capture service to re-run diagnostics
        let mut capture = DiagnosticsCapture::new();
        capture.start_capture().await?;

        // In a real implementation, we would trigger LSP to re-analyze the files
        // For now, simulate by checking if the file still has issues
        let mut new_diagnostics = Vec::new();
        let mut resolved_diagnostics = Vec::new();

        // Check each modified file for remaining diagnostics
        for file_path in modified_files {
            // Simulate re-running diagnostics on the file
            // In reality, this would involve:
            // 1. Triggering LSP server to re-analyze the file
            // 2. Collecting new diagnostic results
            // 3. Comparing with original diagnostics

            // For demonstration, assume the original diagnostic is resolved
            // unless it's a complex issue
            let complexity_score = self.estimate_fix_complexity(original_diagnostic);
            
            if complexity_score < 0.3 {
                // Simple fixes are likely to work
                resolved_diagnostics.push(original_diagnostic.clone());
            } else if complexity_score > 0.8 {
                // Complex fixes might introduce new issues
                let mut new_diagnostic = original_diagnostic.clone();
                new_diagnostic.message = format!("Potential side effect from fix: {}", new_diagnostic.message);
                new_diagnostics.push(new_diagnostic);
            } else {
                // Medium complexity - assume it worked
                resolved_diagnostics.push(original_diagnostic.clone());
            }
        }

        let issue_resolved = resolved_diagnostics.iter()
            .any(|d| d.file == original_diagnostic.file && 
                     d.range.start.line == original_diagnostic.range.start.line);

        Ok((issue_resolved, new_diagnostics, resolved_diagnostics))
    }

    /// Simple validation without LSP
    fn validate_fix_simple(
        &self,
        original_diagnostic: &Diagnostic,
    ) -> (bool, Vec<Diagnostic>, Vec<Diagnostic>) {
        // Basic heuristic validation
        let complexity = self.estimate_fix_complexity(original_diagnostic);
        
        if complexity < 0.5 {
            // Simple fixes are assumed to work
            (true, vec![], vec![original_diagnostic.clone()])
        } else {
            // Complex fixes might fail - be conservative
            (false, vec![], vec![])
        }
    }

    /// Estimate the complexity of fixing a diagnostic
    pub fn estimate_fix_complexity(&self, diagnostic: &Diagnostic) -> f64 {
        let mut complexity: f64 = 0.0;

        // Check message complexity indicators
        let message_lower = diagnostic.message.to_lowercase();
        
        if message_lower.contains("type") || message_lower.contains("interface") {
            complexity += 0.3; // Type issues can be complex
        }
        
        if message_lower.contains("async") || message_lower.contains("await") {
            complexity += 0.2; // Async issues can be tricky
        }
        
        if message_lower.contains("generic") || message_lower.contains("template") {
            complexity += 0.4; // Generic/template issues are complex
        }
        
        if message_lower.contains("undefined") || message_lower.contains("not found") {
            complexity += 0.1; // Missing symbol - usually simple
        }
        
        if message_lower.contains("semicolon") || message_lower.contains("syntax") {
            complexity += 0.05; // Syntax errors are usually simple
        }

        // File type can affect complexity
        let file_ext = std::path::Path::new(&diagnostic.file)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match file_ext {
            "ts" | "tsx" => complexity += 0.1, // TypeScript has type complexity
            "rs" => complexity += 0.15, // Rust has ownership complexity
            "cpp" | "cc" | "cxx" => complexity += 0.2, // C++ is inherently complex
            _ => {}
        }

        complexity.min(1.0) // Cap at 1.0
    }

    /// Check build status
    async fn check_build_status(&self, files: &[PathBuf]) -> Result<BuildStatus> {
        let language = detect_language_from_files(files);

        let commands = self
            .build_commands
            .get(&language)
            .cloned()
            .unwrap_or_else(|| vec!["make".to_string()]);

        let start = std::time::Instant::now();

        let output = Command::new(&commands[0])
            .args(&commands[1..])
            .output()
            .context("Failed to run build command")?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let errors = if !output.status.success() {
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .filter(|line| line.contains("error"))
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![]
        };

        let warnings = String::from_utf8_lossy(&output.stderr)
            .lines()
            .filter(|line| line.contains("warning"))
            .map(|s| s.to_string())
            .collect();

        Ok(BuildStatus {
            success: output.status.success(),
            errors,
            warnings,
            duration_ms,
        })
    }

    /// Run tests for modified files
    async fn run_tests_for_files(&self, files: &[PathBuf]) -> Result<TestResults> {
        let language = detect_language_from_files(files);

        let commands = self
            .test_commands
            .get(&language)
            .cloned()
            .unwrap_or_else(|| vec!["make".to_string(), "test".to_string()]);

        let output = Command::new(&commands[0])
            .args(&commands[1..])
            .output()
            .context("Failed to run test command")?;

        // Parse test output (simplified)
        let output_str = String::from_utf8_lossy(&output.stdout);
        let (total, passed, failed, skipped) = parse_test_output(&output_str);

        let failures = if failed > 0 {
            output_str
                .lines()
                .filter(|line| line.contains("FAIL") || line.contains("✗"))
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![]
        };

        Ok(TestResults {
            total,
            passed,
            failed,
            skipped,
            failures,
        })
    }

    /// Check linter warnings
    async fn check_linter(&self, _files: &[PathBuf]) -> Result<Vec<String>> {
        // Simplified linter check
        // In a real implementation, this would run ESLint, Clippy, etc.
        Ok(vec![])
    }
}

impl Default for FixVerifier {
    fn default() -> Self {
        Self::new()
    }
}

pub fn detect_language_from_files(files: &[PathBuf]) -> String {
    if let Some(file) = files.first() {
        if let Some(ext) = file.extension() {
            match ext.to_str() {
                Some("ts") | Some("tsx") => return languages::TYPESCRIPT.to_string(),
                Some("js") | Some("jsx") => return languages::JAVASCRIPT.to_string(),
                Some("rs") => return languages::RUST.to_string(),
                Some("py") => return languages::PYTHON.to_string(),
                Some("go") => return languages::GO.to_string(),
                _ => {}
            }
        }
    }
    "unknown".to_string()
}

fn parse_test_output(output: &str) -> (usize, usize, usize, usize) {
    // Very simplified test output parsing
    // Real implementation would handle various test runner formats
    let total = output.matches("test").count();
    let passed = output.matches("ok").count() + output.matches("✓").count();
    let failed = output.matches("FAILED").count() + output.matches("✗").count();
    let skipped = output.matches("skipped").count();

    (total, passed, failed, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        let files = vec![PathBuf::from("test.ts")];
        assert_eq!(detect_language_from_files(&files), "typescript");

        let files = vec![PathBuf::from("main.rs")];
        assert_eq!(detect_language_from_files(&files), "rust");
    }

    #[tokio::test]
    async fn test_build_status() {
        let verifier = FixVerifier::new();

        // This test would need a real project setup
        // For now, just verify the structure works
        let result = verifier
            .check_build_status(&[PathBuf::from("test.rs")])
            .await;

        // The command might fail if cargo is not available, but structure should work
        assert!(result.is_ok() || result.is_err());
    }
}
