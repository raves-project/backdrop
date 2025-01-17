use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use hash::{HashUpToDate, MediaHash};
use sqlx::{
    query::Query,
    sqlite::SqliteArguments,
    types::{Json, Uuid},
    Sqlite,
};

use crate::{
    database::{InsertIntoTable, DATABASE},
    error::{DatabaseError, HashError, RavesError},
};

use super::{
    metadata::{Format, OtherMetadataMap, SpecificMetadata},
    tags::Tag,
    thumbnail::Thumbnail,
};

pub mod hash;
pub mod load;

/// Some media file.
#[derive(
    Clone,
    Debug,
    PartialEq,
    PartialOrd,
    serde::Serialize,
    serde::Deserialize,
    sqlx::FromRow,
    sqlx::Encode,
    sqlx::Type,
)]
pub struct Media {
    /// Unique ID identifying which piece of media is represented.
    ///
    /// This should match with the thumbnail database.
    pub id: Uuid,

    /// The last known file path for this media file.
    pub path: String,

    /// How large the file is, in bytes.
    pub filesize: i64,

    /// The MIME type (format) of the file.
    pub format: Json<Format>,

    /// The time the file was created, according to the file system.
    ///
    /// This could be inaccurate or missing depending on the file's source.
    pub creation_date: Option<DateTime<Utc>>,

    /// The time the file was last modified, according to the file system.
    ///
    /// Might be inaccurate or missing.
    pub modification_date: Option<DateTime<Utc>>,

    /// The time the file was first noted by Raves.
    pub first_seen_date: DateTime<Utc>,

    /// The media's width (horizontal) in pixels.
    pub width_px: u32,

    /// The media's height (vertical) in pixels.
    pub height_px: u32,

    /// Additional metadata that's specific to the media's kind, such as a
    /// video's framerate.
    pub specific_metadata: Json<SpecificMetadata>,

    /// Metadata that isn't immensely common, but can be read as a string.
    ///
    /// Or, in other words, it's a hashmap of data.
    ///
    /// This is stored as `Json` for the database.
    pub other_metadata: Option<Json<OtherMetadataMap>>,

    /// The tags of a media file. Note that these can come from the file's EXIF
    /// metadata or Rave's internals.
    pub tags: Json<Vec<Tag>>,
}

impl Media {
    /// Grabs the path of this media file.
    pub fn path(&self) -> Utf8PathBuf {
        self.path.clone().into()
    }

    /// Updates this file's metadata in the database.
    pub async fn update_metadata(path: &Utf8Path) -> Result<(), RavesError> {
        // TODO: optimize using CRC32 to check if we need to update?
        // might require another table..?

        Self::load_from_disk(path).await.map(|_| ())
    }

    /// Returns the thumbnail from the database for this media file.
    pub async fn get_thumbnail(&self, _id: &Uuid) -> Result<Thumbnail, RavesError> {
        // see if we have a thumbnail in the database
        if let Some(thumbnail) = self.database_get_thumbnail().await? {
            return Ok(thumbnail);
        }

        // the file doesn't have one either! let's make one ;D
        let thumbnail = Thumbnail::new(&self.id().await?).await;
        thumbnail.create().await?; // this makes the file
        Ok(thumbnail)
    }

    pub fn specific_type(&self) -> SpecificMetadata {
        self.specific_metadata.clone().0
    }

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
        let new_hash = MediaHash::new(self.id, self.path()).await?;

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

// the private impl
impl Media {
    /// Creates a string from this media file's path.
    pub(crate) fn path_str(&self) -> String {
        self.path().to_string()
    }

    /// Grabs this media file's unique identifier.
    async fn id(&self) -> Result<Uuid, DatabaseError> {
        let mut conn = DATABASE.acquire().await?;

        let record = sqlx::query_as::<_, Media>("SELECT id FROM info WHERE path = $1")
            .bind(&self.path)
            .fetch_one(&mut *conn)
            .await?;

        Ok(record.id)
    }

    /// Tries to grab the thumbnail from the database, if it's there.
    async fn database_get_thumbnail(&self) -> Result<Option<Thumbnail>, RavesError> {
        let mut conn = DATABASE.acquire().await?;
        let id = self.id().await?;

        let thumbnail =
            sqlx::query_as::<_, Thumbnail>("SELECT * FROM thumbnail WHERE image_id = $1")
                .bind(id)
                .fetch_one(&mut *conn)
                .await?;

        Ok(Some(thumbnail))
    }
}

impl InsertIntoTable for Media {
    fn make_insertion_query(&self) -> Query<'_, Sqlite, SqliteArguments<'_>> {
        sqlx::query!(
            r#"
        INSERT INTO info 
        (id, path, filesize, format, creation_date, modification_date, first_seen_date, width_px, height_px, specific_metadata, other_metadata, tags)
        VALUES
        ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT(id)
        DO UPDATE SET
            path = excluded.path,
            filesize = excluded.filesize,
            format = excluded.format,
            creation_date = excluded.creation_date,
            width_px = excluded.width_px,
            height_px = excluded.height_px,
            specific_metadata = excluded.specific_metadata,
            other_metadata = excluded.other_metadata,
            tags = excluded.tags;
        "#,
            self.id,
            self.path,
            self.filesize,
            self.format,
            self.creation_date,
            self.modification_date,
            self.first_seen_date,
            self.width_px,
            self.height_px,
            self.specific_metadata,
            self.other_metadata,
            self.tags
        )
    }
}
