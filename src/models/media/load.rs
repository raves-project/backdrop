use camino::{Utf8Path, Utf8PathBuf};

use crate::{database::DATABASE, error::RavesError, models::metadata::builder::MediaBuilder};

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

    /// Loads file (with metadata) from disk... no matter what.
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
        let _metadata = MediaBuilder::default().apply(path).await?;

        // ok ok... we have everything else. let's save it now!
        tracing::debug!("saving media to database...");

        // let query = sqlx::query!("INSERT INTO info (id, path, filesize, format, creation_date, modification_date, first_seen_date, width_px, height_px, specific_metadata, other_metadata, tags) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)")
        // .bind(Uuid::new_v4()).bind;

        // let db = RavesDb::connect().await?;
        // let v: Vec<Media> = db
        //     .media_info
        //     .insert("info")
        //     .content(Self {
        //         metadata,
        //         tags: Vec::new(), // TODO
        //     })
        //     .await
        //     .map_err(|e| DatabaseError::InsertionFailed(e.to_string()))?;

        // let constructed = v
        //     .first()
        //     .ok_or(DatabaseError::InsertionFailed(
        //         "didn't get anything from return vec! :p".into(),
        //     ))?
        //     .clone();

        // Ok(constructed)
        todo!()
    }
}
