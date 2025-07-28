# Examples and Recipes

This guide provides practical examples for common LSP Bridge use cases.

## Table of Contents

1. [Basic Usage](#basic-usage)
2. [Privacy Configurations](#privacy-configurations)
3. [Integration Patterns](#integration-patterns)
4. [CI/CD Integration](#cicd-integration)
5. [Custom Workflows](#custom-workflows)
6. [Performance Optimization](#performance-optimization)

## Basic Usage

### Quick Export to Claude

```bash
# Export current workspace diagnostics for Claude
lsp-bridge export --format claude

# Export with context for better AI understanding
lsp-bridge export --format claude --context-lines 5 > diagnostics.md

# Copy to clipboard (macOS)
lsp-bridge export --format claude | pbcopy

# Copy to clipboard (Linux)
lsp-bridge export --format claude | xclip -selection clipboard
```

### Watch Mode for Real-time Debugging

```bash
# Watch diagnostics and save to file
lsp-bridge watch --format json --output diagnostics.json

# Watch with filtering
lsp-bridge watch --errors-only --format markdown

# Pipe to another tool for processing
lsp-bridge watch --format json | jq '.diagnostics[] | select(.severity == "error")'
```

## Privacy Configurations

### Development Environment (Permissive)

```toml
# .lsp-bridge.dev.toml
[privacy]
exclude_patterns = []
sanitize_strings = false
include_only_errors = false
max_diagnostics_per_file = 1000

[export]
format = "claude"
include_context = true
context_lines = 10
```

Usage:
```bash
lsp-bridge export --config .lsp-bridge.dev.toml
```

### Production/Shared Environment (Strict)

```toml
# .lsp-bridge.prod.toml
[privacy]
exclude_patterns = [
    "**/.env*",
    "**/secrets/**",
    "**/credentials/**",
    "**/*_key*",
    "**/*.pem",
    "**/config/production.*"
]
sanitize_strings = true
sanitize_comments = true
include_only_errors = true
max_diagnostics_per_file = 10
anonymize_file_paths = true

[export]
format = "json"
include_context = false
```

### Custom Privacy Rules for Specific Projects

```rust
// In a Rust project using the library
use lsp_bridge::{PrivacyPolicy, PrivacyFilter};

fn create_project_filter() -> PrivacyFilter {
    let mut policy = PrivacyPolicy::default();
    
    // Add project-specific exclusions
    policy.exclude_patterns.extend(vec![
        "**/internal/**".to_string(),
        "**/proprietary/**".to_string(),
        "**/*_secret*".to_string(),
    ]);
    
    // Custom sanitization
    policy.sanitize_strings = true;
    policy.max_diagnostics_per_file = 25;
    
    PrivacyFilter::new(policy)
}
```

## Integration Patterns

### Shell Script Integration

```bash
#!/bin/bash
# diagnostic-report.sh

# Generate timestamped diagnostic report
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTPUT_FILE="diagnostics_${TIMESTAMP}.md"

echo "# Diagnostic Report - $(date)" > "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Add git information
echo "## Repository Info" >> "$OUTPUT_FILE"
echo "Branch: $(git branch --show-current)" >> "$OUTPUT_FILE"
echo "Commit: $(git rev-parse --short HEAD)" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Add diagnostics
echo "## Diagnostics" >> "$OUTPUT_FILE"
lsp-bridge export --format markdown >> "$OUTPUT_FILE"

echo "Report saved to: $OUTPUT_FILE"
```

### Git Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Check for errors before commit
ERROR_COUNT=$(lsp-bridge export --format json --errors-only | jq '.summary.errorCount')

if [ "$ERROR_COUNT" -gt 0 ]; then
    echo "❌ Commit blocked: $ERROR_COUNT errors found"
    echo "Run 'lsp-bridge export --errors-only' to see details"
    exit 1
fi

echo "✅ No errors found, proceeding with commit"
```

### Python Integration

```python
import subprocess
import json
from pathlib import Path

class DiagnosticsAnalyzer:
    def __init__(self):
        self.lsp_bridge_cmd = "lsp-bridge"
    
    def get_diagnostics(self, format="json"):
        """Get current diagnostics from LSP Bridge."""
        result = subprocess.run(
            [self.lsp_bridge_cmd, "export", "--format", format],
            capture_output=True,
            text=True
        )
        
        if result.returncode != 0:
            raise Exception(f"LSP Bridge error: {result.stderr}")
        
        if format == "json":
            return json.loads(result.stdout)
        return result.stdout
    
    def analyze_error_trends(self):
        """Analyze error patterns in the codebase."""
        data = self.get_diagnostics()
        
        # Group by file
        by_file = {}
        for diag in data["diagnostics"]:
            file = diag["file"]
            if file not in by_file:
                by_file[file] = []
            by_file[file].append(diag)
        
        # Find files with most errors
        error_counts = {
            file: sum(1 for d in diags if d["severity"] == "error")
            for file, diags in by_file.items()
        }
        
        return sorted(error_counts.items(), key=lambda x: x[1], reverse=True)

# Usage
analyzer = DiagnosticsAnalyzer()
top_error_files = analyzer.analyze_error_trends()
print("Files with most errors:")
for file, count in top_error_files[:5]:
    print(f"  {file}: {count} errors")
```

### Node.js Integration

```javascript
const { exec } = require('child_process');
const { promisify } = require('util');
const execAsync = promisify(exec);

class LspBridgeClient {
    async getDiagnostics(options = {}) {
        const format = options.format || 'json';
        const cmd = `lsp-bridge export --format ${format}`;
        
        try {
            const { stdout } = await execAsync(cmd);
            return format === 'json' ? JSON.parse(stdout) : stdout;
        } catch (error) {
            throw new Error(`LSP Bridge failed: ${error.message}`);
        }
    }
    
    async watchDiagnostics(callback, interval = 1000) {
        const watcher = setInterval(async () => {
            try {
                const diagnostics = await this.getDiagnostics();
                callback(null, diagnostics);
            } catch (error) {
                callback(error);
            }
        }, interval);
        
        return () => clearInterval(watcher);
    }
}

// Usage with Express API
const express = require('express');
const app = express();
const client = new LspBridgeClient();

app.get('/api/diagnostics', async (req, res) => {
    try {
        const diagnostics = await client.getDiagnostics();
        res.json(diagnostics);
    } catch (error) {
        res.status(500).json({ error: error.message });
    }
});
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Diagnostic Check

on: [push, pull_request]

jobs:
  check-diagnostics:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install LSP Bridge
        run: |
          cargo install lsp-bridge
      
      - name: Setup Language Servers
        run: |
          # Install required LSPs
          npm install -g typescript typescript-language-server
      
      - name: Run Diagnostic Check
        run: |
          # Export diagnostics
          lsp-bridge export --format json --output diagnostics.json
          
          # Check for errors
          ERROR_COUNT=$(jq '.summary.errorCount' diagnostics.json)
          if [ "$ERROR_COUNT" -gt 0 ]; then
            echo "::error::Found $ERROR_COUNT errors"
            lsp-bridge export --format markdown --errors-only
            exit 1
          fi
      
      - name: Upload Diagnostic Report
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: diagnostic-report
          path: diagnostics.json
```

### GitLab CI

```yaml
diagnostic-check:
  stage: test
  script:
    - cargo install lsp-bridge
    - lsp-bridge export --format json --output diagnostics.json
    - |
      ERROR_COUNT=$(jq '.summary.errorCount' diagnostics.json)
      if [ "$ERROR_COUNT" -gt 0 ]; then
        echo "Found $ERROR_COUNT errors"
        lsp-bridge export --format markdown --errors-only
        exit 1
      fi
  artifacts:
    when: on_failure
    paths:
      - diagnostics.json
    reports:
      junit: diagnostics.xml  # If you add XML export
```

## Custom Workflows

### Diagnostic Dashboard

```rust
// Rust web service for diagnostic dashboard
use actix_web::{web, App, HttpResponse, HttpServer};
use lsp_bridge::{DiagnosticsCapture, ExportService};
use std::sync::Mutex;

struct AppState {
    capture: Mutex<DiagnosticsCapture>,
    export: ExportService,
}

async fn get_diagnostics(data: web::Data<AppState>) -> HttpResponse {
    let capture = data.capture.lock().unwrap();
    
    if let Some(snapshot) = capture.current_snapshot() {
        let json = data.export.to_json(snapshot).unwrap();
        HttpResponse::Ok()
            .content_type("application/json")
            .body(json)
    } else {
        HttpResponse::NotFound().body("No diagnostics available")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        capture: Mutex::new(DiagnosticsCapture::new()),
        export: ExportService::new(),
    });
    
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/diagnostics", web::get().to(get_diagnostics))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### Slack Integration

```python
import requests
import subprocess
import json

def send_diagnostic_summary_to_slack(webhook_url):
    # Get diagnostics
    result = subprocess.run(
        ["lsp-bridge", "export", "--format", "json"],
        capture_output=True,
        text=True
    )
    
    if result.returncode != 0:
        return
    
    data = json.loads(result.stdout)
    summary = data["summary"]
    
    # Create Slack message
    message = {
        "text": "Diagnostic Summary",
        "blocks": [
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": f"*Diagnostic Report*\n"
                           f"• Errors: {summary['errorCount']}\n"
                           f"• Warnings: {summary['warningCount']}\n"
                           f"• Total: {summary['totalDiagnostics']}"
                }
            }
        ]
    }
    
    # Add error details if any
    if summary["errorCount"] > 0:
        errors = [d for d in data["diagnostics"] if d["severity"] == "error"]
        error_text = "\n".join([
            f"• `{e['file']}:{e['range']['start']['line']}` - {e['message']}"
            for e in errors[:5]  # First 5 errors
        ])
        
        message["blocks"].append({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": f"*Recent Errors:*\n{error_text}"
            }
        })
    
    # Send to Slack
    requests.post(webhook_url, json=message)
```

## Performance Optimization

### Batch Processing Large Codebases

```rust
use lsp_bridge::{DiagnosticsCapture, PrivacyPolicy};
use rayon::prelude::*;
use std::path::PathBuf;

fn process_large_codebase(root: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut capture = DiagnosticsCapture::new();
    capture.set_privacy_policy(PrivacyPolicy::default());
    
    // Find all source files
    let files: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension() == Some("rs"))
        .map(|e| e.path().to_owned())
        .collect();
    
    // Process in parallel batches
    files.par_chunks(100)
        .try_for_each(|batch| {
            // Process batch of files
            let diagnostics = collect_diagnostics_for_files(batch)?;
            
            // Merge into main capture
            capture.add_diagnostics(diagnostics)?;
            
            Ok::<(), Box<dyn Error>>(())
        })?;
    
    // Export final results
    let export = ExportService::new();
    let output = export.to_claude_format(capture.current_snapshot().unwrap())?;
    
    std::fs::write("diagnostics.md", output)?;
    
    Ok(())
}
```

### Incremental Diagnostic Updates

```typescript
// VS Code extension with incremental updates
class IncrementalDiagnosticsCollector {
    private cache = new Map<string, Diagnostic[]>();
    private lastExport: string | null = null;
    
    constructor(private bridge: BridgeInterface) {
        // Listen for changes
        vscode.languages.onDidChangeDiagnostics(this.handleChange.bind(this));
    }
    
    private handleChange(event: vscode.DiagnosticChangeEvent) {
        // Update only changed files
        for (const uri of event.uris) {
            const diagnostics = vscode.languages.getDiagnostics(uri);
            
            if (diagnostics.length === 0) {
                this.cache.delete(uri.toString());
            } else {
                this.cache.set(uri.toString(), diagnostics);
            }
        }
        
        // Debounce exports
        this.scheduleExport();
    }
    
    private scheduleExport = debounce(async () => {
        const snapshot = this.createIncrementalSnapshot();
        this.lastExport = await this.bridge.exportSnapshot(snapshot);
    }, 500);
}
```

### Memory-Efficient Streaming

```bash
#!/bin/bash
# Stream large diagnostic outputs

# Use head/tail for pagination
lsp-bridge export --format json | jq -c '.diagnostics[]' | head -n 100

# Filter while streaming
lsp-bridge watch --format json | \
    jq -c 'select(.diagnostics[].severity == "error")' | \
    while read -r line; do
        echo "Error detected: $line"
        # Process each error as it comes
    done

# Compress large exports
lsp-bridge export --format json | gzip > diagnostics.json.gz
```

## Advanced Recipes

### Multi-Language Project Analysis

```python
#!/usr/bin/env python3
import subprocess
import json
from collections import defaultdict

def analyze_multi_language_project():
    """Analyze diagnostics grouped by language server."""
    
    # Get all diagnostics
    result = subprocess.run(
        ["lsp-bridge", "export", "--format", "json"],
        capture_output=True,
        text=True
    )
    
    data = json.loads(result.stdout)
    
    # Group by source (language server)
    by_source = defaultdict(list)
    for diag in data["diagnostics"]:
        by_source[diag["source"]].append(diag)
    
    # Generate report
    print("# Multi-Language Diagnostic Report\n")
    
    for source, diagnostics in by_source.items():
        error_count = sum(1 for d in diagnostics if d["severity"] == "error")
        warning_count = sum(1 for d in diagnostics if d["severity"] == "warning")
        
        print(f"## {source}")
        print(f"- Errors: {error_count}")
        print(f"- Warnings: {warning_count}")
        print(f"- Total: {len(diagnostics)}\n")
        
        # Show sample errors
        errors = [d for d in diagnostics if d["severity"] == "error"][:3]
        if errors:
            print("### Sample Errors:")
            for err in errors:
                print(f"- `{err['file']}:{err['range']['start']['line']}`: {err['message']}")
            print()

if __name__ == "__main__":
    analyze_multi_language_project()
```

### Diagnostic History Tracking

```rust
use lsp_bridge::{DiagnosticsCapture, DiagnosticSnapshot};
use chrono::{DateTime, Utc};
use std::collections::VecDeque;

struct DiagnosticHistory {
    snapshots: VecDeque<(DateTime<Utc>, DiagnosticSnapshot)>,
    max_history: usize,
}

impl DiagnosticHistory {
    fn new(max_history: usize) -> Self {
        Self {
            snapshots: VecDeque::with_capacity(max_history),
            max_history,
        }
    }
    
    fn add_snapshot(&mut self, snapshot: DiagnosticSnapshot) {
        if self.snapshots.len() >= self.max_history {
            self.snapshots.pop_front();
        }
        
        self.snapshots.push_back((Utc::now(), snapshot));
    }
    
    fn get_trend(&self) -> DiagnosticTrend {
        if self.snapshots.len() < 2 {
            return DiagnosticTrend::Stable;
        }
        
        let recent = &self.snapshots.back().unwrap().1;
        let previous = &self.snapshots[self.snapshots.len() - 2].1;
        
        let recent_errors = count_errors(&recent.diagnostics);
        let previous_errors = count_errors(&previous.diagnostics);
        
        match recent_errors.cmp(&previous_errors) {
            std::cmp::Ordering::Greater => DiagnosticTrend::Worsening,
            std::cmp::Ordering::Less => DiagnosticTrend::Improving,
            std::cmp::Ordering::Equal => DiagnosticTrend::Stable,
        }
    }
}
```

These examples demonstrate the flexibility and power of LSP Bridge across different use cases and integration scenarios. Choose the patterns that best fit your workflow and customize them to your needs.