use camino::Utf8Path;
use chrono::{DateTime, Utc};
use sqlx::{
    query::Query,
    sqlite::SqliteArguments,
    types::{Json, Uuid},
    Sqlite,
};

use super::tags::Tag;
use crate::{database::InsertIntoTable, error::RavesError};
use metadata::{Format, OtherMetadataMap, SpecificMetadata};

mod builder;
pub mod hash;
pub mod load;
pub mod metadata;

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
    /// Loads a media file's metadata.
    ///
    /// This function handles all path, hashing, and caching operations. You
    /// may safely call it for anything.
    #[tracing::instrument]
    pub async fn load<P: AsRef<Utf8Path> + core::fmt::Debug>(path: P) -> Result<Self, RavesError> {
        load::load_internal(path.as_ref()).await
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
