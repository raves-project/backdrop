//! Helps to connect to the database.

use sea_orm::{Database, DatabaseConnection};

use super::error::DatabaseError;

pub const PROTOCOL: &str = "mysql";
pub const FILENAME: &str = "raves.db";

/// Attempts to connect to the database according to the constants
pub async fn connect() -> Result<DatabaseConnection, DatabaseError> {
    let formatted = format!("{PROTOCOL}:{FILENAME}");
    let connection = Database::connect(formatted)
        .await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
    Ok(connection)
}
