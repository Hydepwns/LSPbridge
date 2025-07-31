pub mod types;
pub mod database;
pub mod manager;
pub mod sync;

// Re-export main types and functionality
pub use types::{
    TeamMember, TeamRole, DiagnosticAssignment, AssignmentStatus, Priority, TeamMetrics
};
pub use database::TeamDatabase;
pub use manager::CollaborationManager;
pub use sync::{AssignmentSynchronizer, SyncResult, AssignmentConflict, ConflictType};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_collaboration_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_collaboration.db");
        
        let manager = CollaborationManager::new(&db_path).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_team_member_operations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_collaboration.db");
        
        let manager = CollaborationManager::new(&db_path).await.unwrap();
        
        let member = TeamMember {
            id: "test_member_1".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            role: TeamRole::Developer,
            active: true,
            last_activity: Some(Utc::now()),
        };
        
        // Add member
        manager.add_team_member(member.clone()).await.unwrap();
        
        // Get member
        let retrieved = manager.get_team_member("test_member_1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test User");
        
        // List members
        let members = manager.list_team_members().await.unwrap();
        assert_eq!(members.len(), 1);
    }

    #[tokio::test]
    async fn test_assignment_operations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_collaboration.db");
        
        let manager = CollaborationManager::new(&db_path).await.unwrap();
        
        // Add a team member first
        let member = TeamMember {
            id: "assignee_1".to_string(),
            name: "Assignee User".to_string(),
            email: "assignee@example.com".to_string(),
            role: TeamRole::Developer,
            active: true,
            last_activity: Some(Utc::now()),
        };
        manager.add_team_member(member).await.unwrap();
        
        let assigner = TeamMember {
            id: "assigner_1".to_string(),
            name: "Assigner User".to_string(),
            email: "assigner@example.com".to_string(),
            role: TeamRole::Lead,
            active: true,
            last_activity: Some(Utc::now()),
        };
        manager.add_team_member(assigner).await.unwrap();
        
        // Create assignment
        let assignment_id = manager.assign_diagnostic(
            "test_repo".to_string(),
            "src/main.rs".to_string(),
            "diagnostic_hash_123".to_string(),
            "assignee_1".to_string(),
            "assigner_1".to_string(),
            Priority::High,
            None,
            Some("Test assignment".to_string()),
        ).await.unwrap();
        
        // Get assignments for member
        let assignments = manager.get_member_assignments("assignee_1", None).await.unwrap();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].status, AssignmentStatus::Open);
        
        // Update assignment status
        manager.start_assignment(&assignment_id, "assignee_1").await.unwrap();
        
        let assignments = manager.get_member_assignments("assignee_1", Some(AssignmentStatus::InProgress)).await.unwrap();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].status, AssignmentStatus::InProgress);
        
        // Resolve assignment
        manager.resolve_assignment(&assignment_id, "assignee_1").await.unwrap();
        
        let resolved_assignments = manager.get_member_assignments("assignee_1", Some(AssignmentStatus::Resolved)).await.unwrap();
        assert_eq!(resolved_assignments.len(), 1);
    }

    #[tokio::test]
    async fn test_team_metrics() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_collaboration.db");
        
        let manager = CollaborationManager::new(&db_path).await.unwrap();
        
        // Add team members
        let member1 = TeamMember {
            id: "member1".to_string(),
            name: "Member One".to_string(),
            email: "member1@example.com".to_string(),
            role: TeamRole::Developer,
            active: true,
            last_activity: Some(Utc::now()),
        };
        manager.add_team_member(member1).await.unwrap();
        
        let member2 = TeamMember {
            id: "member2".to_string(),
            name: "Member Two".to_string(),
            email: "member2@example.com".to_string(),
            role: TeamRole::Lead,
            active: true,
            last_activity: Some(Utc::now()),
        };
        manager.add_team_member(member2).await.unwrap();
        
        // Get metrics (should work even with no resolved assignments)
        let metrics = manager.get_team_metrics().await.unwrap();
        assert_eq!(metrics.len(), 2);
        
        // All members should have 0 resolved assignments initially
        for (_, resolved_count, _) in &metrics {
            assert_eq!(*resolved_count, 0);
        }
    }
}