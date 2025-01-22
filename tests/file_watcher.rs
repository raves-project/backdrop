//! Tests the file watcher to ensure that it's working properly.

mod common;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use backdrop::{
        database::{DATABASE, INFO_TABLE},
        models::media::{metadata::Format, Media},
        watch::Watch,
    };

    use camino::Utf8PathBuf;
    use temp_dir::TempDir;

    use crate::common::{self, Setup};

    /// Ensures that the File Watcher doesn't immediately die... :p
    #[tokio::test]
    async fn watcher_alive() {
        common::setup(Setup::new(6669)).await;

        // spawn the watcher as a task
        let task = tokio::spawn(Watch::watch());

        // sleep for a bit
        tokio::time::sleep(Duration::from_millis(100)).await;

        // ensure the watcher is still running
        assert!(!task.is_finished(), "watcher should run indefinitely!");

        // kill it to stop the test lol
        task.abort();
    }

    /// Checks that the watcher can find files.
    ///
    /// We'll ensure that it finds at least the beach file.
    #[tokio::test]
    async fn find_file_in_temp_dir() {
        // generate a temp dir
        let temp_dir = TempDir::new().unwrap();
        let temp_dir_path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();
        println!("temp dir located at: `{temp_dir_path}`");
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        // set up the app
        common::setup(Setup {
            port: 6670,
            watched_folders: [temp_dir_path.clone()].into(),
        })
        .await;
        let mut conn = DATABASE.acquire().await.unwrap();

        // turn on the file watcher
        tokio::spawn(Watch::watch());

        // wipe the `info` table
        sqlx::query(&format!("DELETE FROM {INFO_TABLE}"))
            .execute(&mut *conn)
            .await
            .expect("remove all from info table");

        // copy a photo to the temp dir
        tokio::time::sleep(Duration::from_millis(150)).await;
        tokio::fs::copy("tests/assets/fear.avif", temp_dir_path.join("fear.avif"))
            .await
            .expect("copy to temp dir should work");

        // wait... then check if we got metadata!
        tokio::time::sleep(Duration::from_millis(150)).await;
        let media = sqlx::query_as::<_, Media>(&format!("SELECT * FROM {INFO_TABLE}"))
            .fetch_one(&mut *conn)
            .await
            .expect("should find media after adding it");

        assert!(
            media.path.contains("fear.avif"),
            "path should contain og filename. was: {}",
            media.path
        );
        assert_eq!(
            media.format.0,
            Format::new_from_mime("image/avif").unwrap(),
            "media mime ty should match"
        );
    }
}
