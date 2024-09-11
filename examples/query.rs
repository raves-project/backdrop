//! Let's see if we can chain query method calls...

use std::ffi::OsString;

use backdrop::{
    database::RavesDb,
    models::{media::Media, metadata::types::Orientation},
};

#[tokio::main]
async fn main() {
    let db = RavesDb::connect().await.unwrap();
    let all: Vec<Media> = db.media_info.select("info").await.unwrap();

    // SEARCH: all media where:
    // - orientation is portrait,
    // - filename contains a number, and
    // - resolution is >1080p
    let executed_search = all
        .iter()
        .filter(|m| {
            matches!(
                Orientation::from(m.metadata.resolution.clone()),
                Orientation::Portrait
            )
        })
        .filter(|m| {
            m.metadata
                .path
                .file_name()
                .unwrap_or(OsString::new().as_os_str())
                .to_string_lossy()
                .to_string()
                .contains(('0'..='9').collect::<Vec<_>>().as_slice())
        })
        .filter(|m| m.metadata.resolution.width > 1920 && m.metadata.resolution.height > 1080)
        .collect::<Vec<_>>();

    println!("found results: {:#?}", executed_search);
}
