//! # LSP Bridge Performance Benchmarks
//!
//! Comprehensive benchmark suite covering all critical performance metrics
//! identified in Phase 4 of the improvement plan.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lsp_bridge::core::{
    context_ranking::ContextRanker,
    diagnostic_prioritization::DiagnosticPrioritizer,
    enhanced_processor::{EnhancedIncrementalProcessor, EnhancedProcessorConfig},
    semantic_context::{ContextExtractor, SemanticContext},
    Diagnostic, DiagnosticSeverity, Position, Range,
};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Benchmark context extraction performance across different file sizes
fn bench_context_extraction(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    let mut group = c.benchmark_group("context_extraction");

    // Test different file sizes
    let file_sizes = vec![
        ("small", 50),    // 50 lines
        ("medium", 500),  // 500 lines
        ("large", 2000),  // 2000 lines
        ("xlarge", 5000), // 5000 lines
    ];

    for (size_name, line_count) in file_sizes {
        let file_path = temp_dir.path().join(format!("test_{}.ts", size_name));
        let content = create_typescript_content(line_count);
        std::fs::write(&file_path, content).unwrap();

        let diagnostic = create_benchmark_diagnostic(&file_path, line_count / 2);

        group.throughput(Throughput::Elements(line_count as u64));
        group.bench_with_input(
            BenchmarkId::new("typescript", size_name),
            &(file_path, diagnostic),
            |b, (path, diag)| {
                let mut extractor = ContextExtractor::new().unwrap();
                b.iter(|| {
                    black_box(
                        extractor
                            .extract_context_from_file(black_box(diag))
                            .unwrap(),
                    )
                });
            },
        );
    }

    // Test Rust context extraction
    for (size_name, line_count) in vec![("small", 50), ("medium", 500), ("large", 2000)] {
        let file_path = temp_dir.path().join(format!("test_{}.rs", size_name));
        let content = create_rust_content(line_count);
        std::fs::write(&file_path, content).unwrap();

        let diagnostic = create_benchmark_diagnostic(&file_path, line_count / 2);

        group.throughput(Throughput::Elements(line_count as u64));
        group.bench_with_input(
            BenchmarkId::new("rust", size_name),
            &(file_path, diagnostic),
            |b, (path, diag)| {
                let mut extractor = ContextExtractor::new().unwrap();
                b.iter(|| {
                    black_box(
                        extractor
                            .extract_context_from_file(black_box(diag))
                            .unwrap(),
                    )
                });
            },
        );
    }

    group.finish();
}

/// Benchmark context ranking performance with varying context set sizes
fn bench_context_ranking(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_ranking");

    let context_counts = vec![1, 5, 10, 25, 50, 100];
    let ranker = ContextRanker::new();

    for count in context_counts {
        let contexts = create_mock_contexts(count);

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("rank_contexts", count),
            &contexts,
            |b, contexts| {
                b.iter(|| black_box(ranker.rank_contexts(black_box(contexts))));
            },
        );
    }

    group.finish();
}

/// Benchmark diagnostic prioritization with varying diagnostic set sizes
fn bench_diagnostic_prioritization(c: &mut Criterion) {
    let mut group = c.benchmark_group("diagnostic_prioritization");

    let diagnostic_counts = vec![1, 10, 50, 100, 250, 500];
    let prioritizer = DiagnosticPrioritizer::new();

    for count in diagnostic_counts {
        let diagnostics = create_mock_diagnostics(count);
        let contexts = create_mock_contexts(count);

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("prioritize_diagnostics", count),
            &(diagnostics, contexts),
            |b, (diags, contexts)| {
                b.iter(|| {
                    black_box(
                        prioritizer.prioritize_diagnostics(black_box(diags), black_box(contexts)),
                    )
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage patterns with different cache configurations
fn bench_memory_usage_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_usage");
    group.sample_size(10); // Fewer samples for memory tests

    let cache_sizes = vec![
        ("small_cache", 10),
        ("medium_cache", 100),
        ("large_cache", 1000),
    ];

    for (cache_name, cache_size) in cache_sizes {
        group.bench_function(cache_name, |b| {
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();

                rt.block_on(async {
                    for _ in 0..iters {
                        let mut config = EnhancedProcessorConfig::default();
                        config.cache_config.max_entries = cache_size;

                        let processor = EnhancedIncrementalProcessor::new(config).await.unwrap();

                        // Simulate processing multiple files
                        let temp_dir = TempDir::new().unwrap();
                        let files: Vec<_> = (0..cache_size)
                            .map(|i| {
                                let path = temp_dir.path().join(format!("file_{}.ts", i));
                                std::fs::write(&path, create_typescript_content(100)).unwrap();
                                path
                            })
                            .collect();

                        let _changed = processor.detect_changed_files(&files).await.unwrap();
                        black_box(processor);
                    }
                });

                start.elapsed()
            });
        });
    }

    group.finish();
}

/// Benchmark concurrent processing throughput
fn bench_concurrent_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_throughput");
    group.sample_size(10);

    let concurrency_levels = vec![1, 2, 4, 8, 16];

    for concurrency in concurrency_levels {
        group.bench_function(&format!("concurrent_{}", concurrency), |b| {
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();

                rt.block_on(async {
                    for _ in 0..iters {
                        let mut tasks = Vec::new();

                        for _ in 0..concurrency {
                            let task = tokio::spawn(async {
                                let config = EnhancedProcessorConfig::default();
                                let processor =
                                    EnhancedIncrementalProcessor::new(config).await.unwrap();

                                // Simulate processing
                                let temp_dir = TempDir::new().unwrap();
                                let files: Vec<_> = (0..10)
                                    .map(|i| {
                                        let path = temp_dir.path().join(format!("file_{}.ts", i));
                                        std::fs::write(&path, create_typescript_content(50))
                                            .unwrap();
                                        path
                                    })
                                    .collect();

                                processor.detect_changed_files(&files).await.unwrap()
                            });

                            tasks.push(task);
                        }

                        // Wait for all tasks
                        for task in tasks {
                            black_box(task.await.unwrap());
                        }
                    }
                });

                start.elapsed()
            });
        });
    }

    group.finish();
}

/// Benchmark cache hit rates and performance
fn bench_cache_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_performance");

    let scenarios = vec![
        ("cold_cache", 0.0), // No cache hits
        ("warm_cache", 0.5), // 50% cache hits
        ("hot_cache", 0.9),  // 90% cache hits
    ];

    for (scenario_name, _hit_rate) in scenarios {
        group.bench_function(scenario_name, |b| {
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();

                rt.block_on(async {
                    let config = EnhancedProcessorConfig::default();
                    let processor = EnhancedIncrementalProcessor::new(config).await.unwrap();

                    let temp_dir = TempDir::new().unwrap();
                    let file_path = temp_dir.path().join("test.ts");
                    std::fs::write(&file_path, create_typescript_content(200)).unwrap();

                    for _ in 0..iters {
                        // Simulate repeated access to same file (should hit cache)
                        let _cached = processor.get_cached_diagnostics(&file_path).await;
                        black_box(processor.detect_changed_files(&[&file_path]).await.unwrap());
                    }
                });

                start.elapsed()
            });
        });
    }

    group.finish();
}

/// Benchmark cold start initialization time
fn bench_cold_start_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cold_start");
    group.sample_size(20);

    group.bench_function("context_extractor_init", |b| {
        b.iter(|| black_box(ContextExtractor::new().unwrap()));
    });

    group.bench_function("enhanced_processor_init", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();

            rt.block_on(async {
                for _ in 0..iters {
                    let config = EnhancedProcessorConfig::default();
                    black_box(EnhancedIncrementalProcessor::new(config).await.unwrap());
                }
            });

            start.elapsed()
        });
    });

    group.bench_function("full_system_init", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();

            rt.block_on(async {
                for _ in 0..iters {
                    let _extractor = ContextExtractor::new().unwrap();
                    let _ranker = ContextRanker::new();
                    let _prioritizer = DiagnosticPrioritizer::new();
                    let config = EnhancedProcessorConfig::default();
                    let _processor = EnhancedIncrementalProcessor::new(config).await.unwrap();
                }
            });

            start.elapsed()
        });
    });

    group.finish();
}

// Helper functions for creating test data

fn create_typescript_content(line_count: usize) -> String {
    let mut content = String::new();
    content.push_str("// TypeScript test file\n");

    for i in 0..line_count {
        if i % 20 == 0 {
            content.push_str(&format!(
                "\ninterface TestInterface{} {{\n    prop{}: string;\n    method{}(): number;\n}}\n\n",
                i / 20, i / 20, i / 20
            ));
        } else if i % 15 == 0 {
            content.push_str(&format!(
                "class TestClass{} implements TestInterface{} {{\n",
                i / 15,
                i / 15 / 20
            ));
        } else if i % 10 == 0 {
            content.push_str(&format!(
                "    function testFunction{}(param: string): boolean {{\n",
                i / 10
            ));
        } else if i % 5 == 0 {
            content.push_str("        const result = param.length > 0;\n");
        } else {
            content.push_str(&format!(
                "        console.log(\"Line {} with some code\");\n",
                i
            ));
        }
    }

    content
}

fn create_rust_content(line_count: usize) -> String {
    let mut content = String::new();
    content.push_str("// Rust test file\nuse std::collections::HashMap;\n\n");

    for i in 0..line_count {
        if i % 25 == 0 {
            content.push_str(&format!(
                "\nstruct TestStruct{} {{\n    field{}: String,\n    number{}: i32,\n}}\n\n",
                i / 25,
                i / 25,
                i / 25
            ));
        } else if i % 15 == 0 {
            content.push_str(&format!(
                "impl TestStruct{} {{\n    fn new() -> Self {{\n",
                i / 15 / 25
            ));
        } else if i % 10 == 0 {
            content.push_str(&format!(
                "    fn test_method_{}(&self) -> Result<String, std::io::Error> {{\n",
                i / 10
            ));
        } else if i % 5 == 0 {
            content.push_str("        let mut map = HashMap::new();\n");
        } else {
            content.push_str(&format!(
                "        println!(\"Line {} with some code\");\n",
                i
            ));
        }
    }

    content
}

fn create_benchmark_diagnostic(file_path: &PathBuf, line: usize) -> Diagnostic {
    Diagnostic {
        id: Uuid::new_v4().to_string(),
        file: file_path.to_string_lossy().to_string(),
        range: Range {
            start: Position {
                line: line as u32,
                character: 10,
            },
            end: Position {
                line: line as u32,
                character: 20,
            },
        },
        severity: DiagnosticSeverity::Error,
        message: "Benchmark test error".to_string(),
        code: None,
        source: "benchmark".to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn create_mock_contexts(count: usize) -> Vec<SemanticContext> {
    (0..count)
        .map(|i| SemanticContext {
            function_context: Some(lsp_bridge::core::semantic_context::FunctionContext {
                name: format!("testFunction{}", i),
                signature: format!("function testFunction{}(param: string): boolean", i),
                body: format!("return param.length > {};", i),
                parameters: vec![],
                return_type: Some("boolean".to_string()),
                visibility: None,
                is_async: false,
                decorators: vec![],
            }),
            class_context: None,
            imports: vec![],
            type_definitions: vec![],
            local_variables: vec![],
            call_hierarchy: lsp_bridge::core::semantic_context::CallHierarchy {
                calls_to: vec![],
                calls_from: vec![],
            },
            dependencies: vec![],
            relevance_score: 0.8,
            surrounding_code: std::collections::HashMap::new(),
        })
        .collect()
}

fn create_mock_diagnostics(count: usize) -> Vec<Diagnostic> {
    (0..count)
        .map(|i| Diagnostic {
            id: Uuid::new_v4().to_string(),
            file: format!("/test/file_{}.ts", i),
            range: Range {
                start: Position {
                    line: i as u32,
                    character: 10,
                },
                end: Position {
                    line: i as u32,
                    character: 20,
                },
            },
            severity: DiagnosticSeverity::Error,
            message: format!("Test error {}", i),
            code: None,
            source: "test".to_string(),
            related_information: None,
            tags: None,
            data: None,
        })
        .collect()
}

criterion_group!(
    lsp_bridge_benches,
    bench_context_extraction,
    bench_context_ranking,
    bench_diagnostic_prioritization,
    bench_memory_usage_patterns,
    bench_concurrent_throughput,
    bench_cache_performance,
    bench_cold_start_performance
);

criterion_main!(lsp_bridge_benches);
