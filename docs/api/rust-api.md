# Rust API Reference

The LSP Bridge Rust library provides a comprehensive API for integrating diagnostic capture and export functionality into Rust applications.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
lsp-bridge = "0.1"

# Optional features
[dependencies]
lsp-bridge = { version = "0.1", features = ["async", "rayon"] }
```

## Features

- `default`: Core functionality with standard synchronous API
- `async`: Async/await support using Tokio
- `rayon`: Parallel processing support
- `serde`: Serialization support (enabled by default)

## Core Types

### `Diagnostic`

Represents a single diagnostic from a language server.

```rust
use lsp_bridge::{Diagnostic, DiagnosticSeverity, Range, Position};

let diagnostic = Diagnostic {
    id: uuid::Uuid::new_v4().to_string(),
    file: "/src/main.rs".to_string(),
    range: Range {
        start: Position { line: 10, character: 5 },
        end: Position { line: 10, character: 15 },
    },
    severity: DiagnosticSeverity::Error,
    message: "expected `;`".to_string(),
    code: Some("E0001".to_string()),
    source: "rustc".to_string(),
    related_information: None,
    tags: None,
    data: None,
};
```

### `DiagnosticSnapshot`

A collection of diagnostics at a specific point in time.

```rust
use lsp_bridge::{DiagnosticSnapshot, WorkspaceInfo};
use chrono::Utc;

let snapshot = DiagnosticSnapshot {
    id: uuid::Uuid::new_v4(),
    timestamp: Utc::now(),
    workspace: WorkspaceInfo {
        name: "my-project".to_string(),
        root_path: "/path/to/project".to_string(),
        language: Some("rust".to_string()),
    },
    diagnostics: vec![diagnostic],
    metadata: Default::default(),
};
```

### `PrivacyPolicy`

Controls what diagnostic information is included in exports.

```rust
use lsp_bridge::PrivacyPolicy;

// Use preset
let policy = PrivacyPolicy::default();
let strict_policy = PrivacyPolicy::strict();
let permissive_policy = PrivacyPolicy::permissive();

// Custom policy
let custom_policy = PrivacyPolicy {
    exclude_patterns: vec!["**/*.secret".to_string()],
    sanitize_strings: true,
    sanitize_comments: false,
    include_only_errors: false,
    max_diagnostics_per_file: 100,
    anonymize_file_paths: false,
    encrypt_exports: false,
};
```

## Core Services

### `DiagnosticsCapture`

Main service for capturing and processing diagnostics.

```rust
use lsp_bridge::{DiagnosticsCapture, RawDiagnostics};

let mut capture = DiagnosticsCapture::new();

// Configure privacy
capture.set_privacy_policy(PrivacyPolicy::default());

// Process raw diagnostics from language server
let raw = RawDiagnostics {
    source: "rust-analyzer",
    data: serde_json::json!({
        "diagnostics": [...]
    }),
};

let snapshot = capture.process_diagnostics(raw)?;
```

#### Methods

```rust
impl DiagnosticsCapture {
    /// Create a new capture service
    pub fn new() -> Self;
    
    /// Create with custom configuration
    pub fn with_config(config: CaptureConfig) -> Self;
    
    /// Set privacy policy
    pub fn set_privacy_policy(&mut self, policy: PrivacyPolicy);
    
    /// Process raw diagnostics into a snapshot
    pub fn process_diagnostics(&mut self, raw: RawDiagnostics) 
        -> Result<DiagnosticSnapshot, LSPBridgeError>;
    
    /// Get current snapshot
    pub fn current_snapshot(&self) -> Option<&DiagnosticSnapshot>;
    
    /// Clear current diagnostics
    pub fn clear(&mut self);
}
```

### `PrivacyFilter`

Applies privacy policies to diagnostic data.

```rust
use lsp_bridge::{PrivacyFilter, PrivacyPolicy};

let filter = PrivacyFilter::new(PrivacyPolicy::strict());

// Apply to diagnostics
let filtered = filter.apply(diagnostics)?;

// Check if file should be included
if filter.should_include_file("/src/secret.rs") {
    // Process file
}
```

#### Methods

```rust
impl PrivacyFilter {
    /// Create with policy
    pub fn new(policy: PrivacyPolicy) -> Self;
    
    /// Apply filter to diagnostics
    pub fn apply(&self, diagnostics: Vec<Diagnostic>) 
        -> Result<Vec<Diagnostic>, LSPBridgeError>;
    
    /// Check if file matches exclusion patterns
    pub fn should_include_file(&self, path: &str) -> bool;
    
    /// Sanitize a single diagnostic
    pub fn sanitize_diagnostic(&self, diagnostic: Diagnostic) -> Diagnostic;
}
```

### `FormatConverter`

Converts between different diagnostic formats.

```rust
use lsp_bridge::{FormatConverter, RawDiagnostics};

let converter = FormatConverter::new();

// Convert from TypeScript LSP format
let diagnostics = converter.from_typescript(raw_data)?;

// Convert from generic LSP format
let diagnostics = converter.from_lsp(lsp_diagnostics)?;
```

#### Methods

```rust
impl FormatConverter {
    /// Create new converter
    pub fn new() -> Self;
    
    /// Normalize any raw diagnostics
    pub async fn normalize(&self, raw: RawDiagnostics) 
        -> Result<Vec<Diagnostic>, LSPBridgeError>;
    
    /// Convert from specific formats
    pub fn from_typescript(&self, data: serde_json::Value) 
        -> Result<Vec<Diagnostic>, LSPBridgeError>;
    
    pub fn from_rust_analyzer(&self, data: serde_json::Value) 
        -> Result<Vec<Diagnostic>, LSPBridgeError>;
    
    pub fn from_eslint(&self, data: serde_json::Value) 
        -> Result<Vec<Diagnostic>, LSPBridgeError>;
}
```

### `ExportService`

Exports diagnostic snapshots to various formats.

```rust
use lsp_bridge::{ExportService, ExportFormat};

let export = ExportService::new();

// Export to different formats
let json = export.to_json(&snapshot)?;
let markdown = export.to_markdown(&snapshot)?;
let claude = export.to_claude_format(&snapshot)?;

// Export with options
let options = ExportOptions {
    include_context: true,
    context_lines: 5,
    include_summary: true,
    group_by_file: true,
};

let output = export.export(&snapshot, ExportFormat::Claude, options)?;
```

#### Methods

```rust
impl ExportService {
    /// Create new export service
    pub fn new() -> Self;
    
    /// Export to specific format with options
    pub fn export(&self, snapshot: &DiagnosticSnapshot, 
                  format: ExportFormat, 
                  options: ExportOptions) 
        -> Result<String, LSPBridgeError>;
    
    /// Format-specific exports
    pub fn to_json(&self, snapshot: &DiagnosticSnapshot) 
        -> Result<String, LSPBridgeError>;
    
    pub fn to_markdown(&self, snapshot: &DiagnosticSnapshot) 
        -> Result<String, LSPBridgeError>;
    
    pub fn to_claude_format(&self, snapshot: &DiagnosticSnapshot) 
        -> Result<String, LSPBridgeError>;
}
```

### `MemoryCache`

LRU cache for diagnostic snapshots.

```rust
use lsp_bridge::MemoryCache;
use std::time::Duration;

let mut cache = MemoryCache::new(100, Duration::from_secs(3600));

// Store snapshot
cache.store(snapshot)?;

// Retrieve by ID
if let Some(snapshot) = cache.get(&snapshot_id) {
    // Use snapshot
}

// Get all snapshots
let all = cache.all_snapshots();

// Clear old entries
cache.clean_expired();
```

## Builder Pattern APIs

### `DiagnosticBuilder`

```rust
use lsp_bridge::DiagnosticBuilder;

let diagnostic = DiagnosticBuilder::new()
    .file("/src/main.rs")
    .range(10, 5, 10, 15)
    .severity(DiagnosticSeverity::Error)
    .message("undefined variable")
    .code("E0425")
    .source("rustc")
    .build()?;
```

### `SnapshotBuilder`

```rust
use lsp_bridge::SnapshotBuilder;

let snapshot = SnapshotBuilder::new()
    .workspace_name("my-project")
    .workspace_path("/path/to/project")
    .add_diagnostic(diagnostic1)
    .add_diagnostics(vec![diagnostic2, diagnostic3])
    .with_metadata("version", "1.0.0")
    .build()?;
```

## Async API

When using the `async` feature:

```rust
use lsp_bridge::{AsyncDiagnosticsCapture, AsyncExportService};

#[tokio::main]
async fn main() -> Result<(), LSPBridgeError> {
    let mut capture = AsyncDiagnosticsCapture::new();
    
    // Process diagnostics asynchronously
    let snapshot = capture.process_diagnostics(raw).await?;
    
    // Export asynchronously
    let export = AsyncExportService::new();
    let output = export.to_claude_format(&snapshot).await?;
    
    Ok(())
}
```

## Error Handling

All operations return `Result<T, LSPBridgeError>`:

```rust
use lsp_bridge::LSPBridgeError;

match capture.process_diagnostics(raw) {
    Ok(snapshot) => {
        // Handle success
    }
    Err(LSPBridgeError::PrivacyViolation(msg)) => {
        eprintln!("Privacy error: {}", msg);
    }
    Err(LSPBridgeError::Io(e)) => {
        eprintln!("IO error: {}", e);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Integration Examples

### Direct Library Integration

```rust
use lsp_bridge::{DiagnosticsCapture, ExportService, PrivacyPolicy};

pub struct MyLspClient {
    capture: DiagnosticsCapture,
    export: ExportService,
}

impl MyLspClient {
    pub fn new() -> Self {
        let mut capture = DiagnosticsCapture::new();
        capture.set_privacy_policy(PrivacyPolicy::default());
        
        Self {
            capture,
            export: ExportService::new(),
        }
    }
    
    pub fn handle_diagnostics(&mut self, raw: RawDiagnostics) 
        -> Result<String, LSPBridgeError> {
        let snapshot = self.capture.process_diagnostics(raw)?;
        self.export.to_claude_format(&snapshot)
    }
}
```

### Custom Privacy Policy

```rust
use lsp_bridge::{PrivacyPolicy, PrivacyFilter};

fn create_custom_filter() -> PrivacyFilter {
    let policy = PrivacyPolicy {
        exclude_patterns: vec![
            "**/.git/**".to_string(),
            "**/target/**".to_string(),
            "**/*.key".to_string(),
        ],
        sanitize_strings: true,
        sanitize_comments: false,
        include_only_errors: false,
        max_diagnostics_per_file: 50,
        anonymize_file_paths: false,
        encrypt_exports: false,
    };
    
    PrivacyFilter::new(policy)
}
```

### Streaming Diagnostics

```rust
use lsp_bridge::{DiagnosticsCapture, ExportService};
use tokio::sync::mpsc;

async fn stream_diagnostics(mut rx: mpsc::Receiver<RawDiagnostics>) {
    let mut capture = DiagnosticsCapture::new();
    let export = ExportService::new();
    
    while let Some(raw) = rx.recv().await {
        match capture.process_diagnostics(raw) {
            Ok(snapshot) => {
                if let Ok(output) = export.to_json(&snapshot) {
                    println!("{}", output);
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
```

## Performance Considerations

- **Batch Processing**: Process diagnostics in batches for better performance
- **Caching**: Use `MemoryCache` to avoid reprocessing
- **Parallel Processing**: Enable `rayon` feature for parallel filtering
- **Async Operations**: Use async API for I/O-bound operations

## Thread Safety

- `DiagnosticsCapture`: Not thread-safe, use `Arc<Mutex<_>>` for sharing
- `ExportService`: Thread-safe, can be shared across threads
- `PrivacyFilter`: Thread-safe, immutable after creation
- `MemoryCache`: Thread-safe with internal locking

## Best Practices

1. **Reuse Services**: Create services once and reuse them
2. **Configure Early**: Set privacy policies before processing
3. **Handle Errors**: Always handle `LSPBridgeError` appropriately
4. **Memory Management**: Clear cache periodically in long-running apps
5. **Privacy First**: Default to stricter privacy settings

## Version Compatibility

| LSP Bridge Version | Minimum Rust Version |
|-------------------|---------------------|
| 0.1.x | 1.70.0 |
| 0.2.x | 1.75.0 |

## Migration Guide

### From 0.1.x to 0.2.x

```rust
// Old (0.1.x)
let capture = DiagnosticsCapture::new();
let snapshot = capture.process(raw)?;

// New (0.2.x)
let mut capture = DiagnosticsCapture::new();
let snapshot = capture.process_diagnostics(raw)?;
```