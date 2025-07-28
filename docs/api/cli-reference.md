# CLI Reference

The `lsp-bridge` command-line interface provides comprehensive access to all diagnostic capture and export functionality.

## Installation

```bash
# Install from source
cargo install --path .

# Install from crates.io (when published)
cargo install lsp-bridge

# Verify installation
lsp-bridge --version
```

## Global Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--help` | `-h` | Display help information | - |
| `--version` | `-V` | Display version information | - |
| `--config` | `-c` | Path to configuration file | `.lsp-bridge.toml` |
| `--verbose` | `-v` | Enable verbose output | false |
| `--quiet` | `-q` | Suppress non-error output | false |

## Commands

### `export`

Export current diagnostics in specified format.

```bash
lsp-bridge export [OPTIONS]
```

#### Options

| Option | Short | Description | Default | Values |
|--------|-------|-------------|---------|--------|
| `--format` | `-f` | Output format | `claude` | `json`, `markdown`, `claude` |
| `--output` | `-o` | Output file path | stdout | Any file path |
| `--input` | `-i` | Input file (JSON diagnostics) | - | JSON file path |
| `--privacy` | `-p` | Privacy preset | `default` | `permissive`, `default`, `strict` |
| `--errors-only` | - | Include only error-level diagnostics | false | - |
| `--exclude` | `-e` | File patterns to exclude (repeatable) | - | Glob patterns |
| `--include-context` | - | Include code context | true | - |
| `--context-lines` | - | Number of context lines | 3 | 0-10 |
| `--no-summary` | - | Omit summary section | false | - |
| `--sanitize-strings` | - | Remove string literals | false | - |
| `--max-diagnostics` | - | Maximum diagnostics per file | 50 | 1-1000 |

#### Examples

```bash
# Basic export to stdout
lsp-bridge export

# Export to file with specific format
lsp-bridge export --format json --output diagnostics.json

# Export with strict privacy
lsp-bridge export --privacy strict --errors-only

# Export with custom exclusions
lsp-bridge export --exclude "**/test/**" --exclude "**/*.spec.ts"

# Export from saved diagnostics
lsp-bridge export --input previous-diagnostics.json --format claude
```

### `watch`

Watch for diagnostic changes and export continuously.

```bash
lsp-bridge watch [OPTIONS]
```

#### Options

All options from `export` plus:

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--interval` | - | Update interval in milliseconds | 1000 |
| `--debounce` | - | Debounce time in milliseconds | 500 |

#### Examples

```bash
# Watch and output to stdout
lsp-bridge watch --format json

# Watch with custom interval
lsp-bridge watch --interval 2000 --format claude

# Pipe to another tool
lsp-bridge watch --format json | jq '.diagnostics[]'
```

### `config`

Manage LSP Bridge configuration.

```bash
lsp-bridge config <SUBCOMMAND>
```

#### Subcommands

##### `init`

Create a new configuration file with defaults.

```bash
lsp-bridge config init [--force]
```

Options:
- `--force`: Overwrite existing configuration

##### `validate`

Validate configuration file.

```bash
lsp-bridge config validate [--file <path>]
```

##### `show`

Display current configuration.

```bash
lsp-bridge config show [--format <json|toml>]
```

#### Examples

```bash
# Initialize configuration
lsp-bridge config init

# Validate custom config
lsp-bridge config validate --file custom-config.toml

# Show current configuration as JSON
lsp-bridge config show --format json
```

### `cache`

Manage diagnostic cache.

```bash
lsp-bridge cache <SUBCOMMAND>
```

#### Subcommands

##### `clear`

Clear all cached diagnostics.

```bash
lsp-bridge cache clear [--force]
```

##### `list`

List cached snapshots.

```bash
lsp-bridge cache list [--format <table|json>]
```

##### `show`

Display a specific cached snapshot.

```bash
lsp-bridge cache show <snapshot-id>
```

#### Examples

```bash
# Clear cache
lsp-bridge cache clear --force

# List cached snapshots
lsp-bridge cache list

# Show specific snapshot
lsp-bridge cache show 123e4567-e89b-12d3-a456-426614174000
```

## Configuration File

LSP Bridge uses TOML configuration files. Default location: `.lsp-bridge.toml`

### Schema

```toml
# Privacy settings
[privacy]
exclude_patterns = [
    "**/.env*",
    "**/secrets/**",
    "**/.git/**",
    "**/node_modules/**"
]
sanitize_strings = true
sanitize_comments = false
include_only_errors = false
max_diagnostics_per_file = 50
anonymize_file_paths = false

# Export settings
[export]
format = "claude"  # json | markdown | claude
include_context = true
context_lines = 3
include_summary = true
group_by_file = false
sort_by = "severity"  # severity | file | source

# Capture settings
[capture]
real_time = true
batch_size = 100
debounce_ms = 500

# Cache settings
[cache]
max_snapshots = 100
max_age_seconds = 86400  # 24 hours
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LSP_BRIDGE_CONFIG` | Configuration file path | `.lsp-bridge.toml` |
| `LSP_BRIDGE_CACHE_DIR` | Cache directory | `~/.cache/lsp-bridge` |
| `LSP_BRIDGE_LOG_LEVEL` | Log level | `info` |
| `NO_COLOR` | Disable colored output | false |

## Exit Codes

| Code | Description |
|------|-------------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | Privacy policy violation |
| 4 | IO error |
| 5 | Serialization error |

## Privacy Presets

### Permissive
- No file exclusions
- All diagnostic levels included
- String literals preserved
- Maximum information shared

### Default (Recommended)
- Common sensitive patterns excluded
- All diagnostic levels included
- String literals preserved
- Balanced privacy/utility

### Strict
- Extensive exclusion patterns
- Only error-level diagnostics
- String literals sanitized
- Maximum privacy protection

## Output Formats

### JSON Format

Structured data suitable for programmatic processing:

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "workspace": {
    "name": "my-project",
    "rootPath": "/path/to/project"
  },
  "diagnostics": [...],
  "summary": {
    "totalDiagnostics": 42,
    "errorCount": 5,
    "warningCount": 37
  }
}
```

### Markdown Format

Human-readable format with syntax highlighting:

```markdown
# Diagnostics Report

Generated: 2024-01-15T10:30:00Z

## Summary
- Errors: 5
- Warnings: 37

## Diagnostics

### src/main.ts
...
```

### Claude Format

Optimized for AI assistant consumption with context and structure:

```markdown
# Diagnostics Report - my-project

## Summary
- **Errors**: 5
- **Warnings**: 37

## Errors

### src/main.ts:10:5
**TypeScript Error (2339)**: Property 'foo' does not exist on type 'Bar'

```typescript
// Context
function processData(data: Bar) {
  const result = data.foo; // ← Error here
  return result;
}
```
...
```

## Performance Tips

1. **Use appropriate update intervals**: Default 1000ms is good for most cases
2. **Enable debouncing**: Prevents excessive updates during rapid changes
3. **Limit diagnostics per file**: Use `--max-diagnostics` for large projects
4. **Use exclusion patterns**: Skip generated/vendor files
5. **Cache management**: Clear cache periodically with `cache clear`

## Troubleshooting

### Common Issues

**No diagnostics found**
- Ensure language servers are running in your IDE
- Check that files aren't excluded by privacy patterns
- Verify workspace path is correct

**Permission denied**
- Check file permissions for output path
- Ensure cache directory is writable

**High memory usage**
- Reduce `max_snapshots` in cache settings
- Use `--max-diagnostics` to limit output
- Clear cache with `lsp-bridge cache clear`

### Debug Mode

Enable verbose logging for troubleshooting:

```bash
LSP_BRIDGE_LOG_LEVEL=debug lsp-bridge export --verbose
```