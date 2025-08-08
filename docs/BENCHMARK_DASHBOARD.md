# LSPbridge Benchmark Dashboard

A comprehensive performance monitoring and regression detection system for LSPbridge.

## 🎯 Overview

The Benchmark Dashboard provides automated performance tracking with the following features:

- **📊 Performance Monitoring**: Tracks 7+ benchmark groups across multiple metrics
- **🚨 Regression Detection**: Automatically detects performance regressions (>15% threshold)
- **📈 Trend Analysis**: Historical performance tracking and visualization  
- **🔄 CI/CD Integration**: Automatic benchmarks on every push and PR
- **📝 Automated Reporting**: Generated performance reports and charts
- **⚡ Real-time Alerts**: CI failures on performance regressions

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   CI Pipeline   │───▶│ Benchmark Runner │───▶│ Result Storage  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │                        │
                                ▼                        ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ Regression      │◀───│ Analysis Engine  │◀───│ Historical Data │
│ Detection       │    └──────────────────┘    └─────────────────┘
└─────────────────┘             │
         │                      ▼
         ▼              ┌──────────────────┐
┌─────────────────┐    │ Visualization    │
│ Alert System    │    │ Generator        │
└─────────────────┘    └──────────────────┘
         │                      │
         ▼                      ▼
┌─────────────────┐    ┌─────────────────┐
│ CI Status       │    │ GitHub Pages    │
│ Updates         │    │ Dashboard       │
└─────────────────┘    └─────────────────┘
```

## 🚀 Quick Start

### 1. Run Benchmarks Locally

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench -- context_extraction

# Generate dashboard (requires setup)
./scripts/benchmark-dashboard.sh run
```

### 2. View Results

```bash
# View latest results
open benchmark-results/reports/index.html

# View CLI summary
cat benchmark-results/latest/latest.txt
```

### 3. Test Setup

```bash
# Test dashboard configuration
./scripts/test-benchmark-dashboard.sh
```

## 📊 Benchmark Groups

| Group | Description | Target | Priority |
|-------|-------------|---------|-----------|
| **Context Extraction** | File parsing and semantic analysis | < 50ms | High |
| **Context Ranking** | Relevance scoring algorithms | < 10ms | High |
| **Diagnostic Prioritization** | Error categorization and sorting | < 20ms | High |
| **Memory Usage** | Memory consumption patterns | < 100MB | Medium |
| **Concurrent Throughput** | Parallel processing efficiency | 2x factor | Medium |
| **Cache Performance** | Cache hit rates and speeds | 80% hit rate | Medium |
| **Cold Start** | Initialization performance | < 200ms | Low |

## 🔧 Configuration

### Benchmark Settings (`benchmark-config.toml`)

```toml
[thresholds]
performance_regression_percent = 15.0
memory_increase_percent = 20.0
cache_hit_rate_decrease_percent = 10.0

[benchmarks]
sample_size = 100
warm_up_time = 3
measurement_time = 5

[alerts]
enabled = true
fail_ci_on_regression = true
```

### Environment Variables

```bash
# Benchmark configuration
export BENCHMARK_DIR="./benchmark-results"
export PERFORMANCE_THRESHOLD=15
export MEMORY_THRESHOLD=20

# CI integration
export CI=true
export GITHUB_SHA="commit_hash"
export GITHUB_REF_NAME="branch_name"
```

## 🚨 Regression Detection

### Thresholds

- **Performance Regression**: 15% increase in execution time
- **Memory Regression**: 20% increase in memory usage
- **Cache Regression**: 10% decrease in hit rate

### Alert Levels

1. **🟢 Good**: Performance within acceptable range
2. **🟡 Warning**: 8-15% performance degradation  
3. **🔴 Critical**: >15% regression (fails CI)

### Example Alert

```markdown
## ⚠️ Performance Regressions Detected

| Benchmark | Baseline | Current | Regression |
|-----------|----------|---------|------------|
| context_extraction/large | 45.2ms | 52.8ms | +16.8% |
| memory_usage/concurrent | 89MB | 112MB | +25.8% |

**Recommendation**: Review recent changes and consider rollback
```

## 📈 Metrics Tracked

### Performance Metrics
- Execution time (mean, median, std dev)
- Throughput (operations/second)
- Memory usage (peak, average)
- Cache efficiency (hit rate, miss penalty)

### System Metrics
- CPU utilization
- Memory allocation patterns
- I/O operations
- Concurrent scaling factors

### Trend Metrics
- Performance over time
- Regression frequency
- Improvement tracking
- Baseline drift

## 🔄 CI/CD Integration

### GitHub Actions Workflow

The benchmark runs automatically on:

- ✅ **Push to main**: Full benchmark suite
- ✅ **Pull requests**: Performance comparison  
- ✅ **Daily schedule**: Baseline updates
- ❌ **Draft PRs**: Skipped (configurable)

### Workflow Steps

1. **Environment Setup**: Rust toolchain + dependencies
2. **Cache Restoration**: Previous benchmark data
3. **Benchmark Execution**: Full criterion suite
4. **Regression Analysis**: Compare with baseline
5. **Report Generation**: Markdown + HTML + Charts
6. **Result Storage**: Archive + GitHub Pages
7. **PR Comments**: Automatic performance feedback

### Example PR Comment

```markdown
## 🚀 Benchmark Results

**Generated**: 2025-08-09 00:54:19
**Commit**: a1b2c3d4
**Branch**: feature/optimization

## 📊 Performance Summary

| Group | Count | Avg Time | Fastest | Slowest |
|-------|-------|----------|---------|---------|
| context_extraction | 4 | 42.3ms | 28.1ms | 65.7ms |
| context_ranking | 6 | 8.7ms | 3.2ms | 18.4ms |

## 🚀 Performance Improvements

| Benchmark | Baseline | Current | Improvement |
|-----------|----------|---------|-------------|
| cache_performance/hot_cache | 12.4ms | 9.8ms | -21.0% |
```

## 🎨 Visualization

### Charts Generated

1. **Performance Trends**: Time series of benchmark results
2. **Group Comparisons**: Performance by benchmark category
3. **Regression Analysis**: Visual regression detection
4. **Memory Usage**: Memory consumption patterns
5. **Cache Efficiency**: Hit rates and performance impact
6. **Historical Overview**: Long-term performance trends

### Interactive Dashboard

The dashboard includes:

- **📊 Live Charts**: Interactive performance visualizations
- **📋 Detailed Tables**: Sortable benchmark results
- **🔍 Drill-down**: Individual benchmark analysis
- **📱 Responsive**: Mobile-friendly design
- **🔗 Navigation**: Links to raw data and archives

## 📁 File Structure

```
benchmark-results/
├── latest/                    # Most recent results
│   ├── latest.json           # Latest benchmark data
│   ├── latest.txt            # Human-readable results
│   └── *_parsed.json         # Processed benchmark data
├── archive/                   # Historical results
│   └── YYYYMMDD_HHMMSS_hash/ # Timestamped archives
├── reports/                   # Generated reports
│   ├── index.html            # Main dashboard
│   ├── performance_*.md      # Markdown reports
│   ├── regression_*.json     # Regression analysis
│   └── *.png                 # Generated charts
└── baseline.json             # Current performance baseline
```

## 🛠️ Troubleshooting

### Common Issues

#### Benchmarks Failing
```bash
# Check compilation
cargo build --benches

# Run single benchmark for debugging  
cargo bench -- context_extraction --nocapture
```

#### Dashboard Not Generating
```bash
# Check script permissions
chmod +x scripts/*.sh

# Test dashboard setup
./scripts/test-benchmark-dashboard.sh

# Check dependencies
python3 -m pip install matplotlib seaborn pandas
```

#### CI Integration Issues
```bash
# Verify workflow syntax
gh workflow view benchmarks

# Check CI logs
gh run view --log
```

### Debug Mode

Enable verbose logging:

```bash
export RUST_LOG=debug
export BENCHMARK_DEBUG=1
./scripts/benchmark-dashboard.sh run
```

## 🔍 Advanced Usage

### Custom Baseline

```bash
# Set custom baseline from specific commit
git checkout baseline-commit
cargo bench
cp benchmark-results/latest/latest_parsed.json benchmark-results/baseline.json
```

### Benchmark Comparison

```bash
# Compare two commits
git checkout commit1
cargo bench
mv benchmark-results/latest results-commit1

git checkout commit2
cargo bench
# Manual comparison with results-commit1
```

### Performance Profiling

```bash
# Enable detailed profiling
export BENCHMARK_PROFILE=1
cargo bench

# Generate flamegraph
cargo install flamegraph
cargo flamegraph --bench lsp_bridge_benchmarks
```

## 🤝 Contributing

### Adding New Benchmarks

1. Add benchmark function to `benches/lsp_bridge_benchmarks.rs`
2. Update `benchmark-config.toml` with target metrics
3. Test locally: `cargo bench -- new_benchmark`
4. Update documentation

### Improving Visualizations

1. Modify `scripts/generate_benchmark_charts.py`
2. Test with: `python3 scripts/generate_benchmark_charts.py`
3. Check generated charts in `benchmark-results/reports/`

### Customizing Thresholds

Edit `benchmark-config.toml`:

```toml
[thresholds]
performance_regression_percent = 10.0  # More strict
memory_increase_percent = 15.0         # More strict
```

## 📚 References

- [Criterion.rs Documentation](https://docs.rs/criterion/)
- [GitHub Actions Benchmarking](https://docs.github.com/en/actions)  
- [Performance Testing Best Practices](https://github.com/microsoft/perfview/blob/main/documentation/Guides/BestPractices.md)
- [Statistical Analysis of Benchmarks](https://www.brendangregg.com/blog/2018-02-09/kpti-kaiser-meltdown-performance.html)

---

**Generated**: 2025-08-09
**Version**: LSPbridge v0.3.1
**Maintainer**: LSPbridge Team