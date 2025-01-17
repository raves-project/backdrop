use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use sqlx::{
    query::Query,
    sqlite::SqliteArguments,
    types::{Json, Uuid},
    Sqlite,
};

use crate::{
    database::{InsertIntoTable, DATABASE},
    error::{DatabaseError, RavesError},
};

use super::{
    metadata::{Format, OtherMetadataMap, SpecificMetadata},
    tags::Tag,
    thumbnail::Thumbnail,
};

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

        // we havn't cached one yet...
        // first, let's see if the media file contains one for us to use
        // TODO: put this back if we use exiv2 again or something
        // if let Some(raw_thumbnail) = self.gexif2_get_thumbnail().await? {
        //     // let's save the file first
        //     let rep = Thumbnail::new(id).await;
        //     rep.save_from_buffer(&raw_thumbnail, self).await?;
        // }

        // the file doesn't have one either! let's make one ;D
        let thumbnail = Thumbnail::new(&self.id().await?).await;
        thumbnail.create().await?; // this makes the file
        Ok(thumbnail)
    }

    pub fn specific_type(&self) -> SpecificMetadata {
        self.specific_metadata.clone().0
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

    // /// Tries to get a thumbnail from the media file's EXIF data.
    // ///
    // /// Note that this is often uncommon for fully-digital media, like screenshots.
    // async fn gexif2_get_thumbnail(&self) -> Result<Option<Vec<u8>>, RavesError> {
    //     // check the file's properties
    //     let m = block_in_place(|| {
    //         rexiv2::Metadata::new_from_path(self.path()).map_err(|_e| {
    //             RavesError::MediaDoesntExist {
    //                 path: self.path_str(),
    //             }
    //         })
    //     })?;

    //     Ok(m.get_thumbnail().map(|bstr| bstr.to_vec()))
    // }
}
