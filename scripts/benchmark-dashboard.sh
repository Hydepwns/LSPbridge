#!/bin/bash
# Benchmark Dashboard Script
# Comprehensive performance tracking and regression detection

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BENCHMARK_DIR="${PROJECT_DIR}/benchmark-results"
LATEST_DIR="${BENCHMARK_DIR}/latest"
ARCHIVE_DIR="${BENCHMARK_DIR}/archive"
REPORTS_DIR="${BENCHMARK_DIR}/reports"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PERFORMANCE_THRESHOLD=15  # % regression threshold
MEMORY_THRESHOLD=20      # % memory increase threshold
CACHE_THRESHOLD=10       # % cache hit rate decrease threshold

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] $*${NC}"
}

warn() {
    echo -e "${YELLOW}[WARNING] $*${NC}"
}

error() {
    echo -e "${RED}[ERROR] $*${NC}"
}

success() {
    echo -e "${GREEN}[SUCCESS] $*${NC}"
}

# Setup benchmark directories
setup_directories() {
    log "Setting up benchmark directories..."
    mkdir -p "${BENCHMARK_DIR}" "${LATEST_DIR}" "${ARCHIVE_DIR}" "${REPORTS_DIR}"
}

# Run benchmarks with detailed output
run_benchmarks() {
    log "Running comprehensive benchmarks..."
    
    cd "${PROJECT_DIR}"
    
    # Create timestamped result file
    local timestamp=$(date +%Y%m%d_%H%M%S)
    local commit_hash=$(git rev-parse --short HEAD)
    local result_file="${LATEST_DIR}/benchmark_${timestamp}_${commit_hash}.json"
    
    # Run benchmarks with JSON output
    log "Executing cargo bench with criterion..."
    cargo bench --message-format=json -- --output-format json > "${result_file}" 2>&1 || {
        error "Benchmarks failed to run"
        return 1
    }
    
    # Also create human-readable output
    cargo bench | tee "${LATEST_DIR}/benchmark_${timestamp}_${commit_hash}.txt"
    
    # Create latest symlink
    ln -sf "benchmark_${timestamp}_${commit_hash}.json" "${LATEST_DIR}/latest.json"
    ln -sf "benchmark_${timestamp}_${commit_hash}.txt" "${LATEST_DIR}/latest.txt"
    
    echo "${result_file}"
}

# Parse benchmark results and extract metrics
parse_benchmark_results() {
    local result_file="$1"
    local parsed_file="${result_file%.json}_parsed.json"
    
    log "Parsing benchmark results from ${result_file}..."
    
    # Extract key metrics using jq (if available) or python
    if command -v jq >/dev/null 2>&1; then
        parse_with_jq "${result_file}" "${parsed_file}"
    else
        parse_with_python "${result_file}" "${parsed_file}"
    fi
    
    echo "${parsed_file}"
}

parse_with_jq() {
    local input="$1"
    local output="$2"
    
    # Extract benchmark metrics using jq
    jq -r '
    {
        "timestamp": now | strftime("%Y-%m-%d %H:%M:%S"),
        "commit": (env.GITHUB_SHA // "unknown"),
        "branch": (env.GITHUB_REF_NAME // "unknown"),
        "benchmarks": [
            .[] | select(.reason == "benchmark-complete") |
            {
                "name": .id,
                "group": (.id | split("/")[0]),
                "mean_ns": .typical.estimate,
                "std_dev_ns": .typical.standard_error,
                "mean_ms": (.typical.estimate / 1000000),
                "throughput": .throughput
            }
        ]
    }' < "${input}" > "${output}"
}

parse_with_python() {
    local input="$1"
    local output="$2"
    
    python3 << EOF
import json
import sys
from datetime import datetime
import os

try:
    with open('${input}', 'r') as f:
        lines = f.readlines()
    
    benchmarks = []
    for line in lines:
        try:
            data = json.loads(line.strip())
            if data.get('reason') == 'benchmark-complete':
                bench_data = {
                    'name': data.get('id', ''),
                    'group': data.get('id', '').split('/')[0] if '/' in data.get('id', '') else 'unknown',
                    'mean_ns': data.get('typical', {}).get('estimate', 0),
                    'std_dev_ns': data.get('typical', {}).get('standard_error', 0),
                    'mean_ms': data.get('typical', {}).get('estimate', 0) / 1000000,
                    'throughput': data.get('throughput')
                }
                benchmarks.append(bench_data)
        except json.JSONDecodeError:
            continue
    
    result = {
        'timestamp': datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
        'commit': os.environ.get('GITHUB_SHA', 'unknown'),
        'branch': os.environ.get('GITHUB_REF_NAME', 'unknown'),
        'benchmarks': benchmarks
    }
    
    with open('${output}', 'w') as f:
        json.dump(result, f, indent=2)
        
except Exception as e:
    print(f"Error parsing benchmark results: {e}", file=sys.stderr)
    sys.exit(1)
EOF
}

# Compare current results with baseline
detect_regressions() {
    local current_file="$1"
    local baseline_file="${BENCHMARK_DIR}/baseline.json"
    
    if [[ ! -f "${baseline_file}" ]]; then
        warn "No baseline found, creating baseline from current results"
        cp "${current_file}" "${baseline_file}"
        return 0
    fi
    
    log "Detecting performance regressions..."
    
    local regression_report="${REPORTS_DIR}/regression_$(date +%Y%m%d_%H%M%S).json"
    
    python3 << EOF > "${regression_report}"
import json
import sys
from datetime import datetime

def load_benchmarks(file_path):
    with open(file_path, 'r') as f:
        data = json.load(f)
    return {b['name']: b for b in data.get('benchmarks', [])}

try:
    current = load_benchmarks('${current_file}')
    baseline = load_benchmarks('${baseline_file}')
    
    regressions = []
    improvements = []
    
    for name, current_bench in current.items():
        if name not in baseline:
            continue
            
        baseline_bench = baseline[name]
        current_time = current_bench['mean_ms']
        baseline_time = baseline_bench['mean_ms']
        
        if baseline_time > 0:
            change_percent = ((current_time - baseline_time) / baseline_time) * 100
            
            if change_percent > ${PERFORMANCE_THRESHOLD}:
                regressions.append({
                    'name': name,
                    'baseline_ms': baseline_time,
                    'current_ms': current_time,
                    'regression_percent': change_percent
                })
            elif change_percent < -5:  # 5% improvement threshold
                improvements.append({
                    'name': name,
                    'baseline_ms': baseline_time,
                    'current_ms': current_time,
                    'improvement_percent': abs(change_percent)
                })
    
    report = {
        'timestamp': datetime.now().isoformat(),
        'current_commit': current.get('commit', 'unknown'),
        'baseline_commit': baseline.get('commit', 'unknown'),
        'regressions': regressions,
        'improvements': improvements,
        'regression_threshold_percent': ${PERFORMANCE_THRESHOLD},
        'has_regressions': len(regressions) > 0
    }
    
    print(json.dumps(report, indent=2))
    
except Exception as e:
    print(f'{{"error": "Failed to detect regressions: {e}"}}', file=sys.stderr)
    sys.exit(1)
EOF
    
    echo "${regression_report}"
}

# Generate performance report
generate_report() {
    local parsed_file="$1"
    local regression_file="$2"
    local report_file="${REPORTS_DIR}/performance_report_$(date +%Y%m%d_%H%M%S).md"
    
    log "Generating performance report..."
    
    python3 << EOF > "${report_file}"
import json
from datetime import datetime

def load_json(file_path):
    try:
        with open(file_path, 'r') as f:
            return json.load(f)
    except:
        return {}

current_data = load_json('${parsed_file}')
regression_data = load_json('${regression_file}')

print("# LSPbridge Performance Report")
print(f"**Generated**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
print(f"**Commit**: {current_data.get('commit', 'unknown')}")
print(f"**Branch**: {current_data.get('branch', 'unknown')}")
print()

# Performance Summary
benchmarks = current_data.get('benchmarks', [])
if benchmarks:
    print("## 📊 Performance Summary")
    print()
    print("| Benchmark Group | Count | Avg Time (ms) | Fastest (ms) | Slowest (ms) |")
    print("|---|---|---|---|---|")
    
    groups = {}
    for bench in benchmarks:
        group = bench['group']
        if group not in groups:
            groups[group] = []
        groups[group].append(bench['mean_ms'])
    
    for group, times in groups.items():
        count = len(times)
        avg_time = sum(times) / count
        min_time = min(times)
        max_time = max(times)
        print(f"| {group} | {count} | {avg_time:.2f} | {min_time:.2f} | {max_time:.2f} |")
    print()

# Regression Analysis
regressions = regression_data.get('regressions', [])
improvements = regression_data.get('improvements', [])

if regressions:
    print("## ⚠️  Performance Regressions Detected")
    print()
    print("| Benchmark | Baseline (ms) | Current (ms) | Regression |")
    print("|---|---|---|---|")
    for reg in regressions:
        print(f"| {reg['name']} | {reg['baseline_ms']:.2f} | {reg['current_ms']:.2f} | +{reg['regression_percent']:.1f}% |")
    print()

if improvements:
    print("## 🚀 Performance Improvements")
    print()
    print("| Benchmark | Baseline (ms) | Current (ms) | Improvement |")
    print("|---|---|---|---|")
    for imp in improvements:
        print(f"| {imp['name']} | {imp['baseline_ms']:.2f} | {imp['current_ms']:.2f} | -{imp['improvement_percent']:.1f}% |")
    print()

# Detailed Benchmark Results
print("## 📋 Detailed Results")
print()
print("| Benchmark | Time (ms) | Std Dev (ms) | Throughput |")
print("|---|---|---|---|")
for bench in benchmarks:
    throughput = bench.get('throughput', {})
    throughput_str = f"{throughput.get('per_iteration', 'N/A')}" if throughput else "N/A"
    std_dev_ms = bench['std_dev_ns'] / 1000000
    print(f"| {bench['name']} | {bench['mean_ms']:.3f} | {std_dev_ms:.3f} | {throughput_str} |")
print()

# Recommendations
if regressions:
    print("## 💡 Recommendations")
    print()
    print("- **Critical**: Address performance regressions before merging")
    print("- Review recent code changes for performance impact")
    print("- Run profiling on regressed benchmarks")
    print("- Consider rolling back problematic changes")
    print()

print("## 🔗 Links")
print("- [View Benchmark History](../archive/)")
print("- [Latest Results](../latest/)")
print("- [Regression Analysis](./)")
EOF

    echo "${report_file}"
}

# Archive results
archive_results() {
    local timestamp=$(date +%Y%m%d_%H%M%S)
    local commit_hash=$(git rev-parse --short HEAD)
    local archive_subdir="${ARCHIVE_DIR}/${timestamp}_${commit_hash}"
    
    log "Archiving results to ${archive_subdir}..."
    
    mkdir -p "${archive_subdir}"
    cp -r "${LATEST_DIR}"/* "${archive_subdir}/"
    cp -r "${REPORTS_DIR}"/* "${archive_subdir}/" 2>/dev/null || true
    
    # Keep only last 50 archived results
    find "${ARCHIVE_DIR}" -maxdepth 1 -type d -name "*_*" | sort | head -n -50 | xargs -r rm -rf
}

# Update baseline if no regressions
update_baseline() {
    local regression_file="$1"
    local parsed_file="$2"
    
    # Check if there are any regressions
    local has_regressions=$(python3 -c "import json; data=json.load(open('${regression_file}')); print(data.get('has_regressions', False))")
    
    if [[ "${has_regressions}" == "False" ]]; then
        log "No regressions detected, updating baseline..."
        cp "${parsed_file}" "${BENCHMARK_DIR}/baseline.json"
        success "Baseline updated successfully"
    else
        warn "Regressions detected, not updating baseline"
    fi
}

# Main execution
main() {
    log "Starting LSPbridge Benchmark Dashboard"
    
    setup_directories
    
    # Run benchmarks
    result_file=$(run_benchmarks)
    if [[ $? -ne 0 ]]; then
        error "Failed to run benchmarks"
        exit 1
    fi
    
    # Parse results
    parsed_file=$(parse_benchmark_results "${result_file}")
    
    # Detect regressions
    regression_file=$(detect_regressions "${parsed_file}")
    
    # Generate report
    report_file=$(generate_report "${parsed_file}" "${regression_file}")
    
    # Archive results
    archive_results
    
    # Update baseline if appropriate
    update_baseline "${regression_file}" "${parsed_file}"
    
    # Output summary
    echo ""
    success "Benchmark dashboard completed successfully!"
    echo ""
    echo "📄 Files created:"
    echo "   Latest results: ${LATEST_DIR}/latest.txt"
    echo "   Regression analysis: ${regression_file}"
    echo "   Performance report: ${report_file}"
    echo ""
    
    # Check if we should fail CI due to regressions
    local has_regressions=$(python3 -c "import json; data=json.load(open('${regression_file}')); print(data.get('has_regressions', False))")
    if [[ "${has_regressions}" == "True" && "${CI:-false}" == "true" ]]; then
        error "Performance regressions detected in CI - failing build"
        exit 1
    fi
}

# Handle command line arguments
case "${1:-run}" in
    "run")
        main
        ;;
    "setup")
        setup_directories
        ;;
    "archive")
        archive_results
        ;;
    "help"|"--help"|"-h")
        echo "Usage: $0 [run|setup|archive|help]"
        echo ""
        echo "Commands:"
        echo "  run     - Run complete benchmark dashboard (default)"
        echo "  setup   - Setup benchmark directories only"
        echo "  archive - Archive current results only"
        echo "  help    - Show this help message"
        ;;
    *)
        error "Unknown command: $1"
        echo "Use '$0 help' for usage information"
        exit 1
        ;;
esac