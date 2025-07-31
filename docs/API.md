# API Reference

## CLI API

### Core Commands

#### `lsp-bridge export`

```bash
lsp-bridge export [OPTIONS]

Options:
  --format <FORMAT>    Output format: json, markdown, claude [default: claude]
  --privacy <LEVEL>    Privacy level: default, strict, permissive [default: default]
  --errors-only        Only include error-level diagnostics
  --output <FILE>      Write to file instead of stdout
  --project <NAME>     Override project name

Example:
  cat diagnostics.json | lsp-bridge export --format claude --privacy strict
```

#### `lsp-bridge config`

```bash
lsp-bridge config <SUBCOMMAND>

Subcommands:
  init        Initialize configuration file
    --commented    Include helpful comments
    --minimal      Create minimal configuration
  show        Display current configuration
  validate    Validate configuration file
    --file <PATH>  Validate specific file
```

#### `lsp-bridge serve`

```bash
lsp-bridge serve [OPTIONS]

Options:
  --metrics-port <PORT>  Prometheus metrics port [default: 9090]
```

## Library API

### Basic Usage

```rust
use lsp_bridge::{Config, ExportService, PrivacyLevel};
use lsp_bridge::format::{Format, ClaudeFormatter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load_or_default()?;
    
    // Create export service
    let export_service = ExportService::new(config);
    
    // Export diagnostics
    let diagnostics = load_diagnostics()?;
    let output = export_service.export(
        diagnostics,
        Format::Claude,
        PrivacyLevel::Default,
    ).await?;
    
    println!("{}", output);
    Ok(())
}
```

### Core Types

#### `Diagnostic`

Represents a single diagnostic item.

```rust
pub struct Diagnostic {
    pub range: Range,
    pub severity: Severity,
    pub code: Option<DiagnosticCode>,
    pub source: Option<String>,
    pub message: String,
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
    pub tags: Option<Vec<DiagnosticTag>>,
}
```

#### `PrivacyLevel`

Controls how sensitive information is handled.

```rust
pub enum PrivacyLevel {
    /// Default privacy settings
    Default,
    /// Strict privacy - anonymize paths, remove sensitive strings
    Strict,
    /// Permissive - minimal filtering
    Permissive,
}
```

#### `Format`

Output format for diagnostics.

```rust
pub enum Format {
    /// Standard JSON format
    Json,
    /// Markdown format
    Markdown,
    /// Claude-optimized format
    Claude,
}
```

### Services

#### `ExportService`

Main service for exporting diagnostics.

```rust
impl ExportService {
    /// Create new export service with configuration
    pub fn new(config: Config) -> Self;
    
    /// Export diagnostics in specified format
    pub async fn export(
        &self,
        diagnostics: Vec<Diagnostic>,
        format: Format,
        privacy: PrivacyLevel,
    ) -> Result<String>;
    
    /// Export with additional options
    pub async fn export_with_options(
        &self,
        diagnostics: Vec<Diagnostic>,
        options: ExportOptions,
    ) -> Result<String>;
}
```

#### `PrivacyFilter`

Filter diagnostics based on privacy settings.

```rust
impl PrivacyFilter {
    /// Create new privacy filter
    pub fn new(level: PrivacyLevel, workspace_root: PathBuf) -> Self;
    
    /// Filter a single diagnostic
    pub fn filter_diagnostic(&self, diagnostic: &mut Diagnostic) -> bool;
    
    /// Filter file path
    pub fn filter_path(&self, path: &Path) -> Option<PathBuf>;
}
```

### Extension Points

#### Custom Formatters

Implement the `Formatter` trait to add new output formats:

```rust
use lsp_bridge::format::{Formatter, FormatterContext};

pub struct MyFormatter;

impl Formatter for MyFormatter {
    fn format(&self, context: FormatterContext) -> Result<String> {
        // Custom formatting logic
        Ok(format!("Custom output"))
    }
}
```

#### Custom Analyzers

Add support for new languages:

```rust
use lsp_bridge::analyzers::{Analyzer, AnalyzerResult};

pub struct MyLanguageAnalyzer;

impl Analyzer for MyLanguageAnalyzer {
    fn analyze(&self, diagnostics: &[Diagnostic]) -> AnalyzerResult {
        // Language-specific analysis
        AnalyzerResult::default()
    }
}
```

### Configuration

#### Loading Configuration

```rust
use lsp_bridge::Config;

// Load from default locations
let config = Config::load_or_default()?;

// Load from specific file
let config = Config::load_from_file("path/to/lspbridge.toml")?;

// Create with defaults
let config = Config::default();
```

#### Configuration Builder

```rust
use lsp_bridge::ConfigBuilder;

let config = ConfigBuilder::new()
    .with_cache_size(1000)
    .with_parallel_processing(true)
    .with_git_integration(true)
    .build()?;
```

### Error Handling

All LSP Bridge errors implement the `Error` trait:

```rust
use lsp_bridge::error::{Error, ErrorKind};

match result {
    Err(Error::Configuration(msg)) => {
        eprintln!("Configuration error: {}", msg);
    }
    Err(Error::Io(e)) => {
        eprintln!("IO error: {}", e);
    }
    _ => {}
}
```

### Async API

Most operations are async for better performance:

```rust
use lsp_bridge::cache::Cache;

let cache = Cache::new(config.cache)?;

// Async operations
let cached = cache.get(&key).await?;
cache.set(&key, &value).await?;
```

## IDE Integration API

### VS Code Extension API

The VS Code extension exposes commands and APIs:

```typescript
// Import the extension API
import * as lspBridge from 'lsp-bridge';

// Export current diagnostics
const result = await lspBridge.exportDiagnostics({
    format: 'claude',
    privacy: 'default',
    includeContext: true,
});

// Get configuration
const config = lspBridge.getConfiguration();

// Listen to export events
lspBridge.onDidExport((event) => {
    console.log(`Exported ${event.count} diagnostics`);
});
```

### Neovim Lua API

```lua
local lsp_bridge = require('lsp-bridge')

-- Export diagnostics
lsp_bridge.export({
    format = 'claude',
    privacy = 'default',
    errors_only = false,
})

-- Get current diagnostics
local diagnostics = lsp_bridge.get_diagnostics()

-- Configure
lsp_bridge.setup({
    auto_export = true,
    privacy_level = 'strict',
})
```

## REST API (Future)

When running with `lsp-bridge serve`, a REST API is available:

### `POST /export`

Export diagnostics via HTTP.

**Request:**
```json
{
    "diagnostics": [...],
    "format": "claude",
    "privacy": "default",
    "options": {
        "errors_only": false,
        "include_context": true
    }
}
```

**Response:**
```json
{
    "success": true,
    "output": "...",
    "metadata": {
        "total": 10,
        "errors": 3,
        "warnings": 5
    }
}
```

### `GET /metrics`

Prometheus metrics endpoint.

### `GET /health`

Health check endpoint.

**Response:**
```json
{
    "status": "healthy",
    "version": "0.3.0",
    "uptime": 3600
}
```