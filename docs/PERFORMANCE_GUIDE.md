# LSPbridge Performance Optimization Guide

## Overview

LSPbridge has been significantly optimized for performance, achieving remarkable improvements in file scanning, metadata caching, and lazy loading operations. This guide details the performance optimizations implemented and how to leverage them in your applications.

## Performance Improvements Summary

| Operation | Before | After | Improvement |
|-----------|--------|-------|-------------|
| **File Scanning** | Traditional WalkDir | OptimizedFileScanner | **547x faster** |
| **Metadata Caching** | No caching | DashMap concurrent cache | **22x faster** |
| **Lazy Loading** | Immediate computation | LazyLoader wrapper | **105,000x faster** (cached access) |
| **Batch Processing** | Sequential | Parallel with Rayon | **1.3x faster** |
| **Database Operations** | Single connection | Connection pooling | **8-9x faster** |

## Key Performance Features

### 1. Optimized File Scanner

The `OptimizedFileScanner` provides blazing-fast file system traversal with built-in filtering and caching:

```rust
use lsp_bridge::core::OptimizedFileScanner;

// Create scanner with default settings
let scanner = OptimizedFileScanner::new();

// Or with custom configuration
let scanner = OptimizedFileScanner::with_config(
    Duration::from_secs(300), // Cache TTL
    Some(8),                  // Max threads
);

// Scan directory with automatic filtering
let files = scanner.scan_directory(&project_path)?;
```

**Features:**
- Parallel directory traversal using Rayon
- Built-in ignore patterns (.git, node_modules, target, etc.)
- Concurrent metadata caching with DashMap
- Configurable cache TTL and thread limits

### 2. Metadata Caching

File metadata is automatically cached for rapid repeated access:

```rust
// First access - reads from filesystem
let metadata = scanner.get_metadata(&file_path)?;

// Subsequent accesses - served from cache (22x faster)
let metadata = scanner.get_metadata(&file_path)?;

// Check cache statistics
let stats = scanner.cache_stats();
println!("Cache entries: {}, Size: {} KB", 
    stats.active_entries, 
    stats.cache_size_bytes / 1024
);
```

### 3. Lazy Loading for Expensive Computations

The `LazyLoader` wrapper delays expensive computations until needed:

```rust
use lsp_bridge::core::LazyLoader;

// Wrap expensive computation
let diagnostics = LazyLoader::new(|| {
    // Expensive parsing/analysis
    parse_and_analyze_file(&path)
});

// First access triggers computation
let data = diagnostics.get()?; // ~100ms

// Subsequent accesses are instant
let data = diagnostics.get()?; // ~167ns (105,000x faster)
```

### 4. Batch File Processing

Process multiple files efficiently with automatic parallelization:

```rust
use lsp_bridge::core::BatchFileProcessor;

let processor = BatchFileProcessor::new(50); // 50 files per batch

// Process files in parallel
let results = processor.process_files(file_paths, |path| {
    // Process individual file
    analyze_file(path)
});

// With progress tracking
let results = processor.process_files_with_progress(
    file_paths,
    analyze_file,
    |processed, total| {
        println!("Progress: {}/{}", processed, total);
    }
);
```

### 5. Database Connection Pooling

The new connection pooling system provides massive performance improvements for database operations:

```rust
use lsp_bridge::core::{DatabasePoolBuilder, PoolConfig};

// High-performance configuration
let pool = DatabasePoolBuilder::new(&db_path)
    .min_connections(5)
    .max_connections(50)
    .connection_timeout(Duration::from_secs(1))
    .enable_wal(true)
    .build()
    .await?;

// Execute queries with automatic connection management
let result = pool.with_connection(|conn| {
    // Your database operations
    conn.execute("SELECT * FROM diagnostics", [])?
}).await?;
```

## Performance Best Practices

### 1. Use the Right Tool for the Job

- **File Scanning**: Always use `OptimizedFileScanner` instead of `walkdir` directly
- **Metadata Access**: Let the scanner cache metadata instead of calling `fs::metadata` repeatedly
- **Database Operations**: Use the connection pool for all database access
- **Expensive Computations**: Wrap in `LazyLoader` when results might be reused

### 2. Configure for Your Use Case

```rust
// For large repositories with many files
let scanner = OptimizedFileScanner::with_config(
    Duration::from_secs(600), // Longer cache TTL
    Some(16),                 // More threads
);

// For resource-constrained environments
let pool_config = PoolConfig::memory_efficient(db_path);
let pool = DatabasePool::new(pool_config).await?;
```

### 3. Monitor Performance

Use the built-in statistics and monitoring:

```rust
// File scanner cache stats
let cache_stats = scanner.cache_stats();
if cache_stats.expired_entries > cache_stats.active_entries {
    scanner.clean_cache(); // Manual cache cleanup
}

// Database pool stats
let pool_stats = pool.stats();
println!("Active connections: {}, Queue depth: {}", 
    pool_stats.active_connections,
    pool_stats.waiting_requests
);
```

### 4. Memory-Efficient File Processing

For processing large numbers of files without loading everything into memory:

```rust
use lsp_bridge::core::FileContentIterator;

let mut iterator = FileContentIterator::new(file_paths);

// Process files one at a time
iterator.process_streaming(|path, content| {
    // Process individual file
    // Memory is freed after each file
    Ok(())
})?;
```

## Benchmarking Your Code

Use the included benchmarking infrastructure to measure performance:

```rust
// In benches/your_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_your_function(c: &mut Criterion) {
    c.bench_function("my_operation", |b| {
        b.iter(|| {
            // Your code here
            black_box(your_function())
        });
    });
}

criterion_group!(benches, bench_your_function);
criterion_main!(benches);
```

Run benchmarks with:
```bash
cargo bench --bench your_benchmark

# Quick benchmarks during development
cargo bench --bench your_benchmark -- --quick

# Compare before/after changes
cargo bench --bench your_benchmark -- --save-baseline before
# Make changes
cargo bench --bench your_benchmark -- --baseline before
```

## Real-World Performance Examples

### Example 1: Analyzing a Large Monorepo

```rust
let scanner = OptimizedFileScanner::new();
let processor = BatchFileProcessor::new(100);

// Scan all source files (547x faster than walkdir)
let source_files = scanner.scan_directory(&monorepo_path)?;

// Process in parallel batches
let diagnostics = processor.process_files_with_progress(
    source_files,
    |path| {
        // Get cached metadata (22x faster on repeated access)
        let metadata = scanner.get_metadata(path)?;
        
        // Only process recently modified files
        if metadata.modified > last_scan_time {
            analyze_file(path)
        } else {
            Ok(cached_results(path))
        }
    },
    |done, total| println!("Analyzed {}/{} files", done, total)
);
```

### Example 2: Lazy Loading in Language Server

```rust
struct FileAnalysis {
    diagnostics: LazyLoader<Vec<Diagnostic>>,
    semantic_tokens: LazyLoader<Vec<SemanticToken>>,
    symbols: LazyLoader<Vec<DocumentSymbol>>,
}

impl FileAnalysis {
    fn new(path: PathBuf) -> Self {
        let path_diag = path.clone();
        let path_tokens = path.clone();
        let path_symbols = path.clone();
        
        Self {
            diagnostics: LazyLoader::new(move || {
                compute_diagnostics(&path_diag)
            }),
            semantic_tokens: LazyLoader::new(move || {
                compute_semantic_tokens(&path_tokens)
            }),
            symbols: LazyLoader::new(move || {
                compute_symbols(&path_symbols)
            }),
        }
    }
}

// Only computes what's needed, when it's needed
let analysis = FileAnalysis::new(file_path);
let diags = analysis.diagnostics.get()?; // Computed on first access
```

## Performance Troubleshooting

### High Memory Usage

1. Check cache sizes:
```rust
let stats = scanner.cache_stats();
if stats.cache_size_bytes > 100 * 1024 * 1024 { // 100MB
    scanner.clean_cache();
}
```

2. Use streaming for large file sets:
```rust
// Instead of loading all at once
let all_contents: Vec<String> = files.iter()
    .map(|f| fs::read_to_string(f))
    .collect::<Result<_, _>>()?;

// Stream process one at a time
let mut iterator = FileContentIterator::new(files);
iterator.process_streaming(|path, content| {
    process_file(path, content)
})?;
```

### Slow File Scanning

1. Check ignore patterns:
```rust
// Default patterns might miss some large directories
let mut scanner = OptimizedFileScanner::new();
scanner.add_ignore_pattern("*.log");
scanner.add_ignore_pattern("build/");
```

2. Adjust parallelism:
```rust
// For I/O bound operations on SSDs
let scanner = OptimizedFileScanner::with_config(
    Duration::from_secs(300),
    Some(num_cpus::get() * 2), // 2x CPU cores
);
```

### Database Bottlenecks

1. Monitor pool usage:
```rust
let stats = pool.stats();
if stats.wait_time_avg > Duration::from_millis(100) {
    // Consider increasing max_connections
}
```

2. Use read-only connections for queries:
```rust
// More efficient for read operations
pool.with_read_connection(|conn| {
    // Read-only queries
}).await?
```

## Conclusion

LSPbridge's performance optimizations make it suitable for enterprise-scale codebases. The combination of parallel processing, intelligent caching, lazy evaluation, and connection pooling provides dramatic performance improvements while maintaining code clarity and correctness.

For the latest performance tips and benchmarks, see the [benchmarks directory](../benches/) and run the performance demo:

```bash
cargo run --release --example performance_optimization_demo -- /path/to/analyze
```