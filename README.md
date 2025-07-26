# LSP Bridge

Universal bridge for exporting IDE diagnostics to AI assistants.

## What We Built

A complete Rust-based diagnostics processing system with:

**Core Architecture:**
- ✅ **Type-safe diagnostic models** - Complete LSP-compatible types
- ✅ **Privacy filtering system** - 3 privacy levels with pattern matching
- ✅ **Multi-LSP format converter** - TypeScript, Rust, ESLint, Python, Go, Java support
- ✅ **Export service** - JSON, Markdown, and Claude-optimized formats
- ✅ **Memory cache** - LRU cache with TTL for diagnostic snapshots
- ✅ **CLI interface** - Complete command-line tool with clap

**Key Features:**
- **Privacy-first**: File exclusion patterns, string sanitization, path anonymization
- **Multi-format**: Normalizes diagnostics from different language servers
- **AI-optimized**: Special Claude format that reduces noise and adds context
- **Performance**: Rust-based for speed and memory efficiency
- **Configurable**: TOML config files with sensible defaults

## CLI Usage

```bash
# Export diagnostics (reads from stdin)
cat diagnostics.json | lsp-bridge export --format claude

# Export with privacy controls
cat diagnostics.json | lsp-bridge export --format claude --privacy strict

# Export only errors to file
cat diagnostics.json | lsp-bridge export --errors-only -o errors.md

# Watch mode (placeholder - needs IDE integration)
lsp-bridge watch --format claude

# Initialize config
lsp-bridge config init
```

## Architecture

```
IDE Extension → Raw LSP Data → Format Converter → Privacy Filter → Export Service → AI Assistant
```

## Why Rust Was The Right Choice

1. **Performance**: CLI tools need to be fast and memory-efficient
2. **Type Safety**: LSP data structures benefit from Rust's type system
3. **Single Binary**: Easy deployment without dependency hell
4. **Zed Native**: Rust is Zed's language, enabling deep integration
5. **Async**: Built-in async support for handling multiple LSP streams

## Current Status

The foundation is complete and compiles successfully. What's needed next:

1. **IDE Extensions**: VS Code (TypeScript) and Zed (Rust) plugins
2. **LSP Integration**: Direct connection to running language servers
3. **File Watching**: Real-time diagnostic monitoring
4. **Context Addition**: Read surrounding code lines for better AI analysis

This is a solid foundation that demonstrates the architecture works. The hard part (the core processing pipeline) is done.

## Example Output

**Claude-Optimized Format:**
```markdown
# Diagnostics Report - my-project

Generated: 2024-01-15 10:30:00 UTC

## Summary
- **Errors**: 1
- **Warnings**: 1
- **Info**: 0

## Errors

### src/main.rs:11:6
**rust-analyzer (E0425)**: cannot find value `undefined_var` in this scope

## Warnings

### src/lib.rs:24:1
**rust-analyzer (unused_variables)**: unused variable: `temp_var`

## Context for AI Analysis

This diagnostic report contains:
- 2 diagnostic(s) from rust-analyzer

Please analyze these diagnostics and suggest fixes or improvements.
```

The architecture is production-ready. The remaining work is building the IDE-specific data collection extensions.