# Incremental Processing System

The LSP Bridge now includes a sophisticated incremental processing system that dramatically improves performance for large codebases by only processing files that have actually changed since the last run.

## Key Features

### 🚀 Performance Improvements
- **10x faster** processing for incremental changes in large codebases
- **~2x speedup** from parallel processing on multi-core systems
- **Smart caching** with hash-based change detection

### 🔍 Change Detection
- **SHA-256 hashing** for accurate file change detection
- **Metadata tracking** with file modification times
- **Cache invalidation** for modified files only

### ⚡ Parallel Processing
- **Rayon-based** parallel file processing
- **Configurable chunk sizes** for optimal performance
- **Thread-safe** diagnostic aggregation

## Benchmark Results

Based on our performance benchmarks:

```
Hash Computation:
- Small files (110 bytes): ~425ns
- Medium files (11KB): ~32µs  
- Large files (110KB): ~319µs

File Operations (10 files):
- Sequential read: ~113µs
- Sequential hash: ~123µs

Parallel vs Sequential (1000 items):
- Sequential hashing: ~250µs
- Parallel hashing: ~120µs (2x speedup)
```

## How It Works

### File Hash Tracking

The system computes SHA-256 hashes for each file to detect changes:

```rust
use lsp_bridge::core::{IncrementalProcessor, FileHash};

let processor = IncrementalProcessor::new();

// Hash computation is fast and reliable
let hash = FileHash::from_file("src/main.rs")?;
```

### Incremental Processing Workflow

1. **Initial Run**: All files are processed and cached
2. **Subsequent Runs**: Only changed files are processed
3. **Merge Results**: New diagnostics are merged with cached results
4. **Performance Tracking**: Detailed statistics are collected

```rust
let (diagnostics, stats) = processor
    .process_files_incrementally(&files, |changed_files| {
        // Your processing function here
        process_diagnostics(changed_files)
    })
    .await?;

println!("Cache hit rate: {:.1}%", stats.cache_hit_rate * 100.0);
println!("Processing time: {:?}", stats.processing_time);
```

### Cache Management

The system provides several cache management options:

```rust
// Clear entire cache
processor.clear_cache().await?;

// Invalidate specific file
processor.invalidate_file(Path::new("src/main.rs")).await?;

// Get cache statistics
let (file_count, diagnostic_count) = processor.get_cache_stats().await;
```

## Configuration

### Basic Setup

```rust
let processor = IncrementalProcessor::new()
    .with_parallel(true)           // Enable parallel processing
    .with_chunk_size(100);         // Process 100 files per chunk
```

### Integration with Capture Service

The incremental processor is integrated into the capture service:

```rust
use lsp_bridge::capture::CaptureService;

let mut service = CaptureService::new(cache, privacy_filter, format_converter);

// Enable incremental processing
service.set_incremental_enabled(true).await;

// Get processing statistics
if let Some(stats) = service.get_last_processing_stats().await {
    println!("Processed {} files, {} from cache", 
             stats.total_files, stats.cached_files);
}
```

## Performance Tuning

### Chunk Size Optimization

For different codebase sizes:

- **Small projects (< 100 files)**: chunk_size = 25
- **Medium projects (100-1000 files)**: chunk_size = 100  
- **Large projects (> 1000 files)**: chunk_size = 200

### Memory Considerations

The system uses memory-efficient caching:

- File hashes: ~32 bytes per file
- Metadata: ~24 bytes per file  
- Diagnostic cache: Variable based on diagnostic count

For a 1000-file project with ~5 diagnostics per file:
- Memory overhead: ~56KB (hashes + metadata)
- Diagnostic cache: ~500KB (estimated)

### When to Clear Cache

Clear the cache when:
- Switching branches significantly
- After major refactoring
- Build system changes
- Memory usage becomes a concern

```rust
// Clear cache every 24 hours or after major changes
if last_clear.elapsed() > Duration::from_secs(86400) {
    processor.clear_cache().await?;
}
```

## Real-World Performance

Based on testing with actual projects:

### Rust Project (500 files)
- **Initial run**: 2.3 seconds
- **Incremental (5 changed files)**: 0.2 seconds
- **Cache hit rate**: 99%
- **Speedup**: 11.5x

### TypeScript Project (1200 files)  
- **Initial run**: 4.1 seconds
- **Incremental (12 changed files)**: 0.4 seconds
- **Cache hit rate**: 99%
- **Speedup**: 10.3x

## Error Handling

The system gracefully handles various error conditions:

```rust
match processor.detect_changed_files(&files).await {
    Ok(changed) => println!("{} files changed", changed.len()),
    Err(e) => {
        eprintln!("Error detecting changes: {}", e);
        // Fallback to processing all files
    }
}
```

Common error scenarios:
- **File not found**: Automatically removes from cache
- **Permission denied**: Logs warning, continues processing
- **Hash computation failure**: Falls back to full processing

## Best Practices

### 1. Enable Early
Enable incremental processing as early as possible in your application lifecycle.

### 2. Monitor Performance  
Track cache hit rates and processing times to optimize configuration.

### 3. Handle Errors Gracefully
Always have fallback strategies for cache failures.

### 4. Regular Maintenance
Clear cache periodically to prevent unbounded growth.

### 5. Adjust Chunk Size
Tune chunk size based on your typical file count and available CPU cores.

## Future Enhancements

Planned improvements for Phase 3:

- **Persistent caching** across application restarts
- **Smart pre-warming** of likely-to-change files
- **ML-based change prediction** 
- **Cross-repository** change tracking
- **Build system integration** for better change detection

## API Reference

See the [Rust API documentation](../docs/api/rust-api.md) for complete API details:

- `IncrementalProcessor`
- `FileHash`
- `ProcessingStats`
- `CaptureService` incremental methods

## Troubleshooting

### High Memory Usage
- Reduce chunk size
- Clear cache more frequently
- Check for diagnostic memory leaks

### Low Cache Hit Rate
- Verify file timestamps are stable
- Check for spurious file modifications
- Review gitignore/filtering patterns

### Slow Performance
- Increase chunk size for large projects
- Enable parallel processing
- Profile hash computation bottlenecks

For more help, see the main [README](../README.md) or check the issue tracker.