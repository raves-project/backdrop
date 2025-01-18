//! This module tests the database.
//!
//! In particular, it focuses on media cache generation, testing against known
//! assets and their metadata fields.
//!
//! Futher contributions to these tests, like weird media, metadata, or
//! regression cases, are greatly appreciated!

mod common;

#[cfg(test)]
mod tests {
    use std::{env::temp_dir, str::FromStr as _};

    use backdrop::{
        database::{self, DATABASE, RAVES_DB_FILE},
        models::media::{metadata::Format, Media},
    };
    use camino::{Utf8Path, Utf8PathBuf};
    use sqlx::{sqlite::SqliteConnectOptions, Sqlite};
    use uuid::Uuid;

    /// The database can cache metadata for the beach photo.
    #[tokio::test]
    async fn beach() {
        // set up the database
        {
            let db_temp_dir = Utf8PathBuf::try_from(temp_dir())
                .unwrap()
                .join(Uuid::new_v4().to_string())
                .join("_raves_db");

            tokio::fs::create_dir_all(&db_temp_dir)
                .await
                .expect("create db temp dir");

            database::DB_FOLDER_PATH
                .set(db_temp_dir)
                .expect("db folder path should be unset");
        }

        // grab database connection from pool
        let mut conn = DATABASE.acquire().await.expect("make database connection");

        // ask it to cache the beach image.
        //
        // (loading from disk will also cache metadata into db)
        let media = Media::load(Utf8Path::new("tests/assets/beach_location_and_tagged.jpg"))
            .await
            .expect("beach image should be found. (make sure you're running from crate root)");

        let media_id = media.id; // TODO: remove media local and just use .id on it directly

        // check if its registered in db
        let media_from_db = sqlx::query_as::<_, Media>("SELECT * FROM info WHERE id = $1")
            .bind(media_id)
            .fetch_one(&mut *conn)
            .await
            .expect("media should be registered in db");

        // check some of the metadata
        assert_eq!(media_from_db.id, media_id, "id match");
        assert!(
            media_from_db.path.contains("beach_location_and_tagged.jpg"),
            "path contains filename"
        );
        assert_eq!(media_from_db.filesize, 5_194_673_i64, "filesize");
        assert_eq!(
            media_from_db.format.0,
            Format::new_from_mime("image/jpeg").unwrap(),
            "mime format"
        );
    }

    /// Ensures that migrations don't error, even when running them twice.
    #[tokio::test]
    async fn migrations() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();

        // this closure will make multiple pools using those migrations.
        //
        // we can run it several times and it should still not panic :D
        let make_pool = |raves_db_folder: Utf8PathBuf| async move {
            let options = SqliteConnectOptions::from_str(&format!(
                "sqlite://{raves_db_folder}/{RAVES_DB_FILE}"
            ))
            .expect("database opts str")
            .create_if_missing(true);

            // connect to the pool
            let pool = sqlx::Pool::<Sqlite>::connect_lazy_with(options);

            // we'll also run migrations here real quick
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .expect("migrations");

            // run them again!
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .expect("migrations twice");

            // make a quick query to ensure things work
            //
            // (pool is lazy)
            let mut conn = pool.acquire().await.expect("make db conn");
            sqlx::query("SELECT * FROM info")
                .execute(&mut *conn)
                .await
                .expect("running select");
        };

        for _ in 0..3 {
            let mut set = tokio::task::JoinSet::new();

            let tempdir = temp_dir();
            let p = Utf8PathBuf::try_from(tempdir).unwrap();
            set.spawn(make_pool(p));

            set.join_all().await;
        }
    }
}
