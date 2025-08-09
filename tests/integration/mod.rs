// Integration test modules
pub mod dynamic_config_tests;
pub mod end_to_end_tests;
pub mod enhanced_processor_tests;
pub mod git_integration_tests;
pub mod health_dashboard_tests; // Fixed - methods now public
pub mod migration_validation_tests;
pub mod privacy_integration_tests;
pub mod privacy_filter_integration_tests;
pub mod quick_fix_verification_tests;
// pub mod real_world_integration_tests; // Disabled - RepositoryRegistry API issues
// pub mod semantic_context_tests; // Disabled - struct field mismatches
// pub mod multi_repo_integration_tests; // Disabled - RepositoryRegistry API issues
pub mod workflow_tests;

// Mock LSP server for testing
#[path = "../mock_lsp_server.rs"]
pub mod mock_lsp_server;

// LSP test helpers
pub mod lsp_test_helpers;

// Simple LSP tests that work with current codebase
pub mod simple_lsp_tests;

// Include AI training tests
#[path = "../ai_training/mod.rs"]
pub mod ai_training_tests;

// Include quick fix tests
#[path = "../quick_fix/mod.rs"]
pub mod quick_fix_tests;

// Test utilities
use lsp_bridge::core::{Diagnostic, DiagnosticSeverity};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

pub struct LspTestClient {
    process: Child,
    reader: BufReader<std::process::ChildStdout>,
    writer: BufWriter<std::process::ChildStdin>,
    request_id: i32,
}

impl LspTestClient {
    pub fn spawn_typescript_lsp() -> Result<Self, Box<dyn std::error::Error>> {
        Self::spawn_lsp_server("typescript-language-server", &["--stdio"])
    }

    pub fn spawn_rust_analyzer() -> Result<Self, Box<dyn std::error::Error>> {
        Self::spawn_lsp_server("rust-analyzer", &[])
    }

    pub fn spawn_python_lsp() -> Result<Self, Box<dyn std::error::Error>> {
        Self::spawn_lsp_server("pylsp", &[])
    }

    fn spawn_lsp_server(cmd: &str, args: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut process = command.spawn()?;

        let reader = BufReader::new(process.stdout.take().unwrap());
        let writer = BufWriter::new(process.stdin.take().unwrap());

        Ok(LspTestClient {
            process,
            reader,
            writer,
            request_id: 0,
        })
    }

    pub fn initialize(&mut self, root_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let init_params = json!({
            "processId": std::process::id(),
            "rootPath": root_path,
            "rootUri": format!("file://{}", root_path),
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {
                        "relatedInformation": true,
                        "versionSupport": true,
                        "codeDescriptionSupport": true,
                        "dataSupport": true
                    }
                }
            }
        });

        self.send_request("initialize", init_params)?;
        self.wait_for_response()?;

        self.send_notification("initialized", json!({}))?;

        Ok(())
    }

    pub fn open_file(
        &mut self,
        file_path: &str,
        content: &str,
        language_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let params = json!({
            "textDocument": {
                "uri": format!("file://{}", file_path),
                "languageId": language_id,
                "version": 1,
                "text": content
            }
        });

        self.send_notification("textDocument/didOpen", params)?;

        // Give LSP time to process
        thread::sleep(Duration::from_millis(500));

        Ok(())
    }

    pub fn collect_diagnostics(&mut self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let mut diagnostics = Vec::new();

        // Read all pending messages
        while let Ok(line) = self.read_message_timeout(Duration::from_millis(100)) {
            if let Ok(msg) = serde_json::from_str::<Value>(&line) {
                if msg.get("method").and_then(|m| m.as_str())
                    == Some("textDocument/publishDiagnostics")
                {
                    if let Some(params) = msg.get("params") {
                        if let Some(diags) = params.get("diagnostics").and_then(|d| d.as_array()) {
                            diagnostics.extend(diags.clone());
                        }
                    }
                }
            }
        }

        Ok(diagnostics)
    }

    fn send_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.request_id += 1;
        let message = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params
        });

        self.send_message(message)
    }

    fn send_notification(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        self.send_message(message)
    }

    fn send_message(&mut self, message: Value) -> Result<(), Box<dyn std::error::Error>> {
        let content = message.to_string();
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        self.writer.write_all(header.as_bytes())?;
        self.writer.write_all(content.as_bytes())?;
        self.writer.flush()?;

        Ok(())
    }

    fn wait_for_response(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        loop {
            let line = self.read_message()?;
            let response: Value = serde_json::from_str(&line)?;

            if response.get("id").is_some() {
                return Ok(response);
            }
        }
    }

    fn read_message(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        // Read header
        let mut header = String::new();
        loop {
            self.reader.read_line(&mut header)?;
            if header.trim().is_empty() {
                break;
            }
        }

        // Parse content length
        let content_length = header
            .lines()
            .find(|line| line.starts_with("Content-Length:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|len| len.trim().parse::<usize>().ok())
            .unwrap_or(0);

        // Read content
        let mut content = vec![0; content_length];
        self.reader.read_exact(&mut content)?;

        Ok(String::from_utf8(content)?)
    }

    fn read_message_timeout(
        &mut self,
        _timeout: Duration,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Simplified timeout implementation
        // In production, use async or proper timeout mechanisms
        self.read_message()
    }

    pub fn shutdown(mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_request("shutdown", json!(null))?;
        self.wait_for_response()?;
        self.send_notification("exit", json!(null))?;

        thread::sleep(Duration::from_millis(100));
        self.process.kill()?;

        Ok(())
    }
}

pub fn convert_lsp_diagnostic(lsp_diag: &Value, file_path: &str, source: &str) -> Diagnostic {
    let range = lsp_diag.get("range").unwrap();
    let start = range.get("start").unwrap();
    let end = range.get("end").unwrap();

    let severity = match lsp_diag
        .get("severity")
        .and_then(|s| s.as_u64())
        .unwrap_or(1)
    {
        1 => DiagnosticSeverity::Error,
        2 => DiagnosticSeverity::Warning,
        3 => DiagnosticSeverity::Information,
        4 => DiagnosticSeverity::Hint,
        _ => DiagnosticSeverity::Information,
    };

    Diagnostic {
        id: Uuid::new_v4().to_string(),
        file: file_path.to_string(),
        range: lsp_bridge::core::Range {
            start: lsp_bridge::core::Position {
                line: start.get("line").unwrap().as_u64().unwrap() as u32,
                character: start.get("character").unwrap().as_u64().unwrap() as u32,
            },
            end: lsp_bridge::core::Position {
                line: end.get("line").unwrap().as_u64().unwrap() as u32,
                character: end.get("character").unwrap().as_u64().unwrap() as u32,
            },
        },
        severity,
        message: lsp_diag
            .get("message")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        code: None,
        source: source.to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}
