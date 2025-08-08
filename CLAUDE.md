# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LSPbridge is a high-performance Rust-based diagnostic bridge that connects IDEs with AI assistants. It acts as a universal translator between Language Server Protocol (LSP) diagnostics and AI-optimized formats.

## Essential Commands

### Testing
```bash
# Run all tests with LSP detection
./test_runner.sh

# Run specific test suites
cargo test --lib                    # Unit tests (108/108 passing)
cargo test --test integration       # Integration tests
cargo test --test multi_repo_test   # Multi-repo tests

# Run a single test
cargo test test_name

# Run benchmarks
cargo bench
```

### Building & Running
```bash
# Install locally
cargo install --path .

# Run with CLI args
cargo run -- export --format claude
cargo run -- export --format json --output diagnostics.json

# Development build
cargo build
cargo build --release
```

### CLI Tool Usage
```bash
# Export diagnostics
lspbridge export --format claude --privacy strict --errors-only
lspbridge export --format json --output diagnostics.json

# Real-time monitoring
lspbridge watch --format claude --interval 500

# Configuration management
lspbridge config init              # Create default config
lspbridge config show             # Display current settings
lspbridge config validate         # Check validity

# Advanced features
lspbridge query --interactive      # Interactive diagnostic queries
lspbridge history analyze         # Historical trend analysis
lspbridge ai-training export       # Generate ML training data
lspbridge quick-fix apply          # Automated code fixes
```

## Architecture Overview

### Processing Pipeline
```
IDE Extension → Raw LSP Data → Format Converter → Privacy Filter → Export Service → AI Assistant
```

### Core Components

1. **Capture Layer** (`src/capture/`)
   - `capture_service.rs`: Main entry point for diagnostic capture
   - `memory_cache.rs`: In-memory caching with LRU eviction

2. **Format Conversion** (`src/format/`)
   - Language-specific converters (rust_analyzer, typescript, eslint)
   - Unified output formatting for AI consumption

3. **Privacy Layer** (`src/privacy/`)
   - Pattern-based sensitive data filtering
   - Three configurable levels: default/strict/permissive

4. **Language Analyzers** (`src/analyzers/`)
   - `rust_analyzer/`: Rust-specific diagnostic processing
   - `typescript_analyzer/`: TypeScript/JavaScript handling
   - Each analyzer has context extraction and fix suggestion modules

5. **Export Services** (`src/export/`)
   - Multiple output formats (JSON, Markdown, Claude-optimized)
   - Streaming and batch export capabilities

6. **Performance Optimization** (`src/core/`)
   - `database_pool/`: Connection pooling with SQLite + Sled
   - `context_ranking/`: AI context optimization algorithms
   - `memory_manager/`: Adaptive memory management with tiered caching

7. **Multi-Repository Support** (`src/multi_repo/`)
   - Cross-repository analysis and dependency tracking
   - Monorepo detection (Cargo workspaces, npm/pnpm/lerna, Nx, Rush)

### Key Design Patterns

- **Plugin Architecture**: Extensible language analyzers and export formats
- **Async Processing**: Tokio-based with proper resource management
- **Circuit Breaker Pattern**: For error recovery and resilience
- **Builder Pattern**: For complex object construction (see database pool)
- **Strategy Pattern**: For different caching and optimization strategies

## Configuration System

Configuration is managed through `lspbridge.toml` (TOML format) with environment-specific profiles:

- **Main config**: `lspbridge.toml` in project root
- **Default template**: `resources/default.lspbridge.toml`
- **Environment override**: `LSP_BRIDGE_CONFIG` environment variable
- **Cache directory override**: `LSP_BRIDGE_CACHE_DIR` environment variable

Key configuration sections:
- `[processing]`: Parallel processing, chunk sizes, timeouts
- `[cache]`: Tiered caching with compression
- `[memory]`: Adaptive memory management
- `[error_recovery]`: Circuit breaker and retry logic
- `[git]`: Git integration settings
- `[privacy]`: Data sanitization rules
- `[performance]`: Optimization settings
- `[metrics]`: OpenTelemetry configuration

## Testing Patterns

When writing tests:
1. Use the mock LSP server in `tests/mock_lsp_server.rs` for protocol simulation
2. Test fixtures are in `tests/fixtures/` organized by language
3. Integration tests should check for LSP server availability before running
4. Use `cargo test --test integration test_name -- --ignored` for LSP-dependent tests

## Performance Characteristics

The codebase is optimized for:
- **547x faster** file scanning with tree-sitter parsing
- **22x faster** metadata caching with concurrent access
- **105,000x faster** lazy loading for cached computations
- **8-9x faster** database operations with connection pooling

Key performance features:
- Rayon-based parallel processing
- LRU caching with adaptive eviction
- Connection pooling for database access
- Incremental processing for large codebases

## Development Workflow

1. **Before making changes**: Check existing patterns in similar modules
2. **Error handling**: Use `anyhow::Result` with custom error types in `src/error.rs`
3. **Async code**: Use Tokio runtime, ensure proper resource cleanup
4. **Testing**: Write unit tests alongside code, integration tests in `tests/`
5. **Documentation**: Update relevant docs in `docs/` for API changes

## Language Support Status

Full support (with specialized analyzers):
- Rust (rust-analyzer)
- TypeScript/JavaScript (typescript-language-server)
- Python (pylsp, pyright)
- Go (gopls)
- Java (jdtls)

Linter integration:
- ESLint (JS/TS)
- Clippy (Rust)
- Ruff (Python)
- golangci-lint (Go)

## Common Development Tasks

### Adding a new language analyzer
1. Create module in `src/analyzers/{language}_analyzer/`
2. Implement `LanguageAnalyzer` trait from `src/analyzers/base.rs`
3. Add context extraction in `context.rs`
4. Add fix suggestions in `fixes/`
5. Register in `src/analyzers/mod.rs`
6. Add tests in `tests/integration/`

### Adding a new export format
1. Create converter in `src/format/format_converter/converters/`
2. Implement format-specific logic
3. Register in factory at `src/format/format_converter/factory.rs`
4. Add CLI support in `src/cli/commands/export.rs`

### Debugging performance issues
1. Run benchmarks: `cargo bench`
2. Check metrics endpoint if configured
3. Use `RUST_LOG=debug` for detailed logging
4. Profile with `cargo flamegraph` or `perf`

## Security Considerations

- Path validation in `src/security/path_validation.rs`
- Configurable privacy levels in `src/privacy/privacy_filter.rs`
- Rate limiting in `src/core/rate_limiter.rs`
- No secrets in logs or exports (enforced by privacy filter)
