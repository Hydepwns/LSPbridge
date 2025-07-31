//! Multi-repository analysis and reporting utilities
//!
//! This module provides analysis capabilities for cross-repository relationships,
//! dependency tracking, impact analysis, and comprehensive reporting.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::multi_repo::{AggregatedDiagnostic, MultiRepoContext, RepositoryInfo};

/// Multi-repository analyzer for cross-repo impact analysis
pub struct MultiRepoAnalyzer {
    /// Minimum impact score threshold
    min_impact_threshold: f32,
    
    /// Whether to include inactive repositories
    include_inactive: bool,
    
    /// Language-specific analysis weights
    language_weights: HashMap<String, f32>,
}

impl MultiRepoAnalyzer {
    /// Create a new multi-repository analyzer
    pub fn new() -> Self {
        let mut language_weights = HashMap::new();
        
        // Higher weights for more commonly shared languages
        language_weights.insert("typescript".to_string(), 1.0);
        language_weights.insert("javascript".to_string(), 1.0);
        language_weights.insert("rust".to_string(), 0.9);
        language_weights.insert("python".to_string(), 0.8);
        language_weights.insert("go".to_string(), 0.8);
        language_weights.insert("java".to_string(), 0.7);
        language_weights.insert("cpp".to_string(), 0.6);
        language_weights.insert("c".to_string(), 0.6);

        Self {
            min_impact_threshold: 0.1,
            include_inactive: false,
            language_weights,
        }
    }

    /// Set minimum impact threshold
    pub fn with_min_impact(mut self, threshold: f32) -> Self {
        self.min_impact_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set whether to include inactive repositories
    pub fn with_inactive(mut self, include: bool) -> Self {
        self.include_inactive = include;
        self
    }

    /// Add custom language weight
    pub fn with_language_weight(mut self, language: String, weight: f32) -> Self {
        self.language_weights.insert(language, weight.clamp(0.0, 1.0));
        self
    }

    /// Analyze cross-repository impact
    pub async fn analyze_cross_repo_impact(
        &self,
        context: &MultiRepoContext,
    ) -> Result<CrossRepoAnalysisResult> {
        let repositories = context.list_repositories(self.include_inactive).await?;
        
        // Analyze repository relationships
        let relationships = self.analyze_repository_relationships(&repositories).await?;
        
        // Analyze shared dependencies
        let shared_deps = self.analyze_shared_dependencies(&repositories).await?;
        
        // Analyze type sharing
        let type_sharing = self.analyze_type_sharing(&repositories).await?;
        
        // Calculate impact scores
        let impact_scores = self.calculate_impact_scores(&repositories, &relationships, &shared_deps, &type_sharing).await?;
        
        // Aggregate diagnostics with cross-repo impact
        let aggregated_diagnostics = self.aggregate_diagnostics_with_impact(&repositories, &impact_scores).await?;

        Ok(CrossRepoAnalysisResult {
            repositories: repositories.clone(),
            relationships,
            shared_dependencies: shared_deps,
            type_sharing,
            impact_scores,
            aggregated_diagnostics,
            analysis_metadata: AnalysisMetadata {
                min_impact_threshold: self.min_impact_threshold,
                total_repositories: repositories.len(),
                active_repositories: repositories.iter().filter(|r| r.active).count(),
                languages_analyzed: self.get_analyzed_languages(&repositories),
                analysis_timestamp: chrono::Utc::now(),
            },
        })
    }

    /// Analyze relationships between repositories
    async fn analyze_repository_relationships(
        &self,
        repositories: &[RepositoryInfo],
    ) -> Result<Vec<RepositoryRelationship>> {
        let mut relationships = Vec::new();

        // Analyze monorepo relationships
        let mut monorepo_groups: HashMap<String, Vec<&RepositoryInfo>> = HashMap::new();
        for repo in repositories {
            if let Some(monorepo_id) = &repo.monorepo_id {
                monorepo_groups.entry(monorepo_id.clone()).or_default().push(repo);
            }
        }

        // Create sibling relationships within monorepos
        for (monorepo_id, siblings) in monorepo_groups {
            for (i, repo1) in siblings.iter().enumerate() {
                for repo2 in siblings.iter().skip(i + 1) {
                    relationships.push(RepositoryRelationship {
                        source_repo_id: repo1.id.clone(),
                        target_repo_id: repo2.id.clone(),
                        relationship_type: RelationshipType::MonorepoSibling,
                        strength: 0.8, // High strength for monorepo siblings
                        metadata: serde_json::json!({
                            "monorepo_id": monorepo_id,
                            "detected_via": "monorepo_analysis"
                        }),
                    });
                }
            }
        }

        // Analyze language-based relationships
        let mut language_groups: HashMap<String, Vec<&RepositoryInfo>> = HashMap::new();
        for repo in repositories {
            if let Some(language) = &repo.primary_language {
                language_groups.entry(language.clone()).or_default().push(repo);
            }
        }

        // Create weak relationships between repositories with the same language
        for (language, repos) in language_groups {
            if repos.len() > 1 {
                let weight = self.language_weights.get(&language).copied().unwrap_or(0.3);
                
                for (i, repo1) in repos.iter().enumerate() {
                    for repo2 in repos.iter().skip(i + 1) {
                        // Skip if already related through monorepo
                        if repo1.monorepo_id.is_some() && repo1.monorepo_id == repo2.monorepo_id {
                            continue;
                        }

                        relationships.push(RepositoryRelationship {
                            source_repo_id: repo1.id.clone(),
                            target_repo_id: repo2.id.clone(),
                            relationship_type: RelationshipType::LanguageSimilarity,
                            strength: weight * 0.3, // Lower strength for language similarity
                            metadata: serde_json::json!({
                                "shared_language": language,
                                "detected_via": "language_analysis"
                            }),
                        });
                    }
                }
            }
        }

        Ok(relationships)
    }

    /// Analyze shared dependencies between repositories
    async fn analyze_shared_dependencies(
        &self,
        repositories: &[RepositoryInfo],
    ) -> Result<Vec<SharedDependency>> {
        let mut shared_deps = Vec::new();
        let mut dependency_map: HashMap<String, Vec<String>> = HashMap::new();

        // Extract dependencies from each repository
        for repo in repositories {
            let deps = self.extract_dependencies(&repo.path).await?;
            for dep in deps {
                dependency_map.entry(dep).or_default().push(repo.id.clone());
            }
        }

        // Find dependencies shared across multiple repositories
        for (dependency, repos) in dependency_map {
            if repos.len() > 1 {
                let impact_score = self.calculate_dependency_impact_score(&dependency, &repos, repositories);
                
                if impact_score >= self.min_impact_threshold {
                    shared_deps.push(SharedDependency {
                        dependency_name: dependency,
                        affected_repositories: repos,
                        dependency_type: self.classify_dependency_type(&dependency),
                        impact_score,
                        metadata: serde_json::json!({
                            "detected_via": "dependency_analysis"
                        }),
                    });
                }
            }
        }

        // Sort by impact score (highest first)
        shared_deps.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(shared_deps)
    }

    /// Analyze type sharing between repositories
    async fn analyze_type_sharing(
        &self,
        repositories: &[RepositoryInfo],
    ) -> Result<Vec<SharedType>> {
        let mut shared_types = Vec::new();
        let mut type_definitions: HashMap<String, Vec<TypeDefinition>> = HashMap::new();

        // Extract type definitions from each repository
        for repo in repositories {
            let types = self.extract_type_definitions(&repo.path, &repo.id).await?;
            for type_def in types {
                type_definitions.entry(type_def.type_name.clone()).or_default().push(type_def);
            }
        }

        // Find types that appear in multiple repositories
        for (type_name, definitions) in type_definitions {
            if definitions.len() > 1 {
                // Check if definitions are similar (potential duplicates) or references
                let (references, duplicates) = self.classify_type_usage(&definitions).await?;
                
                if !references.is_empty() || duplicates.len() > 1 {
                    let impact_score = self.calculate_type_impact_score(&type_name, &definitions, repositories);
                    
                    if impact_score >= self.min_impact_threshold {
                        shared_types.push(SharedType {
                            type_name,
                            definitions,
                            references,
                            duplicates,
                            impact_score,
                            metadata: serde_json::json!({
                                "detected_via": "type_analysis"
                            }),
                        });
                    }
                }
            }
        }

        // Sort by impact score
        shared_types.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(shared_types)
    }

    /// Calculate impact scores for repositories
    async fn calculate_impact_scores(
        &self,
        repositories: &[RepositoryInfo],
        relationships: &[RepositoryRelationship],
        shared_deps: &[SharedDependency],
        type_sharing: &[SharedType],
    ) -> Result<HashMap<String, RepositoryImpactScore>> {
        let mut impact_scores = HashMap::new();

        for repo in repositories {
            let relationship_score = self.calculate_relationship_impact(&repo.id, relationships);
            let dependency_score = self.calculate_dependency_impact(&repo.id, shared_deps);
            let type_score = self.calculate_type_impact(&repo.id, type_sharing);
            
            // Weighted combination of different impact factors
            let overall_score = (relationship_score * 0.3) + (dependency_score * 0.4) + (type_score * 0.3);
            
            impact_scores.insert(repo.id.clone(), RepositoryImpactScore {
                repository_id: repo.id.clone(),
                overall_impact: overall_score,
                relationship_impact: relationship_score,
                dependency_impact: dependency_score,
                type_impact: type_score,
                metadata: serde_json::json!({
                    "calculation_method": "weighted_combination",
                    "weights": {
                        "relationships": 0.3,
                        "dependencies": 0.4,
                        "types": 0.3
                    }
                }),
            });
        }

        Ok(impact_scores)
    }

    /// Aggregate diagnostics with cross-repository impact
    async fn aggregate_diagnostics_with_impact(
        &self,
        repositories: &[RepositoryInfo],
        impact_scores: &HashMap<String, RepositoryImpactScore>,
    ) -> Result<Vec<AggregatedDiagnostic>> {
        let mut aggregated = Vec::new();

        // This is a placeholder implementation
        // In a real scenario, this would analyze actual diagnostics from each repository
        // and calculate their cross-repository impact based on the analysis results
        
        for repo in repositories {
            if let Some(impact_score) = impact_scores.get(&repo.id) {
                if impact_score.overall_impact >= self.min_impact_threshold {
                    // Placeholder diagnostic for demonstration
                    aggregated.push(AggregatedDiagnostic {
                        id: format!("placeholder-{}", repo.id),
                        severity: "info".to_string(),
                        message: format!("Repository {} has cross-repo impact", repo.name),
                        file_path: repo.path.clone(),
                        line_number: 1,
                        column_number: 1,
                        source_repository: repo.id.clone(),
                        affected_repositories: vec![repo.id.clone()], // Would include other affected repos
                        cross_repo_impact: impact_score.overall_impact,
                        category: "cross_repo_impact".to_string(),
                        metadata: serde_json::json!({
                            "impact_breakdown": {
                                "relationship_impact": impact_score.relationship_impact,
                                "dependency_impact": impact_score.dependency_impact,
                                "type_impact": impact_score.type_impact
                            }
                        }),
                    });
                }
            }
        }

        Ok(aggregated)
    }

    /// Extract dependencies from a repository path
    async fn extract_dependencies(&self, _path: &PathBuf) -> Result<Vec<String>> {
        // Placeholder implementation
        // In a real scenario, this would parse package.json, Cargo.toml, requirements.txt, etc.
        Ok(vec![
            "react".to_string(),
            "typescript".to_string(),
            "serde".to_string(),
            "tokio".to_string(),
        ])
    }

    /// Extract type definitions from a repository
    async fn extract_type_definitions(&self, _path: &PathBuf, repo_id: &str) -> Result<Vec<TypeDefinition>> {
        // Placeholder implementation
        // In a real scenario, this would parse source files and extract type definitions
        Ok(vec![
            TypeDefinition {
                type_name: "User".to_string(),
                repository_id: repo_id.to_string(),
                file_path: PathBuf::from("src/types.ts"),
                line_number: 10,
                definition_kind: TypeDefinitionKind::Interface,
                signature: "interface User { id: string; name: string; }".to_string(),
            }
        ])
    }

    /// Calculate dependency impact score
    fn calculate_dependency_impact_score(&self, _dependency: &str, repos: &[String], _all_repos: &[RepositoryInfo]) -> f32 {
        // Simple scoring based on number of affected repositories
        let base_score = (repos.len() as f32).sqrt() / 10.0;
        base_score.min(1.0)
    }

    /// Calculate type impact score
    fn calculate_type_impact_score(&self, _type_name: &str, definitions: &[TypeDefinition], _all_repos: &[RepositoryInfo]) -> f32 {
        // Simple scoring based on number of definitions
        let base_score = (definitions.len() as f32).sqrt() / 5.0;
        base_score.min(1.0)
    }

    /// Calculate relationship impact for a repository
    fn calculate_relationship_impact(&self, repo_id: &str, relationships: &[RepositoryRelationship]) -> f32 {
        relationships.iter()
            .filter(|r| r.source_repo_id == repo_id || r.target_repo_id == repo_id)
            .map(|r| r.strength)
            .sum::<f32>()
            .min(1.0)
    }

    /// Calculate dependency impact for a repository
    fn calculate_dependency_impact(&self, repo_id: &str, shared_deps: &[SharedDependency]) -> f32 {
        shared_deps.iter()
            .filter(|d| d.affected_repositories.contains(repo_id))
            .map(|d| d.impact_score)
            .sum::<f32>()
            .min(1.0)
    }

    /// Calculate type impact for a repository
    fn calculate_type_impact(&self, repo_id: &str, shared_types: &[SharedType]) -> f32 {
        shared_types.iter()
            .filter(|t| t.definitions.iter().any(|d| d.repository_id == repo_id))
            .map(|t| t.impact_score)
            .sum::<f32>()
            .min(1.0)
    }

    /// Classify dependency type
    fn classify_dependency_type(&self, dependency: &str) -> DependencyType {
        match dependency {
            d if d.starts_with("@types/") => DependencyType::TypeDefinitions,
            "react" | "vue" | "angular" => DependencyType::Framework,
            "typescript" | "babel" | "webpack" => DependencyType::BuildTool,
            "jest" | "mocha" | "vitest" => DependencyType::Testing,
            _ => DependencyType::Library,
        }
    }

    /// Classify type usage as references or duplicates
    async fn classify_type_usage(&self, definitions: &[TypeDefinition]) -> Result<(Vec<TypeReference>, Vec<TypeDefinition>)> {
        // Placeholder implementation
        // In a real scenario, this would compare type definitions to determine if they're duplicates or references
        let references = Vec::new();
        let duplicates = definitions.to_vec();
        Ok((references, duplicates))
    }

    /// Get analyzed languages
    fn get_analyzed_languages(&self, repositories: &[RepositoryInfo]) -> Vec<String> {
        repositories.iter()
            .filter_map(|r| r.primary_language.as_ref())
            .collect::<HashSet<_>>()
            .into_iter()
            .cloned()
            .collect()
    }
}

impl Default for MultiRepoAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of cross-repository analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossRepoAnalysisResult {
    pub repositories: Vec<RepositoryInfo>,
    pub relationships: Vec<RepositoryRelationship>,
    pub shared_dependencies: Vec<SharedDependency>,
    pub type_sharing: Vec<SharedType>,
    pub impact_scores: HashMap<String, RepositoryImpactScore>,
    pub aggregated_diagnostics: Vec<AggregatedDiagnostic>,
    pub analysis_metadata: AnalysisMetadata,
}

/// Repository relationship information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryRelationship {
    pub source_repo_id: String,
    pub target_repo_id: String,
    pub relationship_type: RelationshipType,
    pub strength: f32,
    pub metadata: serde_json::Value,
}

/// Types of repository relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    MonorepoSibling,
    Dependency,
    TypeSharing,
    LanguageSimilarity,
    Fork,
    Template,
}

/// Shared dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedDependency {
    pub dependency_name: String,
    pub affected_repositories: Vec<String>,
    pub dependency_type: DependencyType,
    pub impact_score: f32,
    pub metadata: serde_json::Value,
}

/// Types of dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Library,
    Framework,
    BuildTool,
    Testing,
    TypeDefinitions,
}

/// Shared type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedType {
    pub type_name: String,
    pub definitions: Vec<TypeDefinition>,
    pub references: Vec<TypeReference>,
    pub duplicates: Vec<TypeDefinition>,
    pub impact_score: f32,
    pub metadata: serde_json::Value,
}

/// Type definition information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub type_name: String,
    pub repository_id: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub definition_kind: TypeDefinitionKind,
    pub signature: String,
}

/// Type reference information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeReference {
    pub type_name: String,
    pub repository_id: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub reference_context: String,
}

/// Types of type definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeDefinitionKind {
    Interface,
    Type,
    Class,
    Enum,
    Struct,
    Trait,
}

/// Repository impact score breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryImpactScore {
    pub repository_id: String,
    pub overall_impact: f32,
    pub relationship_impact: f32,
    pub dependency_impact: f32,
    pub type_impact: f32,
    pub metadata: serde_json::Value,
}

/// Analysis metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    pub min_impact_threshold: f32,
    pub total_repositories: usize,
    pub active_repositories: usize,
    pub languages_analyzed: Vec<String>,
    pub analysis_timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = MultiRepoAnalyzer::new();
        assert_eq!(analyzer.min_impact_threshold, 0.1);
        assert!(!analyzer.include_inactive);
        assert!(analyzer.language_weights.contains_key("typescript"));
    }

    #[test]
    fn test_analyzer_configuration() {
        let analyzer = MultiRepoAnalyzer::new()
            .with_min_impact(0.5)
            .with_inactive(true)
            .with_language_weight("custom".to_string(), 0.8);
        
        assert_eq!(analyzer.min_impact_threshold, 0.5);
        assert!(analyzer.include_inactive);
        assert_eq!(analyzer.language_weights.get("custom"), Some(&0.8));
    }

    #[test]
    fn test_dependency_type_classification() {
        let analyzer = MultiRepoAnalyzer::new();
        
        assert!(matches!(analyzer.classify_dependency_type("@types/react"), DependencyType::TypeDefinitions));
        assert!(matches!(analyzer.classify_dependency_type("react"), DependencyType::Framework));
        assert!(matches!(analyzer.classify_dependency_type("typescript"), DependencyType::BuildTool));
        assert!(matches!(analyzer.classify_dependency_type("jest"), DependencyType::Testing));
        assert!(matches!(analyzer.classify_dependency_type("lodash"), DependencyType::Library));
    }

    #[test]
    fn test_impact_score_calculation() {
        let analyzer = MultiRepoAnalyzer::new();
        
        let relationships = vec![
            RepositoryRelationship {
                source_repo_id: "repo1".to_string(),
                target_repo_id: "repo2".to_string(),
                relationship_type: RelationshipType::MonorepoSibling,
                strength: 0.8,
                metadata: serde_json::json!({}),
            }
        ];
        
        let score = analyzer.calculate_relationship_impact("repo1", &relationships);
        assert_eq!(score, 0.8);
    }
}