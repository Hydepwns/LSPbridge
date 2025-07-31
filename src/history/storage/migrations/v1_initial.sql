CREATE TABLE IF NOT EXISTS diagnostic_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    error_count INTEGER NOT NULL,
    warning_count INTEGER NOT NULL,
    info_count INTEGER NOT NULL,
    hint_count INTEGER NOT NULL,
    diagnostics_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_snapshots_file_path ON diagnostic_snapshots(file_path);
CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON diagnostic_snapshots(timestamp);
CREATE INDEX IF NOT EXISTS idx_snapshots_created_at ON diagnostic_snapshots(created_at);

CREATE TABLE IF NOT EXISTS file_stats (
    file_path TEXT PRIMARY KEY,
    first_seen INTEGER NOT NULL,
    last_seen INTEGER NOT NULL,
    total_snapshots INTEGER NOT NULL,
    total_errors INTEGER NOT NULL,
    total_warnings INTEGER NOT NULL,
    avg_error_count REAL NOT NULL,
    avg_warning_count REAL NOT NULL,
    max_error_count INTEGER NOT NULL,
    max_warning_count INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS error_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern_hash TEXT NOT NULL UNIQUE,
    first_seen INTEGER NOT NULL,
    last_seen INTEGER NOT NULL,
    occurrence_count INTEGER NOT NULL,
    files_affected INTEGER NOT NULL,
    error_message TEXT NOT NULL,
    error_code TEXT,
    source TEXT
);

CREATE INDEX IF NOT EXISTS idx_patterns_hash ON error_patterns(pattern_hash);
CREATE INDEX IF NOT EXISTS idx_patterns_count ON error_patterns(occurrence_count);

CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);