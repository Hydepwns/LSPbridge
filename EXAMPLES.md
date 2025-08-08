# LSPbridge CLI Examples

This guide provides practical examples for using LSPbridge's command-line interface. Each example includes the command, its purpose, and expected output format.

## Table of Contents
- [Quick Start](#quick-start)
- [Export Commands](#export-commands)
- [Watch Mode](#watch-mode)
- [Query Commands](#query-commands)
- [History Management](#history-management)
- [AI Training Data](#ai-training-data)
- [Quick Fixes](#quick-fixes)
- [Configuration](#configuration)
- [Multi-Repository](#multi-repository)
- [Advanced Workflows](#advanced-workflows)

## Quick Start

### Basic Diagnostic Export
```bash
# Export all current diagnostics to JSON
lspbridge export --format json --output diagnostics.json

# Export only errors to stdout (great for CI/CD)
lspbridge export --errors-only

# Export as Markdown for documentation
lspbridge export --format markdown --output issues.md
```

### Quick Health Check
```bash
# See current diagnostic counts by severity
lspbridge export --format json | jq '.summary'

# Count total errors in your project
lspbridge export --errors-only --format json | jq '.diagnostics | length'
```

## Export Commands

### Format Options

#### JSON Export (Default)
```bash
# Standard JSON export
lspbridge export --format json

# Pretty-printed JSON with indentation
lspbridge export --format json | jq '.'

# Save to file
lspbridge export --format json --output diagnostics.json
```

#### Markdown Export (Human-Readable)
```bash
# Export as Markdown for reports
lspbridge export --format markdown

# Create a diagnostic report with only errors
lspbridge export --format markdown --errors-only > error-report.md

# Include code context (3 lines before/after)
lspbridge export --format markdown --include-context
```

#### Claude-Optimized Export (AI Assistant)
```bash
# Export for Claude AI assistant
lspbridge export --format claude

# Errors only with context for AI analysis
lspbridge export --format claude --errors-only --include-context

# Pipe directly to Claude via API
lspbridge export --format claude | curl -X POST https://api.anthropic.com/...
```

### Filtering Options

#### By Severity
```bash
# Only errors
lspbridge export --errors-only

# Errors and warnings
lspbridge export --warnings-and-errors

# All diagnostics (default)
lspbridge export
```

#### By File Patterns
```bash
# Include only Rust files
lspbridge export --files "*.rs"

# Include multiple patterns
lspbridge export --files "*.rs,*.toml"

# Exclude test files
lspbridge export --exclude "*test*,*spec*"

# Complex filtering
lspbridge export --files "src/**/*.rs" --exclude "src/tests/**"
```

#### By Count
```bash
# Limit to first 10 diagnostics
lspbridge export --max-results 10

# Get top 5 errors with context
lspbridge export --errors-only --max-results 5 --include-context
```

### Context Options
```bash
# Include 3 lines of context (default)
lspbridge export --include-context

# Include 5 lines before and after
lspbridge export --include-context --context-lines 5

# Full context for AI analysis
lspbridge export --format claude --include-context --context-lines 10
```

## Watch Mode

### Real-Time Monitoring
```bash
# Watch for diagnostic changes (default 1 second interval)
lspbridge watch

# Custom interval (500ms)
lspbridge watch --interval 500

# Watch only errors
lspbridge watch --errors-only

# Watch specific files
lspbridge watch --files "src/**/*.rs"
```

### Integration with Other Tools
```bash
# Pipe to notification system
lspbridge watch --format json | while read line; do
    if echo "$line" | jq -e '.severity == "error"' > /dev/null; then
        notify-send "Build Error" "$line"
    fi
done

# Log to file with timestamps
lspbridge watch --format json | while read line; do
    echo "$(date '+%Y-%m-%d %H:%M:%S') $line" >> diagnostics.log
done
```

## Query Commands

### Interactive Mode
```bash
# Start interactive query session
lspbridge query --interactive

# In interactive mode, you can run queries like:
# > SELECT * FROM diagnostics WHERE severity = 'error'
# > SELECT file, COUNT(*) FROM diagnostics GROUP BY file
# > SELECT * FROM diagnostics WHERE message LIKE '%undefined%'
```

### SQL-Like Queries
```bash
# Find all errors
lspbridge query -q "SELECT * FROM diagnostics WHERE severity = 'error'"

# Count diagnostics by file
lspbridge query -q "SELECT file, COUNT(*) as count FROM diagnostics GROUP BY file ORDER BY count DESC"

# Find specific error patterns
lspbridge query -q "SELECT * FROM diagnostics WHERE message LIKE '%type mismatch%'"

# Complex aggregation
lspbridge query -q "SELECT severity, COUNT(*) as count FROM diagnostics GROUP BY severity"
```

### Output Formats
```bash
# Table format (default)
lspbridge query -q "SELECT * FROM diagnostics LIMIT 5"

# JSON format for processing
lspbridge query -q "SELECT * FROM diagnostics" --format json

# CSV for spreadsheets
lspbridge query -q "SELECT file, severity, message FROM diagnostics" --format csv > report.csv
```

### Time-Based Queries
```bash
# Recent diagnostics (last hour)
lspbridge query -q "SELECT * FROM diagnostics WHERE timestamp > datetime('now', '-1 hour')"

# Today's errors
lspbridge query -q "SELECT * FROM diagnostics WHERE severity = 'error' AND date(timestamp) = date('now')"
```

## History Management

### Analyze Trends
```bash
# Show diagnostic history
lspbridge history analyze

# Analyze specific time range
lspbridge history analyze --days 7

# Export history for analysis
lspbridge history export --format json > history.json
```

### Cleanup
```bash
# Remove old entries (older than 30 days)
lspbridge history clean --older-than 30

# Clear all history
lspbridge history clear --confirm
```

## AI Training Data

### Generate Training Data
```bash
# Export training data for ML models
lspbridge ai-training export training_data.jsonl

# Include context for better training
lspbridge ai-training export --include-context training_data.jsonl

# Generate synthetic examples
lspbridge ai-training generate --count 100 --output synthetic.jsonl
```

### Validate Training Data
```bash
# Validate training data format
lspbridge ai-training validate training_data.jsonl

# Check data statistics
lspbridge ai-training stats training_data.jsonl
```

## Quick Fixes

### Apply Fixes
```bash
# List available quick fixes
lspbridge quick-fix list

# Apply a specific fix
lspbridge quick-fix apply --id fix_123

# Apply all safe fixes
lspbridge quick-fix apply --all --safe-only

# Preview fixes without applying
lspbridge quick-fix preview --id fix_123
```

### Verify Fixes
```bash
# Verify fix was successful
lspbridge quick-fix verify --id fix_123

# Run tests after fixes
lspbridge quick-fix apply --id fix_123 && cargo test
```

## Configuration

### Initialize Configuration
```bash
# Create default config
lspbridge config init

# Create with specific profile
lspbridge config init --profile development
```

### Manage Settings
```bash
# Show current configuration
lspbridge config show

# Get specific setting
lspbridge config get privacy.level

# Set configuration value
lspbridge config set privacy.level strict

# Validate configuration
lspbridge config validate
```

### Profiles
```bash
# Use development profile
lspbridge --profile development export

# Use production profile with strict privacy
lspbridge --profile production export --format claude
```

## Multi-Repository

### Repository Management
```bash
# Register a repository
lspbridge multi-repo register --path /path/to/repo --name frontend

# List registered repositories
lspbridge multi-repo list

# Analyze all repositories
lspbridge multi-repo analyze --all
```

### Cross-Repository Analysis
```bash
# Find type mismatches across repos
lspbridge multi-repo analyze --type-check

# Export aggregated diagnostics
lspbridge multi-repo export --format json --output multi-repo-report.json

# Compare repository health
lspbridge multi-repo compare frontend backend
```

## Advanced Workflows

### CI/CD Integration
```bash
#!/bin/bash
# ci-check.sh - Add to your CI pipeline

# Exit on any error
set -e

# Check for errors
ERROR_COUNT=$(lspbridge export --errors-only --format json | jq '.diagnostics | length')
if [ "$ERROR_COUNT" -gt 0 ]; then
    echo "❌ Found $ERROR_COUNT errors"
    lspbridge export --errors-only --format markdown
    exit 1
fi

# Check for high warning count
WARNING_COUNT=$(lspbridge query -q "SELECT COUNT(*) FROM diagnostics WHERE severity = 'warning'" --format json | jq '.[0].count')
if [ "$WARNING_COUNT" -gt 10 ]; then
    echo "⚠️ High warning count: $WARNING_COUNT"
    exit 1
fi

echo "✅ Code quality check passed"
```

### Git Pre-Commit Hook
```bash
#!/bin/bash
# .git/hooks/pre-commit

# Check for errors before commit
if lspbridge export --errors-only --format json | jq -e '.diagnostics | length > 0' > /dev/null; then
    echo "❌ Commit blocked: Fix errors before committing"
    lspbridge export --errors-only --format markdown
    exit 1
fi

echo "✅ No errors found, proceeding with commit"
```

### AI-Assisted Development
```bash
# Get AI help for current errors
lspbridge export --format claude --errors-only --include-context | \
    pbcopy && echo "✅ Errors copied to clipboard for AI assistant"

# Generate fix suggestions
lspbridge export --format claude --errors-only | \
    curl -X POST https://api.anthropic.com/v1/complete \
    -H "Content-Type: application/json" \
    -d @- | jq '.completion'
```

### Performance Monitoring
```bash
# Track diagnostic counts over time
while true; do
    TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
    COUNT=$(lspbridge export --format json | jq '.summary.total')
    echo "$TIMESTAMP,$COUNT" >> diagnostic-trends.csv
    sleep 60
done

# Generate daily report
lspbridge query -q "
    SELECT 
        date(timestamp) as day,
        severity,
        COUNT(*) as count
    FROM diagnostics
    WHERE timestamp > datetime('now', '-7 days')
    GROUP BY day, severity
    ORDER BY day DESC
" --format csv > weekly-report.csv
```

### Team Collaboration
```bash
# Export team-readable report
cat << 'REPORT' > daily-report.md
# Daily Diagnostic Report - $(date '+%Y-%m-%d')

## Summary
$(lspbridge export --format json | jq -r '.summary | "- Total: \(.total)\n- Errors: \(.errors)\n- Warnings: \(.warnings)"')

## Top Issues by File
$(lspbridge query -q "SELECT file, COUNT(*) as count FROM diagnostics GROUP BY file ORDER BY count DESC LIMIT 5" --format table)

## Critical Errors
$(lspbridge export --errors-only --format markdown --max-results 10)
REPORT

# Share via Slack
cat daily-report.md | slack-cli send --channel engineering
```

## Tips and Best Practices

1. **Use Aliases**: Create shell aliases for common commands
   ```bash
   alias lspe='lspbridge export --errors-only'
   alias lspw='lspbridge watch --interval 500'
   alias lspq='lspbridge query --interactive'
   ```

2. **Combine with jq**: Process JSON output efficiently
   ```bash
   # Get unique error messages
   lspbridge export --format json | jq -r '.diagnostics[].message' | sort -u
   ```

3. **Use --verbose for debugging**: Add `-v` to see what's happening
   ```bash
   lspbridge -v export --format json
   ```

4. **Profile-Based Workflows**: Use different profiles for different scenarios
   ```bash
   # Development: Show everything
   lspbridge --profile development export
   
   # Production: Strict privacy, errors only
   lspbridge --profile production export --errors-only
   ```

5. **Regular Cleanup**: Keep history manageable
   ```bash
   # Add to crontab
   0 2 * * * lspbridge history clean --older-than 30
   ```

## Troubleshooting

```bash
# Check if LSP servers are running
lspbridge config validate

# Test with minimal configuration
lspbridge export --format json --max-results 1

# Enable debug logging
RUST_LOG=debug lspbridge export

# Check configuration location
lspbridge config show --path
```

---

For more information, see the [full documentation](https://docs.rs/lspbridge) or run `lspbridge help <command>`.