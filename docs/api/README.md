# LSP Bridge API Documentation

## Overview

LSP Bridge provides multiple APIs for integrating IDE diagnostics with AI assistants:

1. **[CLI API](./cli-reference.md)** - Command-line interface for direct usage
2. **[Rust Library API](./rust-api.md)** - Core library for embedding in Rust applications
3. **[Extension APIs](./extension-api.md)** - APIs for IDE extensions
4. **REST API** - HTTP interface for web integrations (planned for v0.2.0)

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   IDE/Editor    │     │   LSP Bridge    │     │  AI Assistant   │
│                 │     │                 │     │                 │
│  ┌───────────┐  │     │  ┌───────────┐  │     │                 │
│  │    LSP    │  │────▶│  │  Capture  │  │     │                 │
│  │  Server   │  │     │  │  Service  │  │     │                 │
│  └───────────┘  │     │  └───────────┘  │     │                 │
│                 │     │        │        │     │                 │
│  ┌───────────┐  │     │        ▼        │     │                 │
│  │Extension/ │  │     │  ┌───────────┐  │     │  ┌───────────┐  │
│  │  Plugin   │◀─┼─────┼─▶│  Privacy  │  │────▶│  │   Claude  │  │
│  └───────────┘  │     │  │  Filter   │  │     │  │    Code   │  │
│                 │     │  └───────────┘  │     │  └───────────┘  │
│                 │     │        │        │     │                 │
│                 │     │        ▼        │     │                 │
│                 │     │  ┌───────────┐  │     │                 │
│                 │     │  │  Export   │  │     │                 │
│                 │     │  │  Service  │  │     │                 │
│                 │     │  └───────────┘  │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Core Concepts

### Diagnostic
A diagnostic represents an issue identified by a language server:

```rust
pub struct Diagnostic {
    pub id: String,
    pub file: String,
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub code: Option<String>,
    pub source: String,
    pub related_information: Option<Vec<RelatedInformation>>,
    pub tags: Option<Vec<DiagnosticTag>>,
    pub data: Option<serde_json::Value>,
}
```

### DiagnosticSnapshot
A collection of diagnostics at a point in time:

```rust
pub struct DiagnosticSnapshot {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub workspace: WorkspaceInfo,
    pub diagnostics: Vec<Diagnostic>,
    pub metadata: SnapshotMetadata,
}
```

### Privacy Policy
Controls what diagnostic information is shared:

```rust
pub struct PrivacyPolicy {
    pub exclude_patterns: Vec<String>,
    pub sanitize_strings: bool,
    pub sanitize_comments: bool,
    pub include_only_errors: bool,
    pub max_diagnostics_per_file: usize,
    pub anonymize_file_paths: bool,
}
```

## Quick Start

### CLI Usage
```bash
# Export current diagnostics
lsp-bridge export --format claude

# Watch for changes
lsp-bridge watch --format json

# With privacy settings
lsp-bridge export --privacy strict --errors-only
```

### Rust Library Usage
```rust
use lsp_bridge::{DiagnosticsCapture, ExportService, PrivacyPolicy};

// Create capture service
let mut capture = DiagnosticsCapture::new();

// Configure privacy
let policy = PrivacyPolicy::strict();
capture.set_privacy_policy(policy);

// Process diagnostics
let snapshot = capture.process_diagnostics(raw_diagnostics)?;

// Export to desired format
let export = ExportService::new();
let output = export.to_claude_format(&snapshot)?;
```

### Extension Usage (VS Code)
```typescript
// In VS Code extension
import { DiagnosticsBridge } from 'lsp-bridge-vscode';

const bridge = new DiagnosticsBridge({
    privacy: { sanitizeStrings: true },
    export: { format: 'claude' }
});

const diagnostics = await bridge.exportDiagnostics();
```

## API Stability

| API | Status | Version |
|-----|--------|---------|
| CLI | Stable | 0.1.0+ |
| Rust Library | Beta | 0.1.0+ |
| VS Code Extension | Beta | 0.1.0+ |
| REST API | Planned | 0.2.0+ |

## Error Handling

All APIs use consistent error types:

```rust
#[derive(Debug, thiserror::Error)]
pub enum LSPBridgeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Privacy policy violation: {0}")]
    PrivacyViolation(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
```

## Rate Limiting

- CLI: No rate limiting
- Library: Configurable debouncing (default: 500ms)
- Extensions: Follow IDE-specific guidelines

## Next Steps

- [CLI Reference](./cli-reference.md) - Complete command documentation
- [Rust API Reference](./rust-api.md) - Library integration guide
- [Extension Development](./extension-api.md) - Build IDE plugins
- [Examples](./examples.md) - Code samples and recipes