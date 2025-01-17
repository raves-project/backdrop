use camino::{Utf8Path, Utf8PathBuf};

use crate::{
    database::{InsertIntoTable, DATABASE},
    error::{DatabaseError, RavesError},
    models::media::builder::MediaBuilder,
};

use super::Media;

impl Media {
    /// Gets a `Media` from disk or cache.
    #[tracing::instrument]
    pub async fn new(path: Utf8PathBuf) -> Result<Self, RavesError> {
        let mut conn = DATABASE.acquire().await?;

        // query the db for media with given path
        let media = sqlx::query_as::<_, Media>("SELECT *, id as `Uuid!` FROM info WHERE path = $1")
            .bind(path.to_string())
            .fetch_optional(&mut *conn)
            .await?;

        if let Some(media) = media {
            Ok(media)
        } else {
            // otherwise, make the metadata ourselves
            Self::load_from_disk(&path).await
        }
    }

    /// Loads a piece of media from disk.
    ///
    /// This function will also cache the media file's metadata into the
    /// database. If that fails, this function will error.
    #[tracing::instrument]
    pub async fn load_from_disk(path: &Utf8Path) -> Result<Self, RavesError> {
        // make sure the file exists
        let path_str = path.to_string();
        if !path.exists() {
            tracing::error!("the file doesn't exist");
            return Err(RavesError::MediaDoesntExist {
                path: path_str.clone(),
            });
        }

        // get metadata
        tracing::debug!("checking file properties...");
        let media = MediaBuilder::default().apply(path).await?;

        // ok ok... we have everything else. let's cache to the database now..!
        tracing::debug!("saving media to database...");
        let mut conn = DATABASE.acquire().await?;
        let query = media.make_insertion_query();

        query
            .execute(&mut *conn)
            .await
            .inspect_err(|e| {
                tracing::warn!(
                    "Failed to insert new media into database. err: {e}, media: \n{media:#?}"
                )
            })
            .map_err(|e| DatabaseError::InsertionFailed(e.to_string()))?;

        Ok(media)
    }
}
