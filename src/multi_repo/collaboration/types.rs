use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Team member information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    /// Unique member ID
    pub id: String,

    /// Member name
    pub name: String,

    /// Email address
    pub email: String,

    /// Role in the team
    pub role: TeamRole,

    /// Active status
    pub active: bool,

    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
}

/// Team member roles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TeamRole {
    /// Can view diagnostics
    Viewer,

    /// Can be assigned diagnostics
    Developer,

    /// Can review assignments
    Reviewer,

    /// Can maintain repositories
    Maintainer,

    /// Can assign diagnostics to others
    Lead,

    /// Full administrative access
    Admin,
}

/// Diagnostic assignment to team members
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticAssignment {
    /// Assignment ID
    pub id: String,

    /// Repository ID
    pub repository_id: String,

    /// File path
    pub file_path: String,

    /// Diagnostic hash (for tracking)
    pub diagnostic_hash: String,

    /// Assigned team member ID
    pub assignee_id: String,

    /// Member who created the assignment
    pub assigned_by: String,

    /// Assignment timestamp
    pub assigned_at: DateTime<Utc>,

    /// Due date (optional)
    pub due_date: Option<DateTime<Utc>>,

    /// Current status
    pub status: AssignmentStatus,

    /// Priority level
    pub priority: Priority,

    /// Notes about the assignment
    pub notes: Option<String>,
}

/// Assignment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssignmentStatus {
    /// Newly assigned
    Open,

    /// Being worked on
    InProgress,

    /// Needs review
    Review,

    /// Completed
    Resolved,

    /// Won't be fixed
    Closed,
}

/// Priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

/// Team metrics data
pub type TeamMetrics = Vec<(TeamMember, u32, Option<i64>)>;