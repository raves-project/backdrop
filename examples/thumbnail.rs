//! Let's try and grab a thumbnail for an arbitrary file!

use backdrop::{
    config::{BugReportInfo, Config},
    database::DATABASE,
    models::media::Media,
};
use camino::Utf8PathBuf;
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    Config::init_config(
        &[],
        Utf8PathBuf::try_from(dirs::data_dir().unwrap()).unwrap(),
        Utf8PathBuf::try_from(dirs::cache_dir().unwrap()).unwrap(),
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

    let mut conn = DATABASE.acquire().await.expect("db connection");
    let media = sqlx::query_as::<_, Media>("SELECT * FROM info LIMIT 1")
        .fetch_one(&mut *conn)
        .await
        .unwrap();

    // create a thumbnail for it
    let thumbnail = media.get_thumbnail(&media.id).await.unwrap();
    thumbnail.create().await.unwrap();

    println!("result should be at path: {:#?}", thumbnail.path_str());
}
