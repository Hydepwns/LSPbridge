//! Mock LSP Server for Testing
//! 
//! This module provides a mock LSP server that can be used in tests
//! when real language servers are not available.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct MockLspServer {
    pub diagnostics: Arc<Mutex<Vec<MockDiagnostic>>>,
    capabilities: ServerCapabilities,
}

#[derive(Clone)]
pub struct MockDiagnostic {
    pub uri: String,
    pub range: Range,
    pub severity: i32,
    pub message: String,
    pub code: Option<String>,
}

#[derive(Clone)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Clone)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

pub struct ServerCapabilities {
    pub text_document_sync: bool,
    pub diagnostic_provider: bool,
}

impl MockLspServer {
    pub fn new() -> Self {
        Self {
            diagnostics: Arc::new(Mutex::new(Vec::new())),
            capabilities: ServerCapabilities {
                text_document_sync: true,
                diagnostic_provider: true,
            },
        }
    }

    /// Add a diagnostic that will be returned when requested
    pub fn add_diagnostic(&mut self, diagnostic: MockDiagnostic) {
        self.diagnostics.lock().unwrap().push(diagnostic);
    }

    /// Spawn the mock server as a subprocess that speaks LSP over stdio
    pub fn spawn(self) -> std::io::Result<std::process::Child> {
        // In a real implementation, this would spawn a separate binary
        // For testing, we'll create a simple responder
        unimplemented!("Use start_server method for in-process testing")
    }

    /// Start the server in the current process (for testing)
    pub fn start_server(self) {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Some(Ok(line)) = lines.next() {
            if line.starts_with("Content-Length:") {
                // Read the content length
                let len: usize = line
                    .trim_start_matches("Content-Length:")
                    .trim()
                    .parse()
                    .unwrap_or(0);

                // Skip empty line
                lines.next();

                // Read the JSON content
                let mut content = vec![0; len];
                use std::io::Read;
                std::io::stdin().read_exact(&mut content).ok();
                
                if let Ok(content_str) = String::from_utf8(content) {
                    if let Ok(request) = serde_json::from_str::<Value>(&content_str) {
                        let response = self.handle_request(&request);
                        let response_str = serde_json::to_string(&response).unwrap();
                        let response_bytes = response_str.as_bytes();
                        
                        writeln!(stdout, "Content-Length: {}\r", response_bytes.len()).ok();
                        writeln!(stdout, "\r").ok();
                        stdout.write_all(response_bytes).ok();
                        stdout.flush().ok();
                    }
                }
            }
        }
    }

    fn handle_request(&self, request: &Value) -> Value {
        let method = request["method"].as_str().unwrap_or("");
        let id = request.get("id").cloned();

        match method {
            "initialize" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "capabilities": {
                            "textDocumentSync": 1,
                            "diagnosticProvider": {
                                "interFileDependencies": false,
                                "workspaceDiagnostics": false
                            }
                        },
                        "serverInfo": {
                            "name": "mock-lsp-server",
                            "version": "0.1.0"
                        }
                    }
                })
            }
            "textDocument/didOpen" | "textDocument/didChange" => {
                // Send diagnostics notification
                let uri = request["params"]["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or("");
                
                let diagnostics: Vec<Value> = self
                    .diagnostics
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|d| d.uri == uri)
                    .map(|d| {
                        json!({
                            "range": {
                                "start": {
                                    "line": d.range.start.line,
                                    "character": d.range.start.character
                                },
                                "end": {
                                    "line": d.range.end.line,
                                    "character": d.range.end.character
                                }
                            },
                            "severity": d.severity,
                            "code": d.code,
                            "source": "mock-lsp",
                            "message": d.message
                        })
                    })
                    .collect();

                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/publishDiagnostics",
                    "params": {
                        "uri": uri,
                        "diagnostics": diagnostics
                    }
                })
            }
            "shutdown" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": null
                })
            }
            _ => {
                // Return empty response for unknown methods
                if id.is_some() {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": null
                    })
                } else {
                    json!({})
                }
            }
        }
    }
}

/// Create a mock TypeScript LSP server with common diagnostics
pub fn create_typescript_mock() -> MockLspServer {
    let mut server = MockLspServer::new();
    
    // Add some common TypeScript errors
    server.add_diagnostic(MockDiagnostic {
        uri: "file:///test.ts".to_string(),
        range: Range {
            start: Position { line: 10, character: 5 },
            end: Position { line: 10, character: 15 },
        },
        severity: 1, // Error
        message: "Cannot find name 'undefined_var'.".to_string(),
        code: Some("2304".to_string()),
    });
    
    server.add_diagnostic(MockDiagnostic {
        uri: "file:///test.ts".to_string(),
        range: Range {
            start: Position { line: 20, character: 10 },
            end: Position { line: 20, character: 20 },
        },
        severity: 1, // Error
        message: "Type 'string' is not assignable to type 'number'.".to_string(),
        code: Some("2322".to_string()),
    });
    
    server
}

/// Create a mock Rust analyzer server with common diagnostics
pub fn create_rust_analyzer_mock() -> MockLspServer {
    let mut server = MockLspServer::new();
    
    // Add some common Rust errors
    server.add_diagnostic(MockDiagnostic {
        uri: "file:///test.rs".to_string(),
        range: Range {
            start: Position { line: 5, character: 8 },
            end: Position { line: 5, character: 20 },
        },
        severity: 1, // Error
        message: "cannot find value `undefined_var` in this scope".to_string(),
        code: Some("E0425".to_string()),
    });
    
    server.add_diagnostic(MockDiagnostic {
        uri: "file:///test.rs".to_string(),
        range: Range {
            start: Position { line: 15, character: 10 },
            end: Position { line: 15, character: 25 },
        },
        severity: 1, // Error
        message: "mismatched types\nexpected `i32`, found `&str`".to_string(),
        code: Some("E0308".to_string()),
    });
    
    server.add_diagnostic(MockDiagnostic {
        uri: "file:///test.rs".to_string(),
        range: Range {
            start: Position { line: 25, character: 5 },
            end: Position { line: 25, character: 15 },
        },
        severity: 1, // Error
        message: "cannot borrow `data` as mutable more than once at a time".to_string(),
        code: Some("E0499".to_string()),
    });
    
    server
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_server_creation() {
        let server = MockLspServer::new();
        assert!(server.capabilities.text_document_sync);
        assert!(server.capabilities.diagnostic_provider);
    }

    #[test]
    fn test_add_diagnostic() {
        let mut server = MockLspServer::new();
        server.add_diagnostic(MockDiagnostic {
            uri: "file:///test.rs".to_string(),
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 10 },
            },
            severity: 1,
            message: "Test error".to_string(),
            code: None,
        });
        
        assert_eq!(server.diagnostics.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_typescript_mock() {
        let server = create_typescript_mock();
        let diagnostics = server.diagnostics.lock().unwrap();
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0].message.contains("Cannot find name"));
    }

    #[test]
    fn test_rust_analyzer_mock() {
        let server = create_rust_analyzer_mock();
        let diagnostics = server.diagnostics.lock().unwrap();
        assert_eq!(diagnostics.len(), 3);
        assert!(diagnostics[0].message.contains("cannot find value"));
    }
}