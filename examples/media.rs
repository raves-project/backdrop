//! Use this to test the different types of media.

use std::path::PathBuf;

use tokio::{sync::RwLock, time::sleep};

use backdrop::{
    config::{BugReportInfo, Config},
    models::media::Media,
};
use tracing::Level;

// note: this can be a file or folder with MANY media files
const MEDIA_FILE_PATH: &str = "/home/barrett/Pictures/CalyxOS Backup Main/DCIM/Snapchat";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // let's start the watch with the given paths.
    // then we can see what the database looks like afterwards!
    let conf = Config::new(
        vec![PathBuf::from(MEDIA_FILE_PATH)],
        dirs::data_dir().unwrap().join("backdrop_media_example"),
        dirs::cache_dir().unwrap().join("backdrop_media_example"),
        BugReportInfo {
            app_name: "backdrop_media_example".to_string(),
            app_version: "0.1.0".to_string(),
            device: "desktop".to_string(),
            display: "lineage_and_some_other_stuff".to_string(),
            target_triple: "x86_64-farts-gnu".to_string(),
            commit: "unknown".to_string(),
            repo: "https://github.com/onkoe/backdrop".to_string(),
            build_time: "unknown".to_string(),
        },
    );

    tokio::select! {
        _ = wait_and_start_watcher(conf) => {},
        _ = forever_loop_and_watch_db() => {},
        _ = async_ctrlc::CtrlC::new().expect("ctrlc handler should just work") => {},
    }
}

async fn forever_loop_and_watch_db() {
    let db = backdrop::database::RavesDb::connect().await.unwrap();

    loop {
        let v_result: Result<Vec<Media>, surrealdb::Error> = db.media_info.select("info").await;

        let Ok(v) = v_result else {
            tracing::info!("empty db...");
            sleep(std::time::Duration::from_secs(10)).await;

            tracing::info!("fetching new db info...");
            continue;
        };

        tracing::info!("here are all paths in the database: \n");
        for (i, m) in v.iter().enumerate() {
            println!("media {i}: {}", m.metadata.path.display());
        }
        tracing::info!("database: {:?}", v);
        tracing::info!("...");
        sleep(std::time::Duration::from_secs(10)).await;
        tracing::info!("fetching new db info...");
    }
}

async fn wait_and_start_watcher(conf: Config) {
    tracing::debug!("HEY: waiting to start watcher...");
    sleep(std::time::Duration::from_secs(10)).await;
    tracing::debug!("watcher will now begin!");
    backdrop::watch::Watch::watch(RwLock::new(conf)).await
}
