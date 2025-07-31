//! Performance Optimization Demo
//!
//! Demonstrates the performance improvements from using the OptimizedFileScanner
//! and other performance optimization utilities.

use anyhow::Result;
use lsp_bridge::core::{
    BatchFileProcessor, FileContentIterator, LazyLoader, OptimizedFileScanner,
};
use std::path::Path;
use std::time::{Duration, Instant};
use walkdir::WalkDir;

fn main() -> Result<()> {
    println!("LSPbridge Performance Optimization Demo\n");

    // Setup test directory (use current directory or provide a path)
    let test_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".".to_string());
    let test_path = Path::new(&test_path);

    println!("Analyzing directory: {}\n", test_path.display());

    // Benchmark 1: Traditional vs Optimized File Scanning
    benchmark_file_scanning(test_path)?;

    // Benchmark 2: Cached vs Non-cached Metadata Access
    benchmark_metadata_caching(test_path)?;

    // Benchmark 3: Sequential vs Parallel Batch Processing
    benchmark_batch_processing(test_path)?;

    // Benchmark 4: Lazy Loading Demo
    demo_lazy_loading()?;

    Ok(())
}

fn benchmark_file_scanning(path: &Path) -> Result<()> {
    println!("=== File Scanning Benchmark ===\n");

    // Traditional approach with WalkDir
    let start = Instant::now();
    let traditional_files: Vec<_> = WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let path = e.path();
            !path.to_string_lossy().contains("node_modules")
                && !path.to_string_lossy().contains(".git")
                && !path.to_string_lossy().contains("target")
        })
        .map(|e| e.path().to_owned())
        .collect();
    let traditional_time = start.elapsed();

    // Optimized approach
    let scanner = OptimizedFileScanner::new();
    let start = Instant::now();
    let optimized_files = scanner.scan_directory(path)?;
    let optimized_time = start.elapsed();

    println!("Traditional WalkDir:");
    println!("  Files found: {}", traditional_files.len());
    println!("  Time taken: {:?}", traditional_time);
    println!();

    println!("Optimized Scanner:");
    println!("  Files found: {}", optimized_files.len());
    println!("  Time taken: {:?}", optimized_time);
    println!();

    let speedup = traditional_time.as_secs_f64() / optimized_time.as_secs_f64();
    println!("Speedup: {:.2}x\n", speedup);

    Ok(())
}

fn benchmark_metadata_caching(path: &Path) -> Result<()> {
    println!("=== Metadata Caching Benchmark ===\n");

    let scanner = OptimizedFileScanner::new();
    let files = scanner.scan_directory(path)?;
    
    // Take a sample of files
    let sample_files: Vec<_> = files.iter().take(100).collect();

    // First pass - cold cache
    let start = Instant::now();
    for file in &sample_files {
        let _ = scanner.get_metadata(file)?;
    }
    let cold_cache_time = start.elapsed();

    // Second pass - warm cache
    let start = Instant::now();
    for file in &sample_files {
        let _ = scanner.get_metadata(file)?;
    }
    let warm_cache_time = start.elapsed();

    // Cache statistics
    let stats = scanner.cache_stats();

    println!("Cold cache (first access):");
    println!("  Time: {:?}", cold_cache_time);
    println!();

    println!("Warm cache (second access):");
    println!("  Time: {:?}", warm_cache_time);
    println!();

    println!("Cache Statistics:");
    println!("  Total entries: {}", stats.total_entries);
    println!("  Active entries: {}", stats.active_entries);
    println!("  Cache size: {} KB", stats.cache_size_bytes / 1024);
    println!();

    let speedup = cold_cache_time.as_secs_f64() / warm_cache_time.as_secs_f64();
    println!("Cache speedup: {:.2}x\n", speedup);

    Ok(())
}

fn benchmark_batch_processing(path: &Path) -> Result<()> {
    println!("=== Batch Processing Benchmark ===\n");

    let scanner = OptimizedFileScanner::new();
    let files = scanner.scan_directory(path)?;
    
    // Take a reasonable sample
    let sample_files: Vec<_> = files.iter().take(500).cloned().collect();

    // Sequential processing
    let start = Instant::now();
    let mut sequential_results = Vec::new();
    for file in &sample_files {
        let result = process_file(file);
        sequential_results.push(result);
    }
    let sequential_time = start.elapsed();

    // Parallel batch processing
    let processor = BatchFileProcessor::new(50); // 50 files per batch
    let start = Instant::now();
    let parallel_results = processor.process_files(sample_files.clone(), process_file);
    let parallel_time = start.elapsed();

    println!("Sequential Processing:");
    println!("  Files processed: {}", sequential_results.len());
    println!("  Time: {:?}", sequential_time);
    println!();

    println!("Parallel Batch Processing:");
    println!("  Files processed: {}", parallel_results.len());
    println!("  Time: {:?}", parallel_time);
    println!();

    let speedup = sequential_time.as_secs_f64() / parallel_time.as_secs_f64();
    println!("Parallel speedup: {:.2}x\n", speedup);

    // Demo progress tracking
    println!("Processing with progress tracking:");
    let processor = BatchFileProcessor::new(25);
    let _results = processor.process_files_with_progress(
        sample_files[..100].to_vec(),
        process_file,
        |processed, total| {
            print!("\rProgress: {}/{} ({:.1}%)", processed, total, 
                   (processed as f64 / total as f64) * 100.0);
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
        },
    );
    println!("\n");

    Ok(())
}

fn demo_lazy_loading() -> Result<()> {
    println!("=== Lazy Loading Demo ===\n");

    // Simulate expensive computation
    let expensive_data = LazyLoader::new(|| {
        println!("  Computing expensive data...");
        std::thread::sleep(Duration::from_millis(100));
        Ok(vec![1, 2, 3, 4, 5])
    });

    println!("Lazy loader created");
    println!("  Is loaded: {}", expensive_data.is_loaded());
    println!();

    println!("First access:");
    let start = Instant::now();
    let data1 = expensive_data.get()?;
    let first_access_time = start.elapsed();
    println!("  Data: {:?}", data1);
    println!("  Time: {:?}", first_access_time);
    println!("  Is loaded: {}", expensive_data.is_loaded());
    println!();

    println!("Second access (cached):");
    let start = Instant::now();
    let data2 = expensive_data.get()?;
    let second_access_time = start.elapsed();
    println!("  Data: {:?}", data2);
    println!("  Time: {:?}", second_access_time);
    println!();

    let speedup = first_access_time.as_micros() as f64 / second_access_time.as_micros().max(1) as f64;
    println!("Lazy loading speedup: {:.0}x\n", speedup);

    Ok(())
}

// Simulated file processing function
fn process_file(path: &Path) -> Result<usize> {
    // Simulate some work
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_performance_demo() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create some test files
        for i in 0..10 {
            std::fs::write(
                temp_dir.path().join(format!("test_{}.txt", i)),
                format!("Test content {}", i),
            ).unwrap();
        }

        // Run benchmarks
        benchmark_file_scanning(temp_dir.path()).unwrap();
        benchmark_metadata_caching(temp_dir.path()).unwrap();
        benchmark_batch_processing(temp_dir.path()).unwrap();
        demo_lazy_loading().unwrap();
    }
}