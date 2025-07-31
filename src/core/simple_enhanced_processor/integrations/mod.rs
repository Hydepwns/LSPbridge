//! External integrations for the enhanced processor

pub mod config_integration;
pub mod git_integration;

pub use config_integration::ConfigIntegration;
pub use git_integration::GitIntegrationWrapper;