use uuid::Uuid;

/// A hash for a media file, stored in the [`HASHES_TABLE`].
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash, sqlx::FromRow)]
pub struct MediaHash {
    /// The media file's UUID.
    pub media_id: Uuid,
    /// The media file's hash.
    pub hash: Vec<u8>,
}

/// Whether a media file's hash is up-to-date.
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd)]
pub enum HashUpToDate {
    UpToDate,
    Outdated,
    NotInDatabase,
}
