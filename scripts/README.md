# LSPbridge Scripts

Utility scripts for development, testing, and CI/CD automation.

## 📊 Performance & Benchmarks

### `benchmark-dashboard.sh` 
**Comprehensive benchmark dashboard with CI regression detection**

```bash
# Run complete benchmark analysis
./scripts/benchmark-dashboard.sh run

# Setup directories only
./scripts/benchmark-dashboard.sh setup

# Archive current results
./scripts/benchmark-dashboard.sh archive
```

**Features:**
- 🚨 Automated regression detection (15% threshold)
- 📈 Historical performance tracking  
- 📊 HTML/Markdown report generation
- 🔄 CI/CD integration with failure alerts
- 📁 Automatic archiving and cleanup

### `generate_benchmark_charts.py`
**Performance visualization and chart generation**

```bash
# Generate interactive charts
python3 scripts/generate_benchmark_charts.py

# With custom benchmark directory
BENCHMARK_DIR=./custom-results python3 scripts/generate_benchmark_charts.py
```

**Generates:**
- Performance trend charts
- Group comparison visualizations  
- Regression analysis graphs
- Interactive HTML dashboard
- Mobile-friendly responsive design

### `test-benchmark-dashboard.sh`
**Local testing and validation**

```bash
# Test complete dashboard setup
./scripts/test-benchmark-dashboard.sh
```

**Validates:**
- Script permissions and dependencies
- Benchmark compilation and execution
- Visualization library availability
- Configuration file presence

## 🔧 Configuration

### `benchmark-config.toml`
Central configuration for all benchmark settings:

```toml
[thresholds]
performance_regression_percent = 15.0
memory_increase_percent = 20.0

[benchmarks]
sample_size = 100
warm_up_time = 3
```

## 📚 Documentation

See [`docs/BENCHMARK_DASHBOARD.md`](../docs/BENCHMARK_DASHBOARD.md) for comprehensive documentation including:

- 🎯 Architecture overview
- 🚀 Quick start guide  
- 📊 Metrics and thresholds
- 🔄 CI/CD integration
- 🛠️ Troubleshooting guide

## 🚀 Usage Examples

### Quick Performance Check
```bash
# Run benchmarks and generate dashboard
cargo bench && ./scripts/benchmark-dashboard.sh run

# View results
open benchmark-results/reports/index.html
```

### CI Integration
The benchmark dashboard automatically runs in GitHub Actions on:
- ✅ Push to main branch
- ✅ Pull requests  
- ✅ Daily scheduled runs

### Custom Thresholds
Edit `benchmark-config.toml` to customize regression detection:

```toml
[thresholds]
performance_regression_percent = 10.0  # More strict
memory_increase_percent = 15.0         # More strict
```

## 🔍 Troubleshooting

### Common Issues

**Benchmarks not running:**
```bash
cargo build --benches  # Check compilation
cargo bench --dry-run  # Verify benchmark discovery
```

**Dashboard generation failing:**
```bash
./scripts/test-benchmark-dashboard.sh  # Run diagnostics
chmod +x scripts/*.sh                  # Fix permissions
```

**Missing visualizations:**
```bash
python3 -m pip install matplotlib seaborn pandas  # Install deps
```

## 🤝 Contributing

When adding new scripts:

1. ✅ Add execute permissions: `chmod +x script.sh`
2. ✅ Include usage documentation
3. ✅ Add error handling and logging
4. ✅ Test with `./scripts/test-benchmark-dashboard.sh`
5. ✅ Update this README

---

**Maintained by**: LSPbridge Team  
**Last Updated**: 2025-08-09