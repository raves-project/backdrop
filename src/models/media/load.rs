use std::path::{Path, PathBuf};

use crate::{
    database::RavesDb,
    error::{DatabaseError, RavesError},
    models::metadata::builder::MetadataBuilder,
};

use super::Media;

impl Media {
    /// Gets a `Media` from disk or cache.
    #[tracing::instrument]
    pub async fn new(path: PathBuf) -> Result<Self, RavesError> {
        let db = RavesDb::connect().await?;
        // query the db for image
        let mut results = db
            .media_info
            .query("SELECT * FROM info WHERE path = $path")
            .bind(("path", path.clone()))
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let r: Result<Option<Media>, surrealdb::Error> = results.take(0);

        if let Ok(Some(media)) = r {
            // return it here
            Ok(media)
        } else {
            // otherwise, make the metadata ourselves
            Self::load_from_disk(&path).await
        }
    }

    /// Loads file (with metadata) from disk... no matter what.
    #[tracing::instrument]
    pub async fn load_from_disk(path: &Path) -> Result<Self, RavesError> {
        // make sure the file exists
        let path_str = path.display().to_string();
        if !path.exists() {
            tracing::error!("the file doesn't exist");
            return Err(RavesError::MediaDoesntExist {
                path: path_str.clone(),
            });
        }

        // get metadata
        tracing::debug!("checking file properties...");
        let metadata = MetadataBuilder::default().apply(path).await?;

        // ok ok... we have everything else. let's save it now!
        tracing::debug!("saving media to database...");
        let db = RavesDb::connect().await?;
        let v: Vec<Media> = db
            .media_info
            .insert("info")
            .content(Self {
                metadata,
                tags: Vec::new(), // TODO
            })
            .await
            .map_err(|e| DatabaseError::InsertionFailed(e.to_string()))?;

        let constructed = v
            .first()
            .ok_or(DatabaseError::InsertionFailed(
                "didn't get anything from return vec! :p".into(),
            ))?
            .clone();

        Ok(constructed)
    }
}
