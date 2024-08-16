use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("failed to connect to the database. see: {0}")]
    ConnectionError(String),
}
