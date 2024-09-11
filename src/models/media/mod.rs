use std::path::{Path, PathBuf};

use surrealdb::RecordId;
use tokio::task::block_in_place;

use crate::{
    database::RavesDb,
    error::{DatabaseError, RavesError},
    models::metadata::Metadata,
};

use super::{metadata::SpecificMetadata, tags::Tag, thumbnail::Thumbnail};

pub mod load;

/// Some media file.
#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct Media {
    pub metadata: Metadata,
    //
    // The identifer of the media. Used for loading cached metadata,
    // thumbnails, and potentially other information.
    // pub id: RecordId,
    //
    /// The tags of a media file. Note that these can come from the file's EXIF
    /// metadata or Rave's internals.
    pub tags: Vec<Tag>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct MediaRecord {
    pub id: RecordId,
    pub media: Media,
}

impl Media {
    /// Updates this file's metadata in the database.
    pub async fn update_metadata(path: &Path) -> Result<(), RavesError> {
        // TODO: optimize using CRC32 to check if we need to update?
        // might require another table..?

        Self::load_from_disk(path).await.map(|_| ())
    }

    /// Returns the thumbnail from the database for this media file.
    pub async fn get_thumbnail(&self, id: &RecordId) -> Result<Thumbnail, RavesError> {
        // see if we have a thumbnail in the database
        if let Some(thumbnail) = self.database_get_thumbnail().await? {
            return Ok(thumbnail);
        }

        // we havn't cached one yet...
        // first, let's see if the media file contains one for us to use
        if let Some(raw_thumbnail) = self.gexif2_get_thumbnail().await? {
            // let's save the file first
            let rep = Thumbnail::new(id).await;
            rep.save_from_buffer(&raw_thumbnail, self).await?;
        }

        // the file doesn't have one either! let's make one ;D
        let thumbnail = Thumbnail::new(&self.id().await?).await;
        thumbnail.create().await?; // this makes the file
        Ok(thumbnail)
    }

    pub fn specific_type(&self) -> SpecificMetadata {
        self.metadata.specific.clone()
    }
}

// the private impl
impl Media {
    /// Grabs the path of this media file.
    pub(crate) fn path(&self) -> PathBuf {
        self.metadata.path.clone()
    }

    /// Creates a string from this media file's path.
    pub(crate) fn path_str(&self) -> String {
        self.path().display().to_string()
    }

    /// Grabs this media file's unique identifier.
    async fn id(&self) -> Result<RecordId, DatabaseError> {
        let db = RavesDb::connect().await?;

        let mut response = db
            .media_info
            .query("SELECT id FROM info WHERE path = $path")
            .bind(("path", self.path()))
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let maybe: Option<MediaRecord> = response.take(0).map_err(DatabaseError::QueryFailed)?;

        maybe
            .ok_or(DatabaseError::EmptyResponse(self.path_str()))
            .map(|mr| mr.id)
    }

    /// Tries to grab the thumbnail from the database, if it's there.
    async fn database_get_thumbnail(&self) -> Result<Option<Thumbnail>, RavesError> {
        let (db, id) = tokio::try_join!(RavesDb::connect(), self.id())?;

        // grab thumbnail from database
        let mut response = db
            .thumbnails
            .query("SELECT * FROM thumbnail WHERE image_id = $id")
            .bind(("id", id))
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let maybe: Option<Thumbnail> = response.take(0).map_err(DatabaseError::QueryFailed)?;

        Ok(maybe)
    }

    /// Tries to get a thumbnail from the media file's EXIF data.
    ///
    /// Note that this is often uncommon for fully-digital media, like screenshots.
    async fn gexif2_get_thumbnail(&self) -> Result<Option<Vec<u8>>, RavesError> {
        // check the file's properties
        let m = block_in_place(|| {
            rexiv2::Metadata::new_from_path(self.path()).map_err(|_e| {
                RavesError::MediaDoesntExist {
                    path: self.path_str(),
                }
            })
        })?;

        Ok(m.get_thumbnail().map(|bstr| bstr.to_vec()))
    }
}
