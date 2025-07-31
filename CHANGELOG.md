# Changelog

All notable changes to LSP Bridge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2024-01-31

### Added
- **Configuration System Overhaul**
  - New unified configuration file: `lspbridge.toml` (renamed from `lsp-bridge-config.toml`)
  - Profile support for development, production, and testing environments
  - Language-specific configuration sections for Rust, TypeScript, Go, and Python
  - Security configuration section with file path limits and secret scanning
  - Network configuration for future distributed mode
  
- **Performance Enhancements**
  - Tiered caching system (hot/warm/cold) for optimized memory usage
  - Cache compression support via flate2
  - Memory pressure detection for adaptive resource management
  - System resource monitoring with sysinfo
  - Increased default memory allocation (256MB → 512MB)
  - Optimized chunk size (100 → 500) for better throughput
  
- **Monitoring & Observability**
  - OpenTelemetry support for distributed tracing
  - Custom metrics tracking (cache hit rates, processing time by type)
  - Enhanced Prometheus metrics with configurable collection intervals
  
- **Git Integration Improvements**
  - Branch-aware caching for better multi-branch workflows
  - `.gitignore` pattern support
  - Reduced scan frequency (30s → 90s) to minimize overhead
  
- **Error Handling**
  - Retry jitter to prevent thundering herd problems
  - Error categorization for different error types
  - Enhanced circuit breaker configuration
  
- **Development Experience**
  - New dev dependencies: mockall, proptest, test-case, serial_test
  - Feature flags for CLI, git integration, network, and experimental features
  - Improved VS Code extension metadata with proper author and repository info

### Changed
- Configuration file renamed from `lsp-bridge-config.toml` to `lspbridge.toml`
- Default cache size increased from 100MB to 500MB
- Max concurrent files reduced from 1000 to 200 for better resource management
- Git scan interval increased from 30s to 90s
- Metrics collection interval reduced from 10s to 30s
- Package name in Cargo.toml updated to match crates.io: `lspbridge`
- Repository URLs updated to `https://github.com/Hydepwns/LSPbridge`
- Author information updated to DROO <drew@axol.io>

### Fixed
- Resource exhaustion issues with high concurrent file counts
- Cache performance with larger codebases
- Memory management in constrained environments

### Security
- Added maximum file path length limits
- Symlink restrictions outside workspace
- Secret scanning capabilities
- Allowed file extensions whitelist

## [0.2.0] - 2024-01-15

### Added
- Multi-repository support with monorepo detection
- Enhanced caching with persistent storage
- Performance optimizations achieving 547x faster file scanning
- Database connection pooling for 8-9x faster operations
- Comprehensive test suite with 98.7% coverage

### Changed
- Improved diagnostic formatting for AI consumption
- Enhanced privacy filtering with workspace-aware patterns
- Better error recovery with circuit breaker pattern

## [0.1.0] - 2024-01-01

### Added
- Initial release
- Basic LSP diagnostic export functionality
- Support for Rust, TypeScript, Python, Go, and Java
- CLI interface with multiple export formats
- Privacy filtering with three levels
- VS Code and Neovim extensions

[0.3.0]: https://github.com/Hydepwns/LSPbridge/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Hydepwns/LSPbridge/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Hydepwns/LSPbridge/releases/tag/v0.1.0