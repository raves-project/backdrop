use uuid::Uuid;

use crate::{
    database::{InsertIntoTable, DATABASE},
};
/// A hash for a media file, stored in the [`HASHES_TABLE`].
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash, sqlx::FromRow)]
pub struct MediaHash {
    /// The media file's UUID.
    pub media_id: Uuid,
    /// The media file's hash.
    pub hash: Vec<u8>,
}


impl InsertIntoTable for MediaHash {
    #[tracing::instrument]
    fn make_insertion_query(&self) -> Query<'_, Sqlite, SqliteArguments<'_>> {
        // NOTE: if changing `HASHES_TABLE`, also change this!
        sqlx::query!(
            "INSERT INTO hashes (media_id, hash) VALUES ($1, $2)",
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
