# Extension API Reference

This document covers the APIs for building LSP Bridge extensions for various IDEs.

## VS Code Extension API

### Installation

```bash
npm install lsp-bridge-vscode
```

### Core Components

#### `DiagnosticsCollector`

Collects diagnostics from VS Code's language servers.

```typescript
import { DiagnosticsCollector } from 'lsp-bridge-vscode';
import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
    const collector = new DiagnosticsCollector(bridgeInterface);
    
    // Start real-time capture
    collector.startRealTimeCapture();
    
    // Export diagnostics
    const diagnostics = await collector.exportDiagnostics();
    
    // Stop capture
    collector.stopRealTimeCapture();
}
```

##### Methods

```typescript
class DiagnosticsCollector {
    constructor(bridgeInterface: BridgeInterface);
    
    // Start/stop real-time diagnostic capture
    startRealTimeCapture(): void;
    stopRealTimeCapture(): void;
    
    // Export current diagnostics
    async exportDiagnostics(): Promise<string | null>;
    
    // Get raw diagnostic snapshot
    createSnapshot(): DiagnosticSnapshot | null;
    
    // Cleanup resources
    dispose(): void;
}
```

#### `BridgeInterface`

Communicates with the LSP Bridge CLI or library.

```typescript
import { BridgeInterface, ConfigManager } from 'lsp-bridge-vscode';

const config = new ConfigManager();
const bridge = new BridgeInterface(config);

// Test connection to CLI
const isConnected = await bridge.testConnection();

// Export diagnostics snapshot
const output = await bridge.exportSnapshot(snapshot);
```

##### Methods

```typescript
class BridgeInterface {
    constructor(configManager: ConfigManager);
    
    // Export diagnostic snapshot
    async exportSnapshot(snapshot: DiagnosticSnapshot): Promise<string>;
    
    // Test CLI availability
    async testConnection(): Promise<boolean>;
    
    // Get CLI executable path
    getExecutablePath(): string | null;
}
```

#### `ConfigManager`

Manages extension configuration and VS Code settings integration.

```typescript
import { ConfigManager } from 'lsp-bridge-vscode';

const config = new ConfigManager();

// Get configuration
const settings = config.getConfiguration();

// Update settings
await config.updateConfiguration('privacy.sanitizeStrings', true);

// Show configuration wizard
await config.showConfigurationWizard();
```

##### Configuration Schema

```typescript
interface ExtensionConfiguration {
    privacy: {
        excludePatterns: string[];
        sanitizeStrings: boolean;
        includeOnlyErrors: boolean;
    };
    export: {
        format: 'json' | 'markdown' | 'claude';
        includeContext: boolean;
    };
    capture: {
        realTime: boolean;
    };
}
```

### Command Registration

```typescript
// Register commands in extension.ts
const commands = [
    {
        id: 'lspBridge.exportDiagnostics',
        handler: async () => {
            const result = await collector.exportDiagnostics();
            // Handle result
        }
    },
    {
        id: 'lspBridge.configure',
        handler: () => config.showConfigurationWizard()
    }
];

commands.forEach(cmd => {
    context.subscriptions.push(
        vscode.commands.registerCommand(cmd.id, cmd.handler)
    );
});
```

### Events and Subscriptions

```typescript
// Listen for configuration changes
vscode.workspace.onDidChangeConfiguration(event => {
    if (event.affectsConfiguration('lspBridge')) {
        config.reload();
        // React to changes
    }
});

// Listen for diagnostic changes
vscode.languages.onDidChangeDiagnostics(event => {
    // Process changed diagnostics
    for (const uri of event.uris) {
        const diagnostics = vscode.languages.getDiagnostics(uri);
        // Handle diagnostics
    }
});
```

## Zed Extension API

### Setup

```rust
// Cargo.toml
[dependencies]
zed_extension_api = "0.0.6"
lsp_bridge = { path = "../.." }

[lib]
crate-type = ["cdylib"]
```

### Core Implementation

```rust
use zed_extension_api::{self as zed, Result};
use lsp_bridge::{DiagnosticsCapture, ExportService, PrivacyPolicy};

struct LspBridgeExtension {
    capture_service: DiagnosticsCapture,
    export_service: ExportService,
}

#[export]
impl zed::Extension for LspBridgeExtension {
    fn new() -> Self {
        let mut capture = DiagnosticsCapture::new();
        capture.set_privacy_policy(PrivacyPolicy::default());
        
        Self {
            capture_service: capture,
            export_service: ExportService::new(),
        }
    }
    
    fn command_palette_commands(&self) -> Vec<zed::Command> {
        vec![
            zed::Command {
                name: "Export Diagnostics".into(),
                action: Box::new(|_| {
                    self.export_diagnostics()
                }),
            },
            zed::Command {
                name: "Configure LSP Bridge".into(),
                action: Box::new(|_| {
                    self.show_configuration()
                }),
            },
        ]
    }
}
```

### Diagnostic Collection

```rust
impl LspBridgeExtension {
    fn collect_diagnostics(&mut self) -> Result<Vec<Diagnostic>> {
        // Get diagnostics from Zed
        let diagnostics = zed::diagnostics::get_all()?;
        
        // Convert to LSP Bridge format
        let converted = diagnostics.into_iter()
            .map(|d| self.convert_diagnostic(d))
            .collect();
            
        Ok(converted)
    }
    
    fn export_diagnostics(&mut self) -> Result<()> {
        let diagnostics = self.collect_diagnostics()?;
        let snapshot = self.capture_service.create_snapshot(diagnostics)?;
        let output = self.export_service.to_claude_format(&snapshot)?;
        
        // Show in new buffer
        zed::workspace::open_buffer_with_content(output)?;
        
        Ok(())
    }
}
```

## IntelliJ/JetBrains Extension API

### Plugin Configuration

```xml
<!-- plugin.xml -->
<idea-plugin>
    <id>com.lspbridge.intellij</id>
    <name>LSP Bridge</name>
    <vendor>LSP Bridge Team</vendor>
    
    <depends>com.intellij.modules.platform</depends>
    
    <extensions defaultExtensionNs="com.intellij">
        <applicationService 
            serviceImplementation="com.lspbridge.DiagnosticsService"/>
        <projectService 
            serviceImplementation="com.lspbridge.BridgeService"/>
    </extensions>
    
    <actions>
        <action id="LspBridge.Export" 
                class="com.lspbridge.ExportAction"
                text="Export Diagnostics">
            <keyboard-shortcut first-keystroke="ctrl shift D"/>
        </action>
    </actions>
</idea-plugin>
```

### Service Implementation

```kotlin
import com.intellij.openapi.components.Service
import com.intellij.openapi.project.Project
import com.lspbridge.LspBridgeLib

@Service
class BridgeService(private val project: Project) {
    private val bridge = LspBridgeLib()
    
    fun exportDiagnostics(): String {
        val problems = ProblemsView.getInstance(project).getAllProblems()
        val diagnostics = problems.map { convertProblem(it) }
        
        return bridge.export(diagnostics, ExportFormat.CLAUDE)
    }
    
    private fun convertProblem(problem: Problem): Diagnostic {
        return Diagnostic(
            file = problem.virtualFile.path,
            range = convertRange(problem.textRange),
            severity = convertSeverity(problem.severity),
            message = problem.description
        )
    }
}
```

## Neovim Extension API

### Lua API

```lua
-- lua/lsp-bridge/init.lua
local M = {}

-- Configuration
M.config = {
    privacy = {
        exclude_patterns = { "**/.env*", "**/secrets/**" },
        sanitize_strings = true,
    },
    export = {
        format = "claude",
        include_context = true,
    }
}

-- Setup function
function M.setup(opts)
    M.config = vim.tbl_deep_extend("force", M.config, opts or {})
    
    -- Register commands
    vim.api.nvim_create_user_command("LspBridgeExport", M.export_diagnostics, {})
    vim.api.nvim_create_user_command("LspBridgeConfig", M.show_config, {})
    
    -- Set up keymaps
    vim.keymap.set("n", "<leader>de", M.export_diagnostics, { desc = "Export diagnostics" })
end

-- Export diagnostics
function M.export_diagnostics()
    local diagnostics = vim.diagnostic.get()
    local formatted = M.format_diagnostics(diagnostics)
    
    -- Call CLI
    local cmd = string.format("lsp-bridge export --format %s", M.config.export.format)
    local output = vim.fn.system(cmd, vim.json.encode(formatted))
    
    -- Show in new buffer
    vim.cmd("new")
    vim.api.nvim_buf_set_lines(0, 0, -1, false, vim.split(output, "\n"))
end

return M
```

### Usage

```lua
-- In init.lua or init.vim
require("lsp-bridge").setup({
    privacy = {
        sanitize_strings = false,
    },
    export = {
        format = "markdown",
    }
})
```

## Common Patterns

### Privacy Configuration

All extensions should respect privacy settings:

```typescript
// TypeScript/JavaScript
const defaultPrivacy = {
    excludePatterns: [
        '**/.env*',
        '**/secrets/**',
        '**/.git/**',
        '**/node_modules/**'
    ],
    sanitizeStrings: true,
    includeOnlyErrors: false
};

// Apply filters before export
function applyPrivacyFilter(diagnostics: Diagnostic[]): Diagnostic[] {
    return diagnostics
        .filter(d => !matchesExcludePattern(d.file))
        .map(d => sanitizeDiagnostic(d));
}
```

### Error Handling

Consistent error handling across extensions:

```typescript
try {
    const result = await bridge.exportSnapshot(snapshot);
    showSuccess("Diagnostics exported successfully");
} catch (error) {
    if (error.code === 'CLI_NOT_FOUND') {
        showError("LSP Bridge CLI not found. Please install it first.");
    } else if (error.code === 'PRIVACY_VIOLATION') {
        showWarning("Some diagnostics were filtered due to privacy settings.");
    } else {
        showError(`Export failed: ${error.message}`);
    }
}
```

### Configuration Storage

Store user preferences appropriately:

```typescript
// VS Code - Use workspace/global settings
await config.update('lspBridge.privacy.sanitizeStrings', true);

// IntelliJ - Use PropertiesComponent
PropertiesComponent.getInstance().setValue("lsp.bridge.format", "claude");

// Neovim - Use vim.g or setup function
vim.g.lsp_bridge_config = { format = "json" }
```

## Testing Extensions

### VS Code Extension Testing

```typescript
import * as assert from 'assert';
import * as vscode from 'vscode';
import { DiagnosticsCollector } from '../src/diagnosticsCollector';

suite('Extension Test Suite', () => {
    test('Should collect diagnostics', async () => {
        const collector = new DiagnosticsCollector(mockBridge);
        const snapshot = collector.createSnapshot();
        
        assert.ok(snapshot);
        assert.ok(Array.isArray(snapshot.diagnostics));
    });
});
```

### Zed Extension Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_diagnostic_conversion() {
        let zed_diagnostic = create_test_diagnostic();
        let converted = convert_diagnostic(zed_diagnostic);
        
        assert_eq!(converted.severity, DiagnosticSeverity::Error);
    }
}
```

## Publishing Extensions

### VS Code Marketplace

```bash
# Install vsce
npm install -g vsce

# Package extension
vsce package

# Publish (requires publisher account)
vsce publish
```

### JetBrains Plugin Repository

1. Build plugin: `./gradlew buildPlugin`
2. Upload to https://plugins.jetbrains.com
3. Wait for approval

### Vim/Neovim Package Managers

Support multiple package managers:

```lua
-- lazy.nvim
{
    "lsp-bridge/lsp-bridge.nvim",
    config = function()
        require("lsp-bridge").setup()
    end
}

-- packer.nvim
use {
    'lsp-bridge/lsp-bridge.nvim',
    config = function()
        require("lsp-bridge").setup()
    end
}
```

## Best Practices

1. **Lazy Loading**: Initialize services only when needed
2. **Resource Cleanup**: Always dispose of resources properly
3. **User Feedback**: Provide clear progress indicators
4. **Error Messages**: Make errors actionable for users
5. **Performance**: Debounce rapid diagnostic changes
6. **Privacy First**: Default to conservative privacy settings
7. **Testing**: Include comprehensive test suites
8. **Documentation**: Provide inline help and command descriptions