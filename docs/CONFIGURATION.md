# Configuration Reference

## Configuration File

Configuration is stored in `lspbridge.toml`. Search order:

1. `LSP_BRIDGE_CONFIG` environment variable
2. `./lspbridge.toml`
3. `~/.config/lspbridge/lspbridge.toml`
4. Built-in defaults

## Complete Configuration Reference

```toml
# LSP Bridge Configuration
# All settings are optional - defaults are shown below

[processing]
# Enable parallel processing for better performance
parallel_processing = true

# Number of files to process in each batch
chunk_size = 500

# Maximum number of files to process concurrently
max_concurrent_files = 200

# Maximum file size to process (in MB)
file_size_limit_mb = 10

# Timeout for processing operations (in seconds)
timeout_seconds = 30

[cache]
# Enable persistent cache on disk
enable_persistent_cache = true

# Enable in-memory cache
enable_memory_cache = true

# Maximum size of persistent cache (in MB)
max_size_mb = 500

# Maximum number of entries in cache
max_entries = 10000

# Time-to-live for cache entries (in hours)
ttl_hours = 24

# How often to clean up expired cache entries (in minutes)
cleanup_interval_minutes = 60

# Enable compression for cached data
compression_enabled = true

# Cache tiers for optimized storage
cache_tiers = ["hot", "warm", "cold"]

[memory]
# Maximum memory usage (in MB)
max_memory_mb = 512

# Maximum entries in memory cache
max_entries = 50000

# Cache eviction policy: "LRU", "LFU", "Adaptive"
eviction_policy = "Adaptive"

# Start evicting when memory usage reaches this percentage
high_water_mark = 0.8

# Stop evicting when memory usage drops to this percentage
low_water_mark = 0.6

# Number of entries to evict at once
eviction_batch_size = 100

# How often to check memory usage (in seconds)
monitoring_interval_seconds = 30

# Detect system memory pressure and adapt
memory_pressure_detection = true

[error_recovery]
# Enable circuit breaker for failing operations
enable_circuit_breaker = true

# Maximum retry attempts
max_retries = 3

# Initial delay between retries (in milliseconds)
initial_delay_ms = 100

# Maximum delay between retries (in milliseconds)
max_delay_ms = 5000

# Exponential backoff multiplier
backoff_multiplier = 2.0

# Number of failures before opening circuit
failure_threshold = 5

# Number of successes before closing circuit
success_threshold = 3

# Timeout for operations (in milliseconds)
timeout_ms = 10000

# Add random jitter to retry delays
retry_jitter = true

# Categorize errors for better handling
error_categorization = true

[git]
# Enable Git integration features
enable_git_integration = true

# How often to scan for Git changes (in seconds)
scan_interval_seconds = 90

# Ignore untracked files
ignore_untracked = false

# Track staged changes
track_staged_changes = true

# Automatically refresh on Git operations
auto_refresh = true

# Honor .gitignore patterns
respect_gitignore = true

# Maintain separate caches per branch
branch_aware_caching = true

[metrics]
# Enable metrics collection
enable_metrics = true

# Prometheus metrics endpoint port
prometheus_port = 9090

# How often to collect metrics (in seconds)
collection_interval_seconds = 30

# How long to retain metrics (in hours)
retention_hours = 72

# Export format: "prometheus", "json"
export_format = "prometheus"

# Enable OpenTelemetry tracing
enable_opentelemetry = true

# Custom metrics to track
custom_metrics = ["cache_hit_rate", "processing_time_by_type"]

[features]
# Enable automatic performance optimization
auto_optimization = true

# Enable health monitoring
health_monitoring = true

# Pre-warm cache on startup
cache_warming = true

# Enable advanced diagnostic features
advanced_diagnostics = false

# Enable experimental features
experimental_features = false

[performance]
# How often to run optimization (in minutes)
optimization_interval_minutes = 60

# How often to check system health (in minutes)
health_check_interval_minutes = 5

# Trigger garbage collection above this threshold (in MB)
gc_threshold_mb = 512

# Maximum CPU usage percentage
max_cpu_usage_percent = 80.0

# Enable adaptive scaling based on system resources
adaptive_scaling = true

# Language-specific settings
[languages.rust]
max_file_size_mb = 20
parallel_analysis = true
incremental_parsing = true

[languages.typescript]
max_file_size_mb = 15
parallel_analysis = true
use_incremental_parsing = true

[languages.go]
max_file_size_mb = 20
parallel_analysis = true

[languages.python]
max_file_size_mb = 10
parallel_analysis = true

# Security settings
[security]
# Maximum allowed file path length
max_file_path_length = 4096

# Prevent following symlinks outside workspace
disallow_symlinks_outside_workspace = true

# Scan for secrets and credentials
scan_for_secrets = true

# Allowed file extensions
allowed_file_extensions = ["rs", "ts", "js", "go", "py", "java", "cpp", "c", "h", "hpp"]

# Profile configurations
[profiles.development]
debug_logging = true
aggressive_caching = false
detailed_error_messages = true
performance_profiling = true

[profiles.production]
debug_logging = false
aggressive_caching = true
detailed_error_messages = false
performance_profiling = false

[profiles.testing]
debug_logging = true
aggressive_caching = false
mock_external_services = true
deterministic_ids = true

# Network settings (for future distributed mode)
[network]
connection_pool_size = 10
request_timeout_seconds = 30
enable_http2 = true
max_retries = 3
keepalive_seconds = 60
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LSP_BRIDGE_CONFIG` | Path to configuration file | `./lspbridge.toml` |
| `LSP_BRIDGE_PROFILE` | Active configuration profile | None |
| `LSP_BRIDGE_CACHE_DIR` | Override cache directory | Platform-specific |
| `LSP_BRIDGE_LOG_LEVEL` | Log level (trace/debug/info/warn/error) | `info` |
| `LSP_BRIDGE_OTEL_ENDPOINT` | OpenTelemetry collector endpoint | None |

## Configuration Profiles

Profiles allow you to maintain different configurations for different environments:

```bash
# Use development profile
LSP_BRIDGE_PROFILE=development lsp-bridge export

# Use production profile
LSP_BRIDGE_PROFILE=production lsp-bridge serve
```

## Generating Configuration

To generate a configuration file with defaults:

```bash
# Generate default configuration
lsp-bridge config init

# Generate configuration with comments
lsp-bridge config init --commented

# Generate minimal configuration
lsp-bridge config init --minimal
```

## Validating Configuration

To validate your configuration:

```bash
# Validate configuration file
lsp-bridge config validate

# Validate specific file
lsp-bridge config validate --file /path/to/lspbridge.toml
```

## Common Configuration Patterns

### High-Performance Setup

```toml
[processing]
parallel_processing = true
chunk_size = 1000
max_concurrent_files = 500

[cache]
max_size_mb = 2000
compression_enabled = true

[memory]
max_memory_mb = 2048
eviction_policy = "Adaptive"
```

### Low-Resource Setup

```toml
[processing]
parallel_processing = false
chunk_size = 100
max_concurrent_files = 50

[cache]
enable_persistent_cache = false
max_size_mb = 100

[memory]
max_memory_mb = 128
high_water_mark = 0.7
```

### Security-Focused Setup

```toml
[security]
scan_for_secrets = true
disallow_symlinks_outside_workspace = true
allowed_file_extensions = ["rs", "ts", "js"]

[git]
ignore_untracked = true
respect_gitignore = true
```