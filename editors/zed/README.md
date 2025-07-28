# LSP Bridge for Zed

Export IDE diagnostics to AI assistants directly from Zed!

## Features

- 📤 Export diagnostics in multiple formats (JSON, Markdown, Claude-optimized)
- 📋 Quick export to clipboard for AI chat interfaces
- 📊 View diagnostic history and trends
- 🔧 Apply high-confidence fixes automatically
- 🔒 Privacy-aware filtering

## Installation

1. Install the LSP Bridge CLI tool:
   ```bash
   cargo install lsp-bridge
   ```

2. Install the Zed extension:
   - Open Zed
   - Go to Extensions
   - Search for "LSP Bridge"
   - Click Install

## Usage

### Commands

- **Export Diagnostics**: `Cmd+Shift+E, Cmd+Shift+D`
- **Export to Clipboard**: `Cmd+Shift+E, Cmd+Shift+C`
- **Show History**: Command palette → "LSP Bridge: Show Diagnostic History"
- **Apply Fixes**: Command palette → "LSP Bridge: Apply Quick Fixes"

### Status Bar

The status bar shows current diagnostic counts:
- 🔴 Red: Errors present
- 🟡 Yellow: Warnings present
- ✅ Green: No issues

Click the status bar item to export diagnostics.

### Settings

Configure in Zed settings:

```json
{
  "lsp-bridge": {
    "format": "claude",        // "json", "markdown", or "claude"
    "privacy": "default",      // "default", "strict", or "permissive"
    "include_context": true,   // Include code context
    "context_lines": 3         // Number of context lines
  }
}
```

## Privacy Levels

- **Default**: Removes API keys, passwords, and sensitive data
- **Strict**: Additional filtering of paths and identifiers
- **Permissive**: Minimal filtering for trusted environments

## License

MIT