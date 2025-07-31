pub mod privacy_filter;
pub mod workspace_filter;

pub use privacy_filter::PrivacyFilter;
pub use workspace_filter::WorkspaceFilter;

/// Privacy filtering levels
#[derive(Debug, Clone, PartialEq)]
pub enum FilterLevel {
    None,
    Standard,
    Strict,
}

// Re-export PrivacyPolicy from core for convenience
pub use crate::core::PrivacyPolicy;
