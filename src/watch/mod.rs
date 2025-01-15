//! Watches for changes inside of given folders.
//!
//! When a new file appears, it'll be added into the database! If an existing
//! file changes, it'll have its metadata reviewed and, if necessary, changed
//! in the database.

use std::{path::Path, time::Duration};

use async_watcher::{AsyncDebouncer, DebouncedEventKind};
use camino::Utf8Path;
use tokio::sync::RwLock;

use futures::stream::StreamExt;

use crate::{config::Config, models::media::Media};

pub struct Watch;

impl Watch {
    #[tracing::instrument(skip_all)]
    pub async fn watch(conf: RwLock<Config>) {
        tracing::info!("starting watcher...");
        let (mut debouncer, mut file_events) =
            AsyncDebouncer::new_with_channel(Duration::from_millis(100), None)
                .await
                .expect("watcher should be configured correctly");

        let paths = conf.read().await.watched_paths.clone();
        tracing::debug!("got the following paths: {paths:?}");

        let watcher = debouncer.watcher();
        _ = paths.iter().map(|p| {
            watcher.watch(
                p.as_std_path(),
                async_watcher::notify::RecursiveMode::Recursive,
            )
        });

        // start off by checking metadata for all watched files
        tracing::info!("the watcher is now online. performing initial scan on all files...");

        let stream = tokio_stream::iter(paths.iter())
            .map(|p| async move {
                Self::handle_dir(p.as_std_path()).await;
            })
            .buffered(5);

        tokio_stream::StreamExt::chunks_timeout(stream, 3, Duration::from_millis(1000 * 60 * 10))
            .collect::<Vec<_>>()
            .await;

        tracing::info!("initial scan complete!");

        // TODO: keep up with config changes. that'll require `select!` and
        // some other task w/ `mpsc`

        // when anything changes, we must scan its ENTIRE directory.
        // see: https://github.com/notify-rs/notify/issues/412
        while let Some(f) = file_events.recv().await {
            if let Ok(events) = f {
                _ = events.iter().map(|event| async {
                    if matches!(event.kind, DebouncedEventKind::Any) {
                        // handle folders
                        if event.path.is_dir() {
                            Self::handle_dir(&event.path).await;
                        }

                        // handle individual files
                        if event.path.is_file() {
                            Self::handle_file(&event.path).await
                        }
                    }
                });
            }
        }

        // TODO: but pretend this is here rn
    }

    #[tracing::instrument]
    async fn handle_file(path: &Path) {
        tracing::debug!("working on file...");

        let Some(utf8_path) = Utf8Path::from_path(path) else {
            tracing::warn!(
                "Failed to process file, as its name could not be converted to UTF-8. path: {}",
                path.display()
            );
            return;
        };

        if let Err(e) = Media::update_metadata(utf8_path).await {
            tracing::error!(
                "Failed to update metadata for file at path `{}`. See error: `{e}`",
                path.to_string_lossy()
            )
        }
        tracing::debug!("file handled.");
    }

    #[tracing::instrument]
    async fn handle_dir(path: &Path) {
        tracing::debug!("starting...");
        let mut walk_dir = async_walkdir::WalkDir::new(path);

        while let Some(entry) = walk_dir.next().await {
            if let Ok(entry) = entry {
                Self::handle_file(&entry.path()).await;
            }
        }
        tracing::debug!("all entries walked");
    }
}
