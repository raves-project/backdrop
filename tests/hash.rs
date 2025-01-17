mod common;

#[cfg(test)]
mod tests {
    use backdrop::{
        database::DATABASE,
        models::media::{hash::MediaHash, Media},
    };

    use crate::common::{setup, Setup};

    #[tokio::test]
    async fn beach_hash() {
        // perform setup
        setup(Setup::new(6672)).await;
        tracing::info!("post setup");

        // hashing the beach file should give the same one each time
        let beach_hash = MediaHash::hash_file("tests/assets/beach_location_and_tagged.jpg")
            .await
            .expect("hashing should go ok");
        tracing::info!("post beach_hash");

        // load media and grab its hash (to check if they're the same)
        let beach_media =
            Media::load_from_disk("tests/assets/beach_location_and_tagged.jpg".into())
                .await
                .expect("load media from disk");
        tracing::info!("post beach_media");

        // hash the media
        let (beach_media_hash, _) = beach_media
            .hash()
            .await
            .expect("media file hashing should work too");
        tracing::info!("post beach_media_hash");
        assert_eq!(
            *beach_hash.as_bytes(),
            *beach_media_hash.hash,
            "hash_file hash + media file hash"
        );

        beach_media_hash
            .add_to_table()
            .await
            .expect("add hash to hashes table");
        tracing::info!("post db insertion");

        // grabbing that from the db should yield the same hash back!
        let mut conn = DATABASE.acquire().await.unwrap();
        let from_database =
            sqlx::query_as::<_, MediaHash>("SELECT * FROM hashes WHERE media_id = $1")
                .bind(beach_media.id)
                .fetch_one(&mut *conn)
                .await
                .unwrap();
        tracing::info!("post db query");

        // the initial hash + db hash should be equal!
        assert_eq!(
            *beach_hash.as_bytes(),
            *from_database.hash,
            "hash_file hash + db hash are the same"
        );
    }
}
