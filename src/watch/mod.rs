//! Watches for changes inside of given folders.
//!
//! When a new file appears, it'll be added into the database! If an existing
//! file changes, it'll have its metadata reviewed and, if necessary, changed
//! in the database.

use std::{path::Path, time::Duration};

use async_watcher::{notify::RecursiveMode, AsyncDebouncer, DebouncedEventKind};
use camino::Utf8Path;

use futures::stream::StreamExt;

use crate::{config::Config, models::media::Media};

/// A 'daemon' that watches for file changes and
pub struct Watch;

impl Watch {
    /// Begins watching for file changes. When it detects one, it'll update
    /// metadata when necessary.
    ///
    /// NOTE: You should use this with `tokio::spawn`.
    #[tracing::instrument(skip_all)]
    pub async fn watch() {
        tracing::info!("starting watcher...");
        let (mut debouncer, mut file_events) =
            AsyncDebouncer::new_with_channel(Duration::from_millis(2000), None)
                .await
                .expect("watcher should be configured correctly");

        let paths = Config::read().await.watched_paths.clone();
        tracing::debug!("got the following paths: {paths:?}");

        let watcher = debouncer.watcher();
        for path in &paths {
            let res = watcher
                .watch(path.as_std_path(), RecursiveMode::Recursive)
                .inspect_err(|e| {
                    tracing::warn!("Failed to start watching folder! err: {e}, path: `{path}`")
                });

            if res.is_ok() {
                tracing::info!("The file watcher is now watching path: `{path}`");
            }
        }

        // start off by checking metadata for all watched files
        {
            tracing::info!("The watcher is now online! Performing initial scan on all files...");

            let stream = tokio_stream::iter(paths.into_iter())
                .map(|p| async move {
                    Self::handle_dir(p.as_std_path().to_path_buf()).await;
                })
                .buffered(5);

            tokio_stream::StreamExt::chunks_timeout(
                stream,
                3,
                Duration::from_millis(1000 * 60 * 10),
            )
            .collect::<Vec<_>>()
            .await;

            tracing::info!("Initial scan complete!");
        }

        // TODO: keep up with config changes. that'll require `select!` and
        // some other task w/ `mpsc`
        //
        // TODO(2025-01-16): hey i bet we can just restart the watcher instead... lol

        // when anything changes, we must scan its ENTIRE directory.
        // see: https://github.com/notify-rs/notify/issues/412
        while let Some(res_ev) = file_events.recv().await {
            tracing::debug!("File event received! ev: {res_ev:?}");

            // if it's an error, complain and move on...
            let Ok(events) = res_ev else {
                tracing::warn!("File watcher failed to handle event: {res_ev:?}");
                continue;
            };

            // spawn tasks to asynchronously handle the events
            for event in events {
                tracing::debug!("Handling event... ev: {event:?}");

                if event.kind == DebouncedEventKind::Any {
                    // files will have their metadata updated.
                    //
                    // folders will be further split into subtasks for each
                    // contained file to be updated.
                    if event.path.is_dir() {
                        tokio::spawn(Self::handle_dir(event.path.clone()));
                    } else {
                        tokio::spawn(Self::handle_file(event.path.clone()));
                    }
                }
            }
        }

        tracing::debug!("Watcher has died! New file changes will not be detected.");
    }

    #[tracing::instrument(skip_all)]
    async fn handle_file(path: impl AsRef<Path>) {
        let path = path.as_ref();
        let path_str = path.to_string_lossy();
        tracing::debug!("Working on file at `{path_str}`...");

        // give up if we don't have a utf-8 path.
        //
        // this shouldn't occur on Linux/Android, so we're chillin
        let Some(utf8_path) = Utf8Path::from_path(path) else {
            tracing::warn!("Failed to process file, as its path wasn't UTF-8. path: `{path_str}`");
            return;
        };

        // actually perform the update
        let try_update_metdata = Media::update_metadata(utf8_path).await;

        // report error, if any
        if let Err(e) = try_update_metdata {
            tracing::error!("Failed to update metadata for file at `{path_str}`. See error: `{e}`");
        }

        tracing::debug!("Completed file at `{path_str}`!");
    }

    #[tracing::instrument(skip_all)]
    async fn handle_dir(path: impl AsRef<Path>) {
        let path = path.as_ref();
        let path_str = path.to_string_lossy();
        tracing::debug!("Handling directory at `{path_str}`...");

        // we'll need to 'walk' the folder to flatten out its contents!
        let mut walk_dir = async_walkdir::WalkDir::new(path);

        while let Some(res_entry) = walk_dir.next().await {
            // when we hit an error, report it and move on...
            let Ok(entry) = res_entry else {
                tracing::warn!("Walking directory failed. See: {res_entry:?}");
                continue;
            };

            // spawn a new task for each file! just to keep things quick
            tokio::spawn(Self::handle_file(entry.path()));
        }

        tracing::debug!("Completed directory at `{path_str}`!");
    }
}
