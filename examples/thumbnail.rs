//! Let's try and grab a thumbnail for an arbitrary file!

use backdrop::{
    config::{BugReportInfo, Config},
    database::RavesDb,
    models::media::Media,
};
use surrealdb::RecordId;
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    Config::init_config(
        &[],
        dirs::data_dir().unwrap(),
        dirs::cache_dir().unwrap(),
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
    )
    .await;

    let db = RavesDb::connect().await.unwrap();
    let mut one = db.media_info.query("SELECT * FROM info").await.unwrap();

    let m: Vec<Media> = one.take("media").unwrap();
    let id: Vec<RecordId> = one.take("id").unwrap();

    let media = m.first().unwrap();
    let id = id.first().unwrap();

    // create a thumbnail for it
    let thumbnail = media.get_thumbnail(id).await.unwrap();
    thumbnail.create().await.unwrap();

    println!("result should be at path: {:#?}", thumbnail.path_str());
}
