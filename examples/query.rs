//! Let's see if we can chain query method calls...

use backdrop::{
    database::DATABASE,
    models::media::{
        metadata::{Orientation, Resolution},
        Media,
    },
};

#[tokio::main]
async fn main() {
    let mut conn = DATABASE.acquire().await.expect("db conn");
    let all = sqlx::query_as::<_, Media>("SELECT * FROM info")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    // SEARCH: all media where:
    // - orientation is portrait,
    // - filename contains a number, and
    // - resolution is >1080p
    let executed_search = all
        .iter()
        .filter(|m| {
            matches!(
                Orientation::from(Resolution::new(m.width_px, m.height_px)),
                Orientation::Portrait
            )
        })
        .filter(|m| {
            m.path()
                .file_name()
                .unwrap_or_default()
                .to_string()
                .contains(('0'..='9').collect::<Vec<_>>().as_slice())
        })
        .filter(|m| m.width_px > 1920 && m.height_px > 1080)
        .collect::<Vec<_>>();

    println!("found results: {:#?}", executed_search);
}
