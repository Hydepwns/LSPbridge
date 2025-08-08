//! Command types and enums for multi-repository CLI operations
//!
//! This module defines all the command line argument types, enums, and data structures
//! used by the multi-repository CLI system.

use clap::Subcommand;
use std::path::PathBuf;

/// Main multi-repository commands
#[derive(Debug, Subcommand)]
pub enum MultiRepoCommand {
    /// Register a repository in the multi-repo system
    Register {
        /// Repository path
        path: PathBuf,

        /// Repository name
        #[arg(short, long)]
        name: Option<String>,

        /// Remote URL
        #[arg(short = 'u', long)]
        remote_url: Option<String>,

        /// Primary language
        #[arg(short, long)]
        language: Option<String>,

        /// Tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// List registered repositories
    List {
        /// Show inactive repositories
        #[arg(short, long)]
        all: bool,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Analyze diagnostics across all repositories
    Analyze {
        /// Minimum cross-repo impact score to display
        #[arg(short, long, default_value = "0.3")]
        min_impact: f32,

        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Detect monorepo structure
    DetectMonorepo {
        /// Root directory to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Register detected subprojects
        #[arg(short, long)]
        register: bool,
    },

    /// Manage repository relationships
    Relate {
        /// Source repository ID
        source: String,

        /// Target repository ID
        target: String,

        /// Relationship type
        #[arg(value_enum)]
        relation: RelationTypeArg,

        /// Additional data (JSON)
        #[arg(short, long)]
        data: Option<String>,
    },

    /// Team collaboration commands
    Team {
        #[command(subcommand)]
        command: TeamCommand,
    },

    /// Find cross-repository type references
    Types {
        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: OutputFormat,
    },
}

/// Team collaboration sub-commands
#[derive(Debug, Subcommand)]
pub enum TeamCommand {
    /// Add a team member
    AddMember {
        /// Member name
        name: String,

        /// Email address
        email: String,

        /// Role
        #[arg(value_enum)]
        role: TeamRoleArg,
    },

    /// List team members
    ListMembers {
        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Assign a diagnostic to a team member
    Assign {
        /// Repository ID
        repo: String,

        /// File path
        file: String,

        /// Diagnostic hash
        hash: String,

        /// Assignee email
        assignee: String,

        /// Priority
        #[arg(short, long, value_enum)]
        priority: PriorityArg,

        /// Due date (YYYY-MM-DD)
        #[arg(short, long)]
        due_date: Option<String>,
    },

    /// Update assignment status
    UpdateStatus {
        /// Assignment ID
        id: String,

        /// New status
        #[arg(value_enum)]
        status: AssignmentStatusArg,

        /// Optional note
        #[arg(short, long)]
        note: Option<String>,
    },

    /// Show assignment history
    History {
        /// Member email (optional)
        #[arg(short, long)]
        member: Option<String>,

        /// Repository ID (optional)
        #[arg(short, long)]
        repo: Option<String>,

        /// Number of recent assignments to show
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: OutputFormat,
    },
}

/// Output format options
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}

/// Repository relationship types
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum RelationTypeArg {
    Dependency,
    SharedType,
    MonorepoSibling,
    Fork,
    Template,
}

/// Team role options
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum TeamRoleArg {
    Developer,
    Reviewer,
    Maintainer,
    Admin,
}

/// Assignment priority levels
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum PriorityArg {
    Low,
    Medium,
    High,
    Critical,
}

/// Assignment status options
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum AssignmentStatusArg {
    Open,
    InProgress,
    Review,
    Resolved,
    Closed,
}

impl From<RelationTypeArg> for crate::multi_repo::registry::RelationType {
    fn from(arg: RelationTypeArg) -> Self {
        match arg {
            RelationTypeArg::Dependency => crate::multi_repo::registry::RelationType::Dependency,
            RelationTypeArg::SharedType => crate::multi_repo::registry::RelationType::SharedTypes,
            RelationTypeArg::MonorepoSibling => crate::multi_repo::registry::RelationType::MonorepoSibling,
            RelationTypeArg::Fork => crate::multi_repo::registry::RelationType::ApiRelation,
            RelationTypeArg::Template => crate::multi_repo::registry::RelationType::DevDependency,
        }
    }
}

impl From<TeamRoleArg> for crate::multi_repo::collaboration::TeamRole {
    fn from(arg: TeamRoleArg) -> Self {
        match arg {
            TeamRoleArg::Developer => crate::multi_repo::collaboration::TeamRole::Developer,
            TeamRoleArg::Reviewer => crate::multi_repo::collaboration::TeamRole::Reviewer,
            TeamRoleArg::Maintainer => crate::multi_repo::collaboration::TeamRole::Maintainer,
            TeamRoleArg::Admin => crate::multi_repo::collaboration::TeamRole::Admin,
        }
    }
}

impl From<PriorityArg> for crate::multi_repo::collaboration::Priority {
    fn from(arg: PriorityArg) -> Self {
        match arg {
            PriorityArg::Low => crate::multi_repo::collaboration::Priority::Low,
            PriorityArg::Medium => crate::multi_repo::collaboration::Priority::Medium,
            PriorityArg::High => crate::multi_repo::collaboration::Priority::High,
            PriorityArg::Critical => crate::multi_repo::collaboration::Priority::Critical,
        }
    }
}

impl From<AssignmentStatusArg> for crate::multi_repo::collaboration::AssignmentStatus {
    fn from(arg: AssignmentStatusArg) -> Self {
        match arg {
            AssignmentStatusArg::Open => crate::multi_repo::collaboration::AssignmentStatus::Open,
            AssignmentStatusArg::InProgress => crate::multi_repo::collaboration::AssignmentStatus::InProgress,
            AssignmentStatusArg::Review => crate::multi_repo::collaboration::AssignmentStatus::Review,
            AssignmentStatusArg::Resolved => crate::multi_repo::collaboration::AssignmentStatus::Resolved,
            AssignmentStatusArg::Closed => crate::multi_repo::collaboration::AssignmentStatus::Closed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relation_type_conversion() {
        let dependency = RelationTypeArg::Dependency;
        let converted: crate::multi_repo::registry::RelationType = dependency.into();
        assert!(matches!(converted, crate::multi_repo::registry::RelationType::Dependency));
    }

    #[test]
    fn test_team_role_conversion() {
        let developer = TeamRoleArg::Developer;
        let converted: crate::multi_repo::collaboration::TeamRole = developer.into();
        assert!(matches!(converted, crate::multi_repo::collaboration::TeamRole::Developer));
    }

    #[test]
    fn test_priority_conversion() {
        let high = PriorityArg::High;
        let converted: crate::multi_repo::collaboration::Priority = high.into();
        assert!(matches!(converted, crate::multi_repo::collaboration::Priority::High));
    }

    #[test]
    fn test_status_conversion() {
        let in_progress = AssignmentStatusArg::InProgress;
        let converted: crate::multi_repo::collaboration::AssignmentStatus = in_progress.into();
        assert!(matches!(converted, crate::multi_repo::collaboration::AssignmentStatus::InProgress));
    }
}