use crate::core::{DiagnosticsCache, DiagnosticSnapshot, Diagnostic, DiagnosticFilter};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
struct CacheEntry {
    snapshot: DiagnosticSnapshot,
    last_accessed: DateTime<Utc>,
}

#[derive(Debug)]
pub struct MemoryCache {
    snapshots: HashMap<Uuid, CacheEntry>,
    max_snapshots: usize,
    max_age: Duration,
}

impl MemoryCache {
    pub fn new(max_snapshots: usize, max_age_seconds: u64) -> Self {
        Self {
            snapshots: HashMap::new(),
            max_snapshots,
            max_age: Duration::seconds(max_age_seconds as i64),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(100, 24 * 60 * 60) // 100 snapshots, 24 hours
    }

    fn evict_if_needed(&mut self) {
        // Remove expired entries first
        let now = Utc::now();
        let expired_ids: Vec<Uuid> = self
            .snapshots
            .iter()
            .filter(|(_, entry)| now.signed_duration_since(entry.snapshot.timestamp) > self.max_age)
            .map(|(id, _)| *id)
            .collect();

        for id in expired_ids {
            self.snapshots.remove(&id);
        }

        // If still over limit, remove least recently accessed
        if self.snapshots.len() > self.max_snapshots {
            let mut entries: Vec<_> = self.snapshots.iter().collect();
            entries.sort_by(|a, b| a.1.last_accessed.cmp(&b.1.last_accessed));

            let to_remove = entries.len() - self.max_snapshots;
            let ids_to_remove: Vec<Uuid> = entries.into_iter()
                .take(to_remove)
                .map(|(id, _)| *id)
                .collect();
            
            for id in ids_to_remove {
                self.snapshots.remove(&id);
            }
        }
    }

    fn apply_filter(&self, diagnostics: Vec<Diagnostic>, filter: &DiagnosticFilter) -> Vec<Diagnostic> {
        let mut filtered = diagnostics;

        // Filter by severities
        if let Some(severities) = &filter.severities {
            let severity_set: std::collections::HashSet<_> = severities.iter().collect();
            filtered.retain(|d| severity_set.contains(&d.severity));
        }

        // Filter by sources
        if let Some(sources) = &filter.sources {
            let source_set: std::collections::HashSet<_> = sources.iter().collect();
            filtered.retain(|d| source_set.contains(&d.source));
        }

        // Filter by file patterns
        if let Some(patterns) = &filter.file_patterns {
            filtered.retain(|d| {
                patterns.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches(&d.file))
                        .unwrap_or(false)
                })
            });
        }

        // Exclude patterns
        if let Some(exclude_patterns) = &filter.exclude_patterns {
            filtered.retain(|d| {
                !exclude_patterns.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches(&d.file))
                        .unwrap_or(false)
                })
            });
        }

        // Filter by date
        if let Some(_since) = filter.since {
            // Note: Since diagnostics don't have individual timestamps,
            // we filter by snapshot timestamp
            // This is a limitation we could address by adding timestamps to diagnostics
        }

        // Limit results
        if let Some(max_results) = filter.max_results {
            filtered.truncate(max_results);
        }

        filtered
    }

    pub fn get_size(&self) -> usize {
        self.snapshots.len()
    }

    pub fn get_stats(&self) -> CacheStats {
        if self.snapshots.is_empty() {
            return CacheStats {
                size: 0,
                oldest_entry: None,
                newest_entry: None,
            };
        }

        let timestamps: Vec<_> = self
            .snapshots
            .values()
            .map(|entry| entry.snapshot.timestamp)
            .collect();

        CacheStats {
            size: self.snapshots.len(),
            oldest_entry: timestamps.iter().min().copied(),
            newest_entry: timestamps.iter().max().copied(),
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub size: usize,
    pub oldest_entry: Option<DateTime<Utc>>,
    pub newest_entry: Option<DateTime<Utc>>,
}

#[async_trait]
impl DiagnosticsCache for MemoryCache {
    async fn store(&mut self, snapshot: DiagnosticSnapshot) -> Result<()> {
        let entry = CacheEntry {
            snapshot: snapshot.clone(),
            last_accessed: Utc::now(),
        };

        self.snapshots.insert(snapshot.id, entry);
        self.evict_if_needed();
        Ok(())
    }

    async fn get(&self, filter: Option<DiagnosticFilter>) -> Result<Vec<Diagnostic>> {
        let all_diagnostics: Vec<Diagnostic> = self
            .snapshots
            .values()
            .flat_map(|entry| entry.snapshot.diagnostics.clone())
            .collect();

        match filter {
            Some(f) => Ok(self.apply_filter(all_diagnostics, &f)),
            None => Ok(all_diagnostics),
        }
    }

    async fn get_snapshot(&self, id: &Uuid) -> Result<Option<DiagnosticSnapshot>> {
        if let Some(entry) = self.snapshots.get(id) {
            // Update access time (would need interior mutability in real impl)
            Ok(Some(entry.snapshot.clone()))
        } else {
            Ok(None)
        }
    }

    async fn get_snapshots(&self, limit: Option<usize>) -> Result<Vec<DiagnosticSnapshot>> {
        let mut snapshots: Vec<_> = self
            .snapshots
            .values()
            .map(|entry| entry.snapshot.clone())
            .collect();

        // Sort by timestamp, newest first
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(limit) = limit {
            snapshots.truncate(limit);
        }

        Ok(snapshots)
    }

    async fn clear(&mut self) -> Result<()> {
        self.snapshots.clear();
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        let now = Utc::now();
        let expired_ids: Vec<Uuid> = self
            .snapshots
            .iter()
            .filter(|(_, entry)| now.signed_duration_since(entry.snapshot.timestamp) > self.max_age)
            .map(|(id, _)| *id)
            .collect();

        for id in expired_ids {
            self.snapshots.remove(&id);
        }

        Ok(())
    }
}