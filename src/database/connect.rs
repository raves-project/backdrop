//! Helps to connect to the database.

use diesel::SqliteConnection;
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    sync_connection_wrapper::SyncConnectionWrapper,
};

type SqlLiteConnection = SyncConnectionWrapper<SqliteConnection>;
type DbPool = Pool<SqlLiteConnection>;

use super::error::DatabaseError;

pub const PROTOCOL: &str = "sqlite";
pub const FILENAME: &str = "raves.db";

/// Attempts to connect to the database according to the constants
pub async fn connect() -> Result<DbPool, DatabaseError> {
    let database_url = format!("{PROTOCOL}:{FILENAME}");

    let config = AsyncDieselConnectionManager::<SqlLiteConnection>::new(database_url);
    let pool = Pool::builder(config).build().map_err(|e| {
        DatabaseError::ConnectionError(format!("couldn't build the pool. from diesel: {e}"))
    })?;

    Ok(pool)
}
