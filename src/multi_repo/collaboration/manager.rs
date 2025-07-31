use anyhow::Result;
use std::path::Path;
use tracing::{debug, info};

use super::database::TeamDatabase;
use super::types::{TeamMember, DiagnosticAssignment, AssignmentStatus, TeamMetrics};

/// Manages team collaboration features
pub struct CollaborationManager {
    database: TeamDatabase,
}

impl CollaborationManager {
    /// Create a new collaboration manager
    pub async fn new(db_path: &Path) -> Result<Self> {
        let database = TeamDatabase::connect(db_path).await?;
        
        info!("Collaboration manager initialized with database at {:?}", db_path);
        
        Ok(Self { database })
    }

    /// Add a new team member
    pub async fn add_team_member(&self, member: TeamMember) -> Result<()> {
        info!("Adding team member: {} ({})", member.name, member.email);
        self.database.add_member(member).await
    }

    /// Get a specific team member
    pub async fn get_team_member(&self, id: &str) -> Result<Option<TeamMember>> {
        debug!("Fetching team member with ID: {}", id);
        self.database.get_member(id).await
    }

    /// List all active team members
    pub async fn list_team_members(&self) -> Result<Vec<TeamMember>> {
        debug!("Listing all active team members");
        self.database.list_members().await
    }

    /// Create a new diagnostic assignment
    pub async fn create_assignment(&self, assignment: DiagnosticAssignment) -> Result<()> {
        info!(
            "Creating assignment {} for {} in repository {}",
            assignment.id, assignment.assignee_id, assignment.repository_id
        );
        self.database.create_assignment(assignment).await
    }

    /// Update assignment status
    pub async fn update_assignment_status(
        &self,
        assignment_id: &str,
        new_status: AssignmentStatus,
        updated_by: &str,
    ) -> Result<()> {
        info!(
            "Updating assignment {} status to {:?} by {}",
            assignment_id, new_status, updated_by
        );
        self.database
            .update_assignment_status(assignment_id, new_status, updated_by)
            .await
    }

    /// Get assignments for a team member
    pub async fn get_member_assignments(
        &self,
        member_id: &str,
        status_filter: Option<AssignmentStatus>,
    ) -> Result<Vec<DiagnosticAssignment>> {
        debug!(
            "Fetching assignments for member {} with status filter: {:?}",
            member_id, status_filter
        );
        self.database
            .get_member_assignments(member_id, status_filter)
            .await
    }

    /// Get team performance metrics
    pub async fn get_team_metrics(&self) -> Result<TeamMetrics> {
        debug!("Fetching team performance metrics");
        self.database.get_team_metrics().await
    }

    /// Record assignment history
    pub async fn record_assignment_action(
        &self,
        assignment_id: &str,
        member_id: &str,
        action: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
    ) -> Result<()> {
        debug!(
            "Recording action '{}' for assignment {} by member {}",
            action, assignment_id, member_id
        );
        self.database
            .add_history(assignment_id, member_id, action, old_value, new_value)
            .await
    }

    /// Assign a diagnostic to a team member
    pub async fn assign_diagnostic(
        &self,
        repository_id: String,
        file_path: String,
        diagnostic_hash: String,
        assignee_id: String,
        assigned_by: String,
        priority: super::types::Priority,
        due_date: Option<chrono::DateTime<chrono::Utc>>,
        notes: Option<String>,
    ) -> Result<String> {
        let assignment_id = format!("assign_{}", uuid::Uuid::new_v4());
        
        let assignment = DiagnosticAssignment {
            id: assignment_id.clone(),
            repository_id,
            file_path,
            diagnostic_hash,
            assignee_id,
            assigned_by,
            assigned_at: chrono::Utc::now(),
            due_date,
            status: AssignmentStatus::Open,
            priority,
            notes,
        };

        self.create_assignment(assignment).await?;
        
        info!("Diagnostic assigned with ID: {}", assignment_id);
        Ok(assignment_id)
    }

    /// Mark assignment as in progress
    pub async fn start_assignment(&self, assignment_id: &str, member_id: &str) -> Result<()> {
        self.update_assignment_status(assignment_id, AssignmentStatus::InProgress, member_id)
            .await
    }

    /// Mark assignment as resolved
    pub async fn resolve_assignment(&self, assignment_id: &str, member_id: &str) -> Result<()> {
        self.update_assignment_status(assignment_id, AssignmentStatus::Resolved, member_id)
            .await
    }

    /// Close assignment without resolution
    pub async fn close_assignment(&self, assignment_id: &str, member_id: &str) -> Result<()> {
        self.update_assignment_status(assignment_id, AssignmentStatus::Closed, member_id)
            .await
    }
}