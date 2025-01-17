use camino::Utf8Path;
use sqlx::{query::Query, sqlite::SqliteArguments, Sqlite};
use uuid::Uuid;

use crate::{
    database::{InsertIntoTable, DATABASE},
    error::{DatabaseError, HashError},
};

/// A hash for a media file, stored in the [`HASHES_TABLE`].
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash, sqlx::FromRow)]
pub struct MediaHash {
    /// The media file's UUID.
    pub media_id: Uuid,
    /// The media file's hash.
    pub hash: Vec<u8>,
}

impl MediaHash {
    /// Creates a new [`MediaHash`] from the given required components.
    ///
    /// This will actually compute the hash of the file. Use struct
    /// construction instead if you've already got it.
    #[tracing::instrument]
    pub async fn new<P: AsRef<Utf8Path> + core::fmt::Debug>(
        media_id: Uuid,
        path: P,
    ) -> Result<Self, HashError> {
        let path = path.as_ref();

        let blake3_hash = Self::hash_file(path).await?;

        Ok(Self {
            hash: blake3_hash.as_bytes().into(),
            media_id,
        })
    }

    /// Hashes the file at the given path.
    #[tracing::instrument]
    pub async fn hash_file<P: AsRef<Utf8Path> + core::fmt::Debug>(
        path: P,
    ) -> Result<blake3::Hash, HashError> {
        let path = path.as_ref();
        let mut hasher = blake3::Hasher::new();

        // read the file and get its hash
        hasher
            .update_mmap_rayon(path)
            .inspect_err(|e| tracing::warn!("`blake3` file hashing failed! err: {e}"))
            .map_err(|e| HashError::FileReadFailure(path.to_path_buf(), e))
            .map(|hasher| hasher.finalize())
    }

    /// Attempts to add this hash to the [`HASHES_TABLE`].
    #[tracing::instrument]
    pub async fn add_to_table(&self) -> Result<(), DatabaseError> {
        let mut conn = DATABASE
            .acquire()
            .await
            .inspect_err(|e| tracing::error!("Failed to connect to database. err: {e}"))
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        self.make_insertion_query()
            .execute(&mut *conn)
            .await
            .inspect_err(|e| tracing::error!("Hash insertion failed! err: {e}"))
            .map_err(|e| DatabaseError::InsertionFailed(e.to_string()))
            .map(|_query_response| ())
    }
}

impl InsertIntoTable for MediaHash {
    #[tracing::instrument]
    fn make_insertion_query(&self) -> Query<'_, Sqlite, SqliteArguments<'_>> {
        // NOTE: if changing `HASHES_TABLE`, also change this!
        sqlx::query!(
            r#"
            INSERT INTO hashes (media_id, hash) 
            VALUES ($1, $2) 
            ON CONFLICT(media_id)
            DO UPDATE SET
                hash = excluded.hash;
            "#,
            self.media_id,
            self.hash
        )
    }
}

/// Whether a media file's hash is up-to-date.
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd)]
pub enum HashUpToDate {
    UpToDate,
    Outdated,
    NotInDatabase,
}
