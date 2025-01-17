use camino::Utf8Path;
use sqlx::{query::Query, sqlite::SqliteArguments, Sqlite};
use uuid::Uuid;

use crate::{
    database::{InsertIntoTable, DATABASE},
    error::{DatabaseError, HashError},
};

use super::Media;

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

impl Media {
    /// Computes this media file's hash.
    ///
    /// It also checks if the media file's hash is up-to-date in the database, but
    /// DOES NOT update it.
    ///
    /// ## Errors
    ///
    /// This method can fail if the backing file no longer exists or the
    /// database connection errors.
    pub async fn hash(&self) -> Result<(MediaHash, HashUpToDate), HashError> {
        let mut conn = DATABASE
            .acquire()
            .await
            .inspect_err(|e| tracing::error!("Database connection failed! err: {e}"))?;

        // get old hash
        let old_hash_query = sqlx::query_as!(
            MediaHash,
            r#"SELECT
            media_id as `media_id: Uuid`,
            hash
            FROM hashes
            WHERE media_id = $1"#,
            self.id
        )
        .fetch_optional(&mut *conn)
        .await
        .inspect_err(|e| {
            tracing::debug!("Didn't find old hash in hashes table. ignored and totally ok err: {e}")
        });

        // generate new hash
        let new_hash = MediaHash::new(self.id, &self.path).await?;

        // check if they match.
        //
        // if they don't, we'll complain and tell the caller
        let mut is_up_to_date = HashUpToDate::NotInDatabase;
        if let Ok(Some(old_hash)) = old_hash_query {
            if old_hash != new_hash {
                tracing::debug!(
                    "Hash mismatch! {:#x?} != {:#x?}",
                    old_hash.hash,
                    new_hash.hash
                );
                is_up_to_date = HashUpToDate::Outdated;
            } else {
                is_up_to_date = HashUpToDate::UpToDate;
            }
        }

        Ok((new_hash, is_up_to_date))
    }
}
