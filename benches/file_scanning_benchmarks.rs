//! File Scanning and Repository Analysis Performance Benchmarks
//!
//! Focused on identifying and optimizing bottlenecks in large-scale file operations

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;
use walkdir::WalkDir;

/// Create a fake repository structure for benchmarking
fn create_test_repository(root: &Path, num_files: usize, _depth: usize) -> Result<(), std::io::Error> {
    // Create a realistic project structure
    let dirs = vec![
        "src/core",
        "src/utils",
        "src/services",
        "src/models",
        "tests/unit",
        "tests/integration",
        "docs",
        "examples",
    ];

    for dir in &dirs {
        fs::create_dir_all(root.join(dir))?;
    }

    // Distribute files across directories
    let files_per_dir = num_files / dirs.len();
    let extensions = vec!["rs", "ts", "js", "py", "go", "java"];

    for (dir_idx, dir) in dirs.iter().enumerate() {
        for i in 0..files_per_dir {
            let ext = &extensions[i % extensions.len()];
            let filename = format!("file_{}.{}", i + dir_idx * files_per_dir, ext);
            let filepath = root.join(dir).join(filename);
            
            // Create realistic file content based on extension
            let content = match ext {
                &"rs" => generate_rust_content(i),
                &"ts" | &"js" => generate_typescript_content(i),
                &"py" => generate_python_content(i),
                &"go" => generate_go_content(i),
                &"java" => generate_java_content(i),
                _ => format!("// File {}\n", i),
            };
            
            fs::write(filepath, content)?;
        }
    }

    // Add some common config files
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"\n")?;
    fs::write(root.join("package.json"), r#"{"name": "test", "version": "0.1.0"}"#)?;
    fs::write(root.join(".gitignore"), "/target\n/node_modules\n")?;
    fs::write(root.join("README.md"), "# Test Repository\n")?;

    Ok(())
}

/// Benchmark basic file scanning with walkdir
fn bench_file_scanning_walkdir(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_scanning_walkdir");
    
    let file_counts = vec![
        ("small", 100),
        ("medium", 1000),
        ("large", 5000),
        ("xlarge", 10000),
    ];

    for (size_name, num_files) in file_counts {
        let temp_dir = TempDir::new().unwrap();
        create_test_repository(temp_dir.path(), num_files, 3).unwrap();

        group.throughput(Throughput::Elements(num_files as u64));
        group.bench_with_input(
            BenchmarkId::new("sequential", size_name),
            &temp_dir,
            |b, dir| {
                b.iter(|| {
                    let mut count = 0;
                    for entry in WalkDir::new(dir.path())
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                    {
                        count += 1;
                        black_box(&entry);
                    }
                    black_box(count)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parallel file scanning with rayon
fn bench_file_scanning_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_scanning_parallel");
    
    let file_counts = vec![
        ("small", 100),
        ("medium", 1000),
        ("large", 5000),
        ("xlarge", 10000),
    ];

    for (size_name, num_files) in file_counts {
        let temp_dir = TempDir::new().unwrap();
        create_test_repository(temp_dir.path(), num_files, 3).unwrap();

        group.throughput(Throughput::Elements(num_files as u64));
        group.bench_with_input(
            BenchmarkId::new("rayon", size_name),
            &temp_dir,
            |b, dir| {
                b.iter(|| {
                    let entries: Vec<_> = WalkDir::new(dir.path())
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                        .collect();
                    
                    let count: usize = entries
                        .par_iter()
                        .map(|entry| {
                            // Simulate some processing
                            black_box(&entry);
                            1
                        })
                        .sum();
                    
                    black_box(count)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark file filtering strategies
fn bench_file_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_filtering");
    
    let temp_dir = TempDir::new().unwrap();
    create_test_repository(temp_dir.path(), 5000, 3).unwrap();

    // Collect all paths first
    let all_paths: Vec<PathBuf> = WalkDir::new(temp_dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_owned())
        .collect();

    group.bench_function("extension_string_ends_with", |b| {
        b.iter(|| {
            let filtered: Vec<_> = all_paths
                .iter()
                .filter(|p| {
                    p.to_string_lossy().ends_with(".rs") ||
                    p.to_string_lossy().ends_with(".ts") ||
                    p.to_string_lossy().ends_with(".js")
                })
                .collect();
            black_box(filtered.len())
        });
    });

    group.bench_function("extension_path_extension", |b| {
        b.iter(|| {
            let filtered: Vec<_> = all_paths
                .iter()
                .filter(|p| {
                    matches!(
                        p.extension().and_then(|s| s.to_str()),
                        Some("rs") | Some("ts") | Some("js")
                    )
                })
                .collect();
            black_box(filtered.len())
        });
    });

    group.bench_function("gitignore_pattern_matching", |b| {
        let patterns = vec![
            "*.log",
            "node_modules/",
            "target/",
            "*.tmp",
            ".git/",
        ];
        
        b.iter(|| {
            let filtered: Vec<_> = all_paths
                .iter()
                .filter(|p| {
                    let path_str = p.to_string_lossy();
                    !patterns.iter().any(|pattern| {
                        if pattern.ends_with('/') {
                            path_str.contains(pattern)
                        } else if pattern.starts_with("*.") {
                            path_str.ends_with(&pattern[1..])
                        } else {
                            path_str.contains(pattern)
                        }
                    })
                })
                .collect();
            black_box(filtered.len())
        });
    });

    group.finish();
}

/// Benchmark file content reading strategies
fn bench_file_reading(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_reading");
    group.sample_size(50);
    
    let temp_dir = TempDir::new().unwrap();
    create_test_repository(temp_dir.path(), 100, 3).unwrap();
    
    let files: Vec<PathBuf> = WalkDir::new(temp_dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "rs"))
        .map(|e| e.path().to_owned())
        .take(50)
        .collect();

    group.bench_function("sequential_read", |b| {
        b.iter(|| {
            let mut total_size = 0;
            for file in &files {
                if let Ok(content) = fs::read_to_string(file) {
                    total_size += content.len();
                    black_box(&content);
                }
            }
            black_box(total_size)
        });
    });

    group.bench_function("parallel_read", |b| {
        b.iter(|| {
            let total_size: usize = files
                .par_iter()
                .map(|file| {
                    fs::read_to_string(file)
                        .map(|content| {
                            black_box(&content);
                            content.len()
                        })
                        .unwrap_or(0)
                })
                .sum();
            black_box(total_size)
        });
    });

    group.bench_function("mmap_read", |b| {
        use memmap2::Mmap;
        use std::fs::File;
        
        b.iter(|| {
            let mut total_size = 0;
            for file in &files {
                if let Ok(f) = File::open(file) {
                    if let Ok(mmap) = unsafe { Mmap::map(&f) } {
                        total_size += mmap.len();
                        black_box(&mmap[..]);
                    }
                }
            }
            black_box(total_size)
        });
    });

    group.finish();
}

/// Benchmark repository structure analysis
fn bench_repo_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("repo_analysis");
    group.sample_size(20);
    
    let repo_sizes = vec![
        ("small", 500),
        ("medium", 2000),
        ("large", 5000),
    ];

    for (size_name, num_files) in repo_sizes {
        let temp_dir = TempDir::new().unwrap();
        create_test_repository(temp_dir.path(), num_files, 4).unwrap();

        group.throughput(Throughput::Elements(num_files as u64));
        
        // TODO: Re-enable these benchmarks once the API is stabilized
        // group.bench_with_input(
        //     BenchmarkId::new("build_system_detection", size_name),
        //     &temp_dir,
        //     |b, dir| {
        //         b.iter(|| {
        //             use lsp_bridge::project::BuildSystemDetector;
        //             let detector = BuildSystemDetector::new();
        //             black_box(detector.detect_build_systems(dir.path()))
        //         });
        //     },
        // );

        // group.bench_with_input(
        //     BenchmarkId::new("monorepo_detection", size_name),
        //     &temp_dir,
        //     |b, dir| {
        //         b.iter(|| {
        //             let detector = MonorepoDetector::new();
        //             black_box(detector.detect_structure(dir.path()).unwrap())
        //         });
        //     },
        // );
    }

    group.finish();
}

/// Benchmark caching strategies for file metadata
fn bench_metadata_caching(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_caching");
    
    let temp_dir = TempDir::new().unwrap();
    create_test_repository(temp_dir.path(), 1000, 3).unwrap();
    
    let files: Vec<PathBuf> = WalkDir::new(temp_dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_owned())
        .collect();

    group.bench_function("no_cache", |b| {
        b.iter(|| {
            let mut total_size = 0u64;
            for file in &files {
                if let Ok(metadata) = fs::metadata(file) {
                    total_size += metadata.len();
                }
            }
            black_box(total_size)
        });
    });

    group.bench_function("hashmap_cache", |b| {
        use std::collections::HashMap;
        
        b.iter(|| {
            let mut cache = HashMap::new();
            let mut total_size = 0u64;
            
            for file in &files {
                let size = cache.entry(file.clone()).or_insert_with(|| {
                    fs::metadata(file).map(|m| m.len()).unwrap_or(0)
                });
                total_size += *size;
            }
            
            // Simulate cache hits
            for file in &files[..files.len()/2] {
                if let Some(size) = cache.get(file) {
                    total_size += *size;
                }
            }
            
            black_box(total_size)
        });
    });

    group.bench_function("dashmap_concurrent_cache", |b| {
        use dashmap::DashMap;
        use std::sync::Arc;
        
        b.iter(|| {
            let cache = Arc::new(DashMap::new());
            let cache_clone = cache.clone();
            
            let total_size: u64 = files
                .par_iter()
                .map(|file| {
                    cache_clone
                        .entry(file.clone())
                        .or_insert_with(|| {
                            fs::metadata(file).map(|m| m.len()).unwrap_or(0)
                        })
                        .value()
                        .clone()
                })
                .sum();
            
            black_box(total_size)
        });
    });

    group.finish();
}

// Helper functions to generate realistic file content

fn generate_rust_content(index: usize) -> String {
    format!(
        r#"use std::collections::HashMap;
use anyhow::Result;

/// Documentation for struct {}
#[derive(Debug, Clone)]
pub struct Component{} {{
    id: String,
    data: HashMap<String, String>,
    count: usize,
}}

impl Component{} {{
    pub fn new(id: String) -> Self {{
        Self {{
            id,
            data: HashMap::new(),
            count: 0,
        }}
    }}
    
    pub fn process(&mut self) -> Result<()> {{
        self.count += 1;
        // Complex logic here
        Ok(())
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;
    
    #[test]
    fn test_component() {{
        let mut comp = Component{}::new("test".to_string());
        assert_eq!(comp.count, 0);
    }}
}}
"#,
        index, index, index, index
    )
}

fn generate_typescript_content(index: usize) -> String {
    format!(
        r#"import {{ Injectable }} from '@angular/core';
import {{ Observable }} from 'rxjs';

interface Data{} {{
    id: string;
    value: number;
    metadata: Record<string, any>;
}}

@Injectable({{
    providedIn: 'root'
}})
export class Service{} {{
    private data: Data{}[] = [];
    
    constructor() {{}}
    
    getData(): Observable<Data{}[]> {{
        return new Observable(observer => {{
            observer.next(this.data);
            observer.complete();
        }});
    }}
    
    addData(item: Data{}): void {{
        this.data.push(item);
    }}
}}
"#,
        index, index, index, index, index
    )
}

fn generate_python_content(index: usize) -> String {
    format!(
        r#"import asyncio
from typing import List, Dict, Optional
from dataclasses import dataclass

@dataclass
class Model{}:
    """Model class for item {}"""
    id: str
    data: Dict[str, any]
    count: int = 0
    
    def process(self) -> bool:
        """Process the model data"""
        self.count += 1
        return True

class Service{}:
    def __init__(self):
        self.models: List[Model{}] = []
    
    async def fetch_data(self) -> List[Model{}]:
        """Fetch data asynchronously"""
        await asyncio.sleep(0.1)
        return self.models
    
    def add_model(self, model: Model{}) -> None:
        """Add a new model"""
        self.models.append(model)
"#,
        index, index, index, index, index, index
    )
}

fn generate_go_content(index: usize) -> String {
    format!(
        r#"package service{}

import (
    "context"
    "fmt"
    "sync"
)

// Service{} handles operations for component {}
type Service{} struct {{
    mu    sync.RWMutex
    data  map[string]interface{{}}
    count int
}}

// NewService{} creates a new service instance
func NewService{}() *Service{} {{
    return &Service{}{{
        data: make(map[string]interface{{}}),
    }}
}}

// Process handles the main logic
func (s *Service{}) Process(ctx context.Context) error {{
    s.mu.Lock()
    defer s.mu.Unlock()
    
    s.count++
    return nil
}}

// GetCount returns the current count
func (s *Service{}) GetCount() int {{
    s.mu.RLock()
    defer s.mu.RUnlock()
    return s.count
}}
"#,
        index, index, index, index, index, index, index, index, index, index
    )
}

fn generate_java_content(index: usize) -> String {
    format!(
        r#"package com.example.service{};

import java.util.*;
import java.util.concurrent.*;

/**
 * Service class for component {}
 */
public class Service{} {{
    private final Map<String, Object> data;
    private int count;
    
    public Service{}() {{
        this.data = new ConcurrentHashMap<>();
        this.count = 0;
    }}
    
    public void process() {{
        synchronized(this) {{
            count++;
        }}
    }}
    
    public int getCount() {{
        return count;
    }}
    
    public static void main(String[] args) {{
        Service{} service = new Service{}();
        service.process();
    }}
}}
"#,
        index, index, index, index, index, index
    )
}

criterion_group! {
    name = file_scanning_benches;
    config = Criterion::default()
        .sample_size(20)
        .measurement_time(Duration::from_secs(10));
    targets = 
        bench_file_scanning_walkdir,
        bench_file_scanning_parallel,
        bench_file_filtering,
        bench_file_reading,
        bench_repo_analysis,
        bench_metadata_caching
}

criterion_main!(file_scanning_benches);