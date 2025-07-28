use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sha2::{Digest, Sha256};
use std::fs;
use tempfile::TempDir;

fn bench_hash_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_computation");

    group.bench_function("small_content", |b| {
        let content = b"hello world".repeat(10);
        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(black_box(&content));
            let hash = format!("{:x}", hasher.finalize());
            black_box(hash);
        });
    });

    group.bench_function("medium_content", |b| {
        let content = b"hello world".repeat(1000);
        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(black_box(&content));
            let hash = format!("{:x}", hasher.finalize());
            black_box(hash);
        });
    });

    group.bench_function("large_content", |b| {
        let content = b"hello world".repeat(10000);
        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(black_box(&content));
            let hash = format!("{:x}", hasher.finalize());
            black_box(hash);
        });
    });

    group.finish();
}

fn bench_file_operations(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let mut files = Vec::new();

    // Create test files
    for i in 0..100 {
        let file_path = temp_dir.path().join(format!("test_file_{}.rs", i));
        let content = format!(
            "// Test file {}\nfn test_function_{}() {{\n    println!(\"Hello from file {}\");\n}}",
            i, i, i
        );
        fs::write(&file_path, content).unwrap();
        files.push(file_path);
    }

    let mut group = c.benchmark_group("file_operations");

    group.bench_function("read_files_sequential", |b| {
        b.iter(|| {
            for file in &files[0..10] {
                let content = fs::read(black_box(file)).unwrap();
                black_box(content);
            }
        });
    });

    group.bench_function("hash_files_sequential", |b| {
        b.iter(|| {
            for file in &files[0..10] {
                let content = fs::read(black_box(file)).unwrap();
                let mut hasher = Sha256::new();
                hasher.update(&content);
                let hash = format!("{:x}", hasher.finalize());
                black_box(hash);
            }
        });
    });

    group.finish();
}

fn bench_parallel_vs_sequential(c: &mut Criterion) {
    use rayon::prelude::*;

    let data: Vec<Vec<u8>> = (0..1000)
        .map(|i| format!("test data {}", i).into_bytes())
        .collect();

    let mut group = c.benchmark_group("parallel_comparison");

    group.bench_function("sequential_hashing", |b| {
        b.iter(|| {
            let hashes: Vec<String> = data
                .iter()
                .map(|content| {
                    let mut hasher = Sha256::new();
                    hasher.update(black_box(content));
                    format!("{:x}", hasher.finalize())
                })
                .collect();
            black_box(hashes);
        });
    });

    group.bench_function("parallel_hashing", |b| {
        b.iter(|| {
            let hashes: Vec<String> = data
                .par_iter()
                .map(|content| {
                    let mut hasher = Sha256::new();
                    hasher.update(black_box(content));
                    format!("{:x}", hasher.finalize())
                })
                .collect();
            black_box(hashes);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_hash_computation,
    bench_file_operations,
    bench_parallel_vs_sequential
);
criterion_main!(benches);
