# LSP Bridge for Visual Studio Code

Export IDE diagnostics to AI assistants with a single click!

## Features

- **📤 Export Diagnostics**: Export all workspace diagnostics in multiple formats (JSON, Markdown, Claude-optimized)
- **📋 Clipboard Integration**: Quick export to clipboard for pasting into AI chat interfaces
- **👀 Live Watching**: Monitor diagnostics in real-time with automatic updates
- **📊 History View**: Visualize diagnostic trends and identify problem areas
- **🔧 Quick Fixes**: Apply high-confidence fixes automatically
- **🔒 Privacy Control**: Three privacy levels to control what information is shared

## Requirements

- LSP Bridge CLI tool must be installed (`cargo install lsp-bridge`)
- VS Code 1.85.0 or higher

## Installation

1. Install the LSP Bridge CLI:
   ```bash
   cargo install lsp-bridge
   ```

2. Install the VS Code extension from the marketplace or VSIX file

## Usage

### Export Diagnostics

1. **Command Palette**: `Ctrl+Shift+P` → "LSP Bridge: Export Diagnostics"
2. **Status Bar**: Click the LSP Bridge status bar item
3. **Context Menu**: Right-click in editor → "Export Diagnostics"
4. **Keyboard Shortcut**: `Ctrl+Shift+E, Ctrl+Shift+C` (export to clipboard)

### Export Options

- **Format**: JSON, Markdown, or Claude-optimized format
- **Scope**: Workspace, Current File, Open Files, or Errors Only
- **Destination**: File, Clipboard, or Both
- **Privacy**: Default, Strict, or Permissive filtering
- **Context**: Include surrounding code context

### Watch Mode

Start watching diagnostics for real-time updates:
- Command: "LSP Bridge: Start Watching Diagnostics"
- Updates appear in the Output panel

### History View

View diagnostic trends and hot spots:
- Command: "LSP Bridge: Show Diagnostic History"
- Interactive dashboard with metrics and visualizations

### Quick Fixes

Apply automated fixes for high-confidence diagnostics:
- Command: "LSP Bridge: Apply Quick Fixes"
- Configure confidence threshold in settings
- Preview fixes before applying

## Extension Settings

- `lsp-bridge.executablePath`: Path to the LSP Bridge executable (default: `lsp-bridge`)
- `lsp-bridge.exportFormat`: Default export format (`json`, `markdown`, `claude`)
- `lsp-bridge.privacyLevel`: Privacy filtering level (`default`, `strict`, `permissive`)
- `lsp-bridge.includeContext`: Include code context around diagnostics
- `lsp-bridge.contextLines`: Number of context lines to include
- `lsp-bridge.autoExportOnSave`: Automatically export diagnostics on file save
- `lsp-bridge.quickFixThreshold`: Minimum confidence for auto-applying fixes (0.0-1.0)

## Privacy Levels

- **Default**: Removes sensitive information like API keys and passwords
- **Strict**: Additional filtering of file paths and identifiers
- **Permissive**: Minimal filtering for trusted environments

## Status Bar

The status bar shows current diagnostic counts:
- 🔴 Red background: Errors present
- 🟡 Yellow background: Warnings present  
- ✅ Green check: No issues

Click the status bar item to quickly export diagnostics.

## Troubleshooting

1. **"LSP Bridge not found"**: Ensure `lsp-bridge` is installed and in PATH
2. **"No diagnostics found"**: Wait for language servers to fully initialize
3. **Export fails**: Check Output panel for detailed error messages

## Contributing

Report issues and contribute at: https://github.com/your-org/lsp-bridge

## License

MIT