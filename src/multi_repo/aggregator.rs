//! Diagnostic aggregation across multiple repositories

use anyhow::Result;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

use super::registry::RepositoryInfo;
use crate::core::types::{Diagnostic, DiagnosticSeverity};

/// Aggregated diagnostic across repositories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedDiagnostic {
    /// Original diagnostic
    pub diagnostic: Diagnostic,

    /// Repository ID where the diagnostic was found
    pub repository_id: String,

    /// Repository name for display
    pub repository_name: String,

    /// Relative path within the repository
    pub relative_path: PathBuf,

    /// Cross-repository impact score (0.0-1.0)
    pub cross_repo_impact: f32,

    /// Related diagnostics in other repositories
    pub related_diagnostics: Vec<RelatedDiagnostic>,
}

/// Related diagnostic in another repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedDiagnostic {
    pub repository_id: String,
    pub repository_name: String,
    pub file_path: PathBuf,
    pub diagnostic_summary: String,
    pub relation_type: DiagnosticRelation,
}

/// Types of relationships between diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticRelation {
    /// Same error pattern
    SamePattern,

    /// Caused by shared dependency
    SharedDependency,

    /// Type mismatch across repos
    TypeMismatch,

    /// API contract violation
    ApiViolation,

    /// Similar code structure
    SimilarCode,
}

/// Aggregates diagnostics from multiple repositories
pub struct DiagnosticAggregator {
    /// Maximum concurrent repository analysis
    semaphore: Arc<Semaphore>,

    /// Cache of repository diagnostics
    cache: Arc<Mutex<HashMap<String, Vec<Diagnostic>>>>,
}

impl DiagnosticAggregator {
    /// Create a new diagnostic aggregator
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Analyze diagnostics across multiple repositories
    pub async fn analyze_repositories(
        &self,
        repositories: Vec<RepositoryInfo>,
    ) -> Result<Vec<AggregatedDiagnostic>> {
        // Collect diagnostics from all repositories in parallel
        let mut tasks = Vec::with_capacity(repositories.len());

        for repo in &repositories {
            let repo = repo.clone();
            let semaphore = self.semaphore.clone();
            let cache = self.cache.clone();

            tasks.push(tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let diagnostics = Self::collect_diagnostics(&repo).await?;

                // Cache the results
                let mut cache_guard = cache.lock().await;
                cache_guard.insert(repo.id.clone(), diagnostics.clone());

                Ok::<(RepositoryInfo, Vec<Diagnostic>), anyhow::Error>((repo, diagnostics))
            }));
        }

        // Wait for all collections to complete
        let results = join_all(tasks).await;

        // Pre-allocate based on expected diagnostics (estimate ~10 per repo)
        let mut all_diagnostics = Vec::with_capacity(repositories.len() * 10);
        let mut repo_diagnostics_map = HashMap::with_capacity(repositories.len());

        for result in results {
            match result {
                Ok(Ok((repo, diagnostics))) => {
                    repo_diagnostics_map
                        .insert(repo.id.clone(), (repo.clone(), diagnostics.clone()));

                    // Convert to aggregated diagnostics
                    for diagnostic in diagnostics {
                        let relative_path = PathBuf::from(&diagnostic.file);

                        all_diagnostics.push(AggregatedDiagnostic {
                            diagnostic: diagnostic.clone(),
                            repository_id: repo.id.clone(),
                            repository_name: repo.name.clone(),
                            relative_path,
                            cross_repo_impact: 0.0, // Will be calculated
                            related_diagnostics: Vec::new(), // Will be populated
                        });
                    }
                }
                Ok(Err(e)) => eprintln!("Failed to collect diagnostics: {e}"),
                Err(e) => eprintln!("Task failed: {e}"),
            }
        }

        // Find relationships between diagnostics
        self.find_relationships(&mut all_diagnostics, &repo_diagnostics_map)
            .await?;

        // Calculate cross-repository impact scores
        self.calculate_impact_scores(&mut all_diagnostics);

        // Sort by impact score (highest first)
        all_diagnostics.sort_by(|a, b| {
            b.cross_repo_impact
                .partial_cmp(&a.cross_repo_impact)
                .unwrap()
        });

        Ok(all_diagnostics)
    }

    /// Collect diagnostics from a single repository
    async fn collect_diagnostics(_repo: &RepositoryInfo) -> Result<Vec<Diagnostic>> {
        // For now, return empty diagnostics
        // In a real implementation, this would integrate with LSP clients
        Ok(Vec::new())
    }

    /// Find relationships between diagnostics across repositories
    async fn find_relationships(
        &self,
        diagnostics: &mut [AggregatedDiagnostic],
        repo_map: &HashMap<String, (RepositoryInfo, Vec<Diagnostic>)>,
    ) -> Result<()> {
        for i in 0..diagnostics.len() {
            let current = &diagnostics[i];
            let mut related = Vec::with_capacity(3); // Most diagnostics have 0-3 related items

            // Check for similar patterns in other repositories
            for (repo_id, (repo_info, repo_diagnostics)) in repo_map {
                if repo_id == &current.repository_id {
                    continue;
                }

                for other_diagnostic in repo_diagnostics {
                    if let Some(relation) = Self::check_relation(
                        &current.diagnostic,
                        other_diagnostic,
                        &current.repository_id,
                        repo_id,
                    ) {
                        related.push(RelatedDiagnostic {
                            repository_id: repo_id.clone(),
                            repository_name: repo_info.name.clone(),
                            file_path: PathBuf::from(&other_diagnostic.file),
                            diagnostic_summary: other_diagnostic.message.clone(),
                            relation_type: relation,
                        });
                    }
                }
            }

            diagnostics[i].related_diagnostics = related;
        }

        Ok(())
    }

    /// Check if two diagnostics are related
    fn check_relation(
        diag1: &Diagnostic,
        diag2: &Diagnostic,
        _repo1: &str,
        _repo2: &str,
    ) -> Option<DiagnosticRelation> {
        // Check for same error patterns
        if Self::is_same_pattern(diag1, diag2) {
            return Some(DiagnosticRelation::SamePattern);
        }

        // Check for type mismatches across repos
        if Self::is_type_mismatch(diag1, diag2) {
            return Some(DiagnosticRelation::TypeMismatch);
        }

        // Check for API violations
        if Self::is_api_violation(diag1, diag2) {
            return Some(DiagnosticRelation::ApiViolation);
        }

        // Check for similar code structure
        if Self::is_similar_code(diag1, diag2) {
            return Some(DiagnosticRelation::SimilarCode);
        }

        None
    }

    /// Check if diagnostics have the same pattern
    fn is_same_pattern(diag1: &Diagnostic, diag2: &Diagnostic) -> bool {
        // Compare error codes
        if diag1.code == diag2.code && diag1.code.is_some() {
            return true;
        }

        // Compare message patterns (simplified)
        let msg1_words: HashSet<_> = diag1.message.split_whitespace().collect();
        let msg2_words: HashSet<_> = diag2.message.split_whitespace().collect();

        let intersection = msg1_words.intersection(&msg2_words).count();
        let similarity = intersection as f32 / msg1_words.len().max(msg2_words.len()) as f32;

        similarity > 0.7
    }

    /// Check if diagnostics represent a type mismatch
    fn is_type_mismatch(diag1: &Diagnostic, diag2: &Diagnostic) -> bool {
        let type_keywords = ["type", "Type", "interface", "Interface", "struct", "Struct"];

        let has_type_error = |msg: &str| {
            type_keywords.iter().any(|kw| msg.contains(kw))
                && (msg.contains("mismatch")
                    || msg.contains("incompatible")
                    || msg.contains("expected")
                    || msg.contains("found"))
        };

        has_type_error(&diag1.message) && has_type_error(&diag2.message)
    }

    /// Check if diagnostics represent an API violation
    fn is_api_violation(diag1: &Diagnostic, diag2: &Diagnostic) -> bool {
        let api_keywords = ["api", "API", "endpoint", "route", "contract", "schema"];

        api_keywords
            .iter()
            .any(|kw| diag1.message.contains(kw) || diag2.message.contains(kw))
    }

    /// Check if diagnostics occur in similar code
    fn is_similar_code(diag1: &Diagnostic, diag2: &Diagnostic) -> bool {
        // Check if file names are similar
        let path1 = PathBuf::from(&diag1.file);
        let path2 = PathBuf::from(&diag2.file);

        if let (Some(name1), Some(name2)) = (path1.file_name(), path2.file_name()) {
            return name1 == name2;
        }

        false
    }

    /// Calculate cross-repository impact scores
    fn calculate_impact_scores(&self, diagnostics: &mut [AggregatedDiagnostic]) {
        for diagnostic in diagnostics {
            let mut score = 0.0;

            // Base score from severity
            score += match diagnostic.diagnostic.severity {
                DiagnosticSeverity::Error => 0.5,
                DiagnosticSeverity::Warning => 0.3,
                DiagnosticSeverity::Information => 0.1,
                DiagnosticSeverity::Hint => 0.05,
            };

            // Increase score based on number of related diagnostics
            let related_count = diagnostic.related_diagnostics.len();
            score += (related_count as f32 * 0.1).min(0.3);

            // Increase score for certain relation types
            for related in &diagnostic.related_diagnostics {
                match related.relation_type {
                    DiagnosticRelation::TypeMismatch => score += 0.1,
                    DiagnosticRelation::ApiViolation => score += 0.15,
                    DiagnosticRelation::SharedDependency => score += 0.05,
                    _ => score += 0.02,
                }
            }

            diagnostic.cross_repo_impact = score.min(1.0);
        }
    }

    /// Get cached diagnostics for a repository
    pub async fn get_cached(&self, repo_id: &str) -> Option<Vec<Diagnostic>> {
        let cache = self.cache.lock().await;
        cache.get(repo_id).cloned()
    }

    /// Clear the diagnostic cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }
}
