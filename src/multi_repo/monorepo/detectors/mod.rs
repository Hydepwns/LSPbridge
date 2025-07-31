//! Workspace detectors for different monorepo types

use super::types::WorkspaceLayout;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

pub mod bazel;
pub mod cargo;
pub mod custom;
pub mod lerna;
pub mod npm;
pub mod nx;
pub mod pnpm;
pub mod rush;

/// Trait for detecting specific workspace types
#[async_trait]
pub trait WorkspaceDetector {
    /// Detect workspace type at the given root path
    async fn detect(&self, root: &Path) -> Result<Option<WorkspaceLayout>>;

    /// Get the workspace type this detector handles
    fn workspace_type(&self) -> &'static str;
}

/// Registry of all available workspace detectors
pub struct DetectorRegistry {
    detectors: Vec<Box<dyn WorkspaceDetector + Send + Sync>>,
}

impl DetectorRegistry {
    /// Create a new registry with all standard detectors
    pub fn new() -> Self {
        let detectors: Vec<Box<dyn WorkspaceDetector + Send + Sync>> = vec![
            Box::new(npm::NpmDetector),
            Box::new(pnpm::PnpmDetector),
            Box::new(lerna::LernaDetector),
            Box::new(cargo::CargoDetector),
            Box::new(nx::NxDetector),
            Box::new(rush::RushDetector),
            Box::new(bazel::BazelDetector),
            Box::new(custom::CustomDetector),
        ];

        Self { detectors }
    }

    /// Try to detect workspace type using all registered detectors
    pub async fn detect(&self, root: &Path) -> Result<Option<WorkspaceLayout>> {
        for detector in &self.detectors {
            if let Some(layout) = detector.detect(root).await? {
                return Ok(Some(layout));
            }
        }
        Ok(None)
    }
}

impl Default for DetectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}