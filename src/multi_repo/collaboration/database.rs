use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::types::{TeamMember, TeamRole, DiagnosticAssignment, AssignmentStatus, Priority, TeamMetrics};

/// Team collaboration database
pub struct TeamDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl TeamDatabase {
    /// Connect to team database
    pub async fn connect(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create team database directory")?;
        }

        let conn = Connection::open(path).context("Failed to open team database")?;

        // Initialize schema
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS team_members (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                role TEXT NOT NULL,
                active BOOLEAN DEFAULT 1,
                last_activity INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS diagnostic_assignments (
                id TEXT PRIMARY KEY,
                repository_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                diagnostic_hash TEXT NOT NULL,
                assignee_id TEXT NOT NULL,
                assigned_by TEXT NOT NULL,
                assigned_at INTEGER NOT NULL,
                due_date INTEGER,
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                notes TEXT,
                completed_at INTEGER,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (assignee_id) REFERENCES team_members(id),
                FOREIGN KEY (assigned_by) REFERENCES team_members(id)
            );
            
            CREATE TABLE IF NOT EXISTS assignment_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                assignment_id TEXT NOT NULL,
                member_id TEXT NOT NULL,
                action TEXT NOT NULL,
                old_value TEXT,
                new_value TEXT,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (assignment_id) REFERENCES diagnostic_assignments(id),
                FOREIGN KEY (member_id) REFERENCES team_members(id)
            );
            
            CREATE TABLE IF NOT EXISTS team_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                member_id TEXT NOT NULL,
                repository_id TEXT NOT NULL,
                resolved_count INTEGER DEFAULT 0,
                avg_resolution_time INTEGER,
                last_updated INTEGER NOT NULL,
                FOREIGN KEY (member_id) REFERENCES team_members(id),
                UNIQUE(member_id, repository_id)
            );
            
            CREATE INDEX IF NOT EXISTS idx_assignments_assignee ON diagnostic_assignments(assignee_id);
            CREATE INDEX IF NOT EXISTS idx_assignments_status ON diagnostic_assignments(status);
            CREATE INDEX IF NOT EXISTS idx_assignments_repo ON diagnostic_assignments(repository_id);
            CREATE INDEX IF NOT EXISTS idx_history_assignment ON assignment_history(assignment_id);
            CREATE INDEX IF NOT EXISTS idx_metrics_member ON team_metrics(member_id);
            "#,
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Add a new team member
    pub async fn add_member(&self, member: TeamMember) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now().timestamp();

        let role = role_to_string(&member.role);

        conn.execute(
            r#"
            INSERT INTO team_members 
            (id, name, email, role, active, last_activity, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                member.id,
                member.name,
                member.email,
                role,
                member.active,
                member.last_activity.map(|dt| dt.timestamp()),
                now,
                now,
            ],
        )?;

        Ok(())
    }

    /// Get team member by ID
    pub async fn get_member(&self, id: &str) -> Result<Option<TeamMember>> {
        let conn = self.conn.lock().await;

        let result = conn
            .query_row(
                "SELECT * FROM team_members WHERE id = ?1",
                params![id],
                |row| {
                    Ok(TeamMember {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        email: row.get(2)?,
                        role: string_to_role(&row.get::<_, String>(3)?),
                        active: row.get(4)?,
                        last_activity: row
                            .get::<_, Option<i64>>(5)?
                            .map(|ts| DateTime::from_timestamp(ts, 0).unwrap()),
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// List all active team members
    pub async fn list_members(&self) -> Result<Vec<TeamMember>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare("SELECT * FROM team_members WHERE active = 1 ORDER BY name")?;

        let members = stmt
            .query_map([], |row| {
                Ok(TeamMember {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    email: row.get(2)?,
                    role: string_to_role(&row.get::<_, String>(3)?),
                    active: row.get(4)?,
                    last_activity: row
                        .get::<_, Option<i64>>(5)?
                        .map(|ts| DateTime::from_timestamp(ts, 0).unwrap()),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(members)
    }

    /// Create a new diagnostic assignment
    pub async fn create_assignment(&self, assignment: DiagnosticAssignment) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now().timestamp();

        let status = status_to_string(&assignment.status);
        let priority = priority_to_string(&assignment.priority);

        conn.execute(
            r#"
            INSERT INTO diagnostic_assignments
            (id, repository_id, file_path, diagnostic_hash, assignee_id, assigned_by,
             assigned_at, due_date, status, priority, notes, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                assignment.id,
                assignment.repository_id,
                assignment.file_path,
                assignment.diagnostic_hash,
                assignment.assignee_id,
                assignment.assigned_by,
                assignment.assigned_at.timestamp(),
                assignment.due_date.map(|dt| dt.timestamp()),
                status,
                priority,
                assignment.notes,
                now,
            ],
        )?;

        // Record in history
        drop(conn); // Release lock before calling add_history
        self.add_history(
            &assignment.id,
            &assignment.assigned_by,
            "created",
            None,
            Some(&format!("Assigned to {}", assignment.assignee_id)),
        )
        .await?;

        Ok(())
    }

    /// Update assignment status
    pub async fn update_assignment_status(
        &self,
        assignment_id: &str,
        new_status: AssignmentStatus,
        updated_by: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now().timestamp();

        // Get current status
        let old_status: String = conn.query_row(
            "SELECT status FROM diagnostic_assignments WHERE id = ?1",
            params![assignment_id],
            |row| row.get(0),
        )?;

        let new_status_str = status_to_string(&new_status);

        // Update status
        let completed_at = if new_status == AssignmentStatus::Resolved {
            Some(now)
        } else {
            None
        };

        conn.execute(
            r#"
            UPDATE diagnostic_assignments 
            SET status = ?1, updated_at = ?2, completed_at = ?3
            WHERE id = ?4
            "#,
            params![new_status_str, now, completed_at, assignment_id],
        )?;

        // Record in history
        drop(conn); // Release lock before calling add_history
        self.add_history(
            assignment_id,
            updated_by,
            "status_changed",
            Some(&old_status),
            Some(&new_status_str),
        )
        .await?;

        // Update metrics if resolved
        if new_status == AssignmentStatus::Resolved {
            self.update_member_metrics(assignment_id).await?;
        }

        Ok(())
    }

    /// Get assignments for a team member
    pub async fn get_member_assignments(
        &self,
        member_id: &str,
        status_filter: Option<AssignmentStatus>,
    ) -> Result<Vec<DiagnosticAssignment>> {
        let conn = self.conn.lock().await;

        let (query, status_str) = if let Some(status) = status_filter {
            let status_str = status_to_string(&status);
            (
                "SELECT * FROM diagnostic_assignments WHERE assignee_id = ?1 AND status = ?2 ORDER BY priority, assigned_at",
                Some(status_str)
            )
        } else {
            (
                "SELECT * FROM diagnostic_assignments WHERE assignee_id = ?1 ORDER BY priority, assigned_at",
                None
            )
        };

        let mut stmt = conn.prepare(query)?;

        let assignments = if let Some(ref status) = status_str {
            stmt.query_map(params![member_id, status], Self::map_assignment_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map(params![member_id], Self::map_assignment_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };

        Ok(assignments)
    }

    /// Get team metrics
    pub async fn get_team_metrics(&self) -> Result<TeamMetrics> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            r#"
            SELECT m.*, 
                   COALESCE(SUM(tm.resolved_count), 0) as total_resolved,
                   AVG(tm.avg_resolution_time) as avg_time
            FROM team_members m
            LEFT JOIN team_metrics tm ON m.id = tm.member_id
            WHERE m.active = 1
            GROUP BY m.id
            ORDER BY total_resolved DESC
            "#,
        )?;

        let metrics = stmt
            .query_map([], |row| {
                let member = TeamMember {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    email: row.get(2)?,
                    role: string_to_role(&row.get::<_, String>(3)?),
                    active: row.get(4)?,
                    last_activity: row
                        .get::<_, Option<i64>>(5)?
                        .map(|ts| DateTime::from_timestamp(ts, 0).unwrap()),
                };

                let resolved_count: u32 = row.get(8)?;
                let avg_time: Option<i64> = row.get(9)?;

                Ok((member, resolved_count, avg_time))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(metrics)
    }

    /// Add history entry
    pub async fn add_history(
        &self,
        assignment_id: &str,
        member_id: &str,
        action: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now().timestamp();

        conn.execute(
            r#"
            INSERT INTO assignment_history
            (assignment_id, member_id, action, old_value, new_value, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![assignment_id, member_id, action, old_value, new_value, now],
        )?;

        Ok(())
    }

    /// Update member metrics when assignment is resolved
    async fn update_member_metrics(&self, assignment_id: &str) -> Result<()> {
        let conn = self.conn.lock().await;

        // Get assignment details
        let (assignee_id, repo_id, assigned_at): (String, String, i64) = conn.query_row(
            "SELECT assignee_id, repository_id, assigned_at FROM diagnostic_assignments WHERE id = ?1",
            params![assignment_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

        let now = Utc::now().timestamp();
        let resolution_time = now - assigned_at;

        // Update or insert metrics
        conn.execute(
            r#"
            INSERT INTO team_metrics (member_id, repository_id, resolved_count, avg_resolution_time, last_updated)
            VALUES (?1, ?2, 1, ?3, ?4)
            ON CONFLICT(member_id, repository_id) DO UPDATE SET
                resolved_count = resolved_count + 1,
                avg_resolution_time = ((avg_resolution_time * resolved_count) + ?3) / (resolved_count + 1),
                last_updated = ?4
            "#,
            params![assignee_id, repo_id, resolution_time, now],
        )?;

        Ok(())
    }

    /// Map database row to DiagnosticAssignment
    fn map_assignment_row(row: &rusqlite::Row) -> rusqlite::Result<DiagnosticAssignment> {
        Ok(DiagnosticAssignment {
            id: row.get(0)?,
            repository_id: row.get(1)?,
            file_path: row.get(2)?,
            diagnostic_hash: row.get(3)?,
            assignee_id: row.get(4)?,
            assigned_by: row.get(5)?,
            assigned_at: DateTime::from_timestamp(row.get::<_, i64>(6)?, 0).unwrap(),
            due_date: row
                .get::<_, Option<i64>>(7)?
                .map(|ts| DateTime::from_timestamp(ts, 0).unwrap()),
            status: string_to_status(&row.get::<_, String>(8)?),
            priority: string_to_priority(&row.get::<_, String>(9)?),
            notes: row.get(10)?,
        })
    }
}

// Helper functions for type conversion
fn role_to_string(role: &TeamRole) -> String {
    match role {
        TeamRole::Viewer => "viewer",
        TeamRole::Developer => "developer",
        TeamRole::Reviewer => "reviewer",
        TeamRole::Maintainer => "maintainer",
        TeamRole::Lead => "lead",
        TeamRole::Admin => "admin",
    }.to_string()
}

fn string_to_role(s: &str) -> TeamRole {
    match s {
        "viewer" => TeamRole::Viewer,
        "developer" => TeamRole::Developer,
        "reviewer" => TeamRole::Reviewer,
        "maintainer" => TeamRole::Maintainer,
        "lead" => TeamRole::Lead,
        "admin" => TeamRole::Admin,
        _ => TeamRole::Viewer,
    }
}

fn status_to_string(status: &AssignmentStatus) -> String {
    match status {
        AssignmentStatus::Open => "open",
        AssignmentStatus::InProgress => "in_progress",
        AssignmentStatus::Review => "review",
        AssignmentStatus::Resolved => "resolved",
        AssignmentStatus::Closed => "closed",
    }.to_string()
}

fn string_to_status(s: &str) -> AssignmentStatus {
    match s {
        "open" => AssignmentStatus::Open,
        "in_progress" => AssignmentStatus::InProgress,
        "review" => AssignmentStatus::Review,
        "resolved" => AssignmentStatus::Resolved,
        "closed" => AssignmentStatus::Closed,
        _ => AssignmentStatus::Open,
    }
}

fn priority_to_string(priority: &Priority) -> String {
    match priority {
        Priority::Critical => "critical",
        Priority::High => "high",
        Priority::Medium => "medium",
        Priority::Low => "low",
    }.to_string()
}

fn string_to_priority(s: &str) -> Priority {
    match s {
        "critical" => Priority::Critical,
        "high" => Priority::High,
        "medium" => Priority::Medium,
        "low" => Priority::Low,
        _ => Priority::Medium,
    }
}