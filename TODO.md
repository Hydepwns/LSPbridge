# LSPbridge TODO List

## High Priority Issues

### 1. Incomplete Implementations
- [ ] Implement Parquet export functionality (`src/ai_training/export.rs:155`)
- [ ] Complete manual annotation interface (`src/cli/mod.rs:1401`)
- [ ] Add dependency parsing for Maven (`src/project/build_system.rs:377`)
- [ ] Add dependency parsing for Gradle (`src/project/build_system.rs:408`)
- [ ] Add dependency parsing for Go modules (`src/project/build_system.rs:434`)
- [ ] Fix broken test helpers in quick_fix tests (`src/quick_fix/engine.rs:283,286`)
- [ ] Connect build system detection (`src/cli/multi_repo.rs:297`)
- [ ] Re-enable semantic context filtering (`src/core/semantic_context.rs:257`)
- [ ] Implement cross-file analysis (`src/core/semantic_context.rs:1003`)

### 2. Error Handling Improvements
- [ ] Replace all `unwrap()` calls with proper error handling
- [ ] Add context to error messages using `anyhow::Context`
- [ ] Implement consistent error handling patterns across all modules
- [ ] Create custom error types for domain-specific errors

### 3. Configuration Fixes
- [ ] Sync version numbers (Cargo.toml: 0.2.0 vs VS Code extension: 0.1.0)
- [ ] Make cache directory configurable and platform-aware
- [ ] Remove Cargo.lock from repository or update .gitignore
- [ ] Add configuration validation on startup
- [ ] Create platform-specific default configurations

## Medium Priority Issues

### 4. Code Quality
- [ ] Replace hardcoded paths (e.g., `/tmp/test_fix.ts`) with platform-agnostic alternatives
- [ ] Replace `any` type suggestions with proper type inference
- [ ] Convert string-based error codes to enums
- [ ] Add comprehensive doc comments to all public APIs
- [ ] Implement proper logging instead of println! macros

### 5. Performance Optimizations
- [ ] Implement connection pooling for database operations
- [ ] Add real-world benchmarks for common operations
- [ ] Optimize file scanning for large repositories
- [ ] Implement lazy loading for diagnostic data
- [ ] Add caching for expensive computations

### 6. Security Enhancements
- [ ] Add input validation for file paths and patterns
- [ ] Implement proper sanitization in privacy filter
- [ ] Add rate limiting for API endpoints
- [ ] Validate and sanitize user-provided regex patterns
- [ ] Implement secure defaults for all configurations

## Low Priority Issues

### 7. Testing Infrastructure
- [ ] Enable and fix ignored quick fix tests
- [ ] Add integration tests for multi-repo features
- [ ] Create end-to-end tests for common workflows
- [ ] Add property-based tests for parsers
- [ ] Implement snapshot testing for CLI output

### 8. Documentation
- [ ] Add comprehensive API documentation
- [ ] Create usage examples for all major features
- [ ] Write architecture documentation
- [ ] Add troubleshooting guide
- [ ] Create video tutorials for complex features

### 9. DevOps & CI/CD
- [ ] Add GitHub Actions workflow for CI
- [ ] Configure automated releases
- [ ] Add code coverage reporting
- [ ] Implement dependency vulnerability scanning
- [ ] Create Docker images for easy deployment

### 10. Feature Enhancements
- [ ] Add support for more build systems (Bazel, Buck, etc.)
- [ ] Implement diagnostic trend analysis
- [ ] Add machine learning-based fix suggestions
- [ ] Create web UI for diagnostic visualization
- [ ] Add support for custom diagnostic providers

## Technical Debt
- [ ] Refactor large modules into smaller, focused components
- [ ] Standardize naming conventions across the codebase
- [ ] Remove duplicate code between analyzers
- [ ] Consolidate configuration structures
- [ ] Improve separation of concerns in CLI module

## Breaking Changes for v1.0
- [ ] Redesign configuration file format
- [ ] Standardize CLI argument naming
- [ ] Restructure export formats
- [ ] Unify error types across modules
- [ ] Stabilize public API surface

## Notes
- Priority levels are suggestions based on impact and effort
- Some tasks may have dependencies on others
- Consider creating GitHub issues for tracking progress
- Regular code reviews recommended for all changes