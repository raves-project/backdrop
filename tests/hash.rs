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
        let beach_media = Media::load("tests/assets/beach_location_and_tagged.jpg")
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

    #[tokio::test]
    async fn hardcoded_hash() {
        setup(Setup::new(6673)).await;

        const FEAR_AVIF_HASH: [u8; 32] = [
            0xf8, 0xc, 0xa1, 0x56, 0x78, 0xa3, 0x16, 0xe8, 0x29, 0xa5, 0xd4, 0x9e, 0x1a, 0xad,
            0x9b, 0xdc, 0x66, 0xb6, 0xa1, 0xa2, 0xe6, 0x2a, 0xac, 0xc3, 0x47, 0xfe, 0xba, 0x71,
            0x15, 0xec, 0xd5, 0x2c,
        ];

        // hash the file
        let media = Media::load("tests/assets/fear.avif").await.unwrap();
        let hash = media.hash().await.unwrap().0;

        assert_eq!(
            FEAR_AVIF_HASH, *hash.hash,
            "hardcoded hash is eq to runtime one."
        );
    }
}
