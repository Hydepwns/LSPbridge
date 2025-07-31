use crate::core::errors::DatabaseError;
use rusqlite::Connection;
use std::collections::HashMap;

pub struct MigrationRunner {
    migrations: HashMap<&'static str, &'static str>,
}

impl MigrationRunner {
    pub fn new() -> Self {
        let mut migrations = HashMap::new();
        migrations.insert("1.0", include_str!("v1_initial.sql"));
        
        Self { migrations }
    }

    pub fn run_migrations(&self, conn: &mut Connection) -> Result<(), DatabaseError> {
        // Get current schema version
        let current_version = self.get_schema_version(conn)?;
        
        // For now, we only have one migration, so we just ensure it's applied
        if current_version.is_none() {
            conn.execute_batch(self.migrations["1.0"])
                .map_err(|e| DatabaseError::Sqlite {
                    operation: "run_migrations".to_string(),
                    message: format!("Failed to run initial migration: {}", e),
                    source: e,
                })?;
            
            self.set_schema_version(conn, "1.0")?;
        }
        
        Ok(())
    }

    fn get_schema_version(&self, conn: &Connection) -> Result<Option<String>, DatabaseError> {
        // Check if metadata table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='metadata'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::Sqlite {
                operation: "check_metadata_table".to_string(),
                message: e.to_string(),
                source: e,
            })?;

        if !table_exists {
            return Ok(None);
        }

        // Get schema version
        use rusqlite::OptionalExtension;
        let version: Option<String> = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| DatabaseError::Sqlite {
                operation: "get_schema_version".to_string(),
                message: e.to_string(),
                source: e,
            })?;

        Ok(version)
    }

    fn set_schema_version(&self, conn: &Connection, version: &str) -> Result<(), DatabaseError> {
        conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?, ?)",
            ["schema_version", version],
        )
        .map_err(|e| DatabaseError::Sqlite {
            operation: "set_schema_version".to_string(),
            message: e.to_string(),
            source: e,
        })?;
        
        Ok(())
    }
}

impl Default for MigrationRunner {
    fn default() -> Self {
        Self::new()
    }
}