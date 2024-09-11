//! Helps to connect to the database.

use surrealdb::{
    engine::local::{Db, SurrealKV},
    Surreal,
};

use crate::error::DatabaseError;

pub struct RavesDb {
    pub media_info: Surreal<Db>,
    pub thumbnails: Surreal<Db>,
}

impl RavesDb {
    pub const INFO_TABLE: &str = "info";

    /// Attempts to connect to the database according to the constants.
    pub async fn connect() -> Result<Self, DatabaseError> {
        const MEDIA_INFO_PATH: &str = "raves_media_info.db";
        const THUMBNAIL_PATH: &str = "raves_thumbnails.db";

        // create database connections
        let (media_info, thumbnails) = tokio::try_join! {
            Surreal::new::<SurrealKV>(MEDIA_INFO_PATH),
            Surreal::new::<SurrealKV>(THUMBNAIL_PATH)
        }?;

        media_info.use_ns("raves").await?;
        media_info.use_db("media").await?;

        Ok(Self {
            media_info,
            thumbnails,
        })
    }
}
