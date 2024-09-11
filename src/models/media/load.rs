use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use ffmpeg_next::{media, Rational};
use tokio::task::block_in_place;

use crate::{
    database::RavesDb,
    error::{DatabaseError, RavesError},
    models::metadata::{
        types::{Filesize, Format, MediaKind, Resolution},
        Metadata, SpecificMetadata,
    },
};

use super::Media;

impl Media {
    /// Gets a `Media` from disk or cache.
    #[tracing::instrument]
    pub async fn new(path: PathBuf) -> Result<Self, RavesError> {
        let db = RavesDb::connect().await?;
        // query the db for image
        let mut results = db
            .media_info
            .query("SELECT * FROM info WHERE path = $path")
            .bind(("path", path.clone()))
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let r: Result<Option<Media>, surrealdb::Error> = results.take(0);

        if let Ok(Some(media)) = r {
            // return it here
            Ok(media)
        } else {
            // otherwise, make the metadata ourselves
            Self::load_from_disk(&path).await
        }
    }

    /// Loads file (with metadata) from disk... no matter what.
    #[tracing::instrument]
    pub async fn load_from_disk(path: &Path) -> Result<Self, RavesError> {
        // make sure the file exists
        let path_str = path.display().to_string();
        if !path.exists() {
            tracing::error!("the file doesn't exist");
            return Err(RavesError::MediaDoesntExist {
                path: path_str.clone(),
            });
        }

        // grab some file metadata from disk
        tracing::debug!("grabbing file metadata...");
        let file_metadata = path
            .metadata()
            .map_err(|err| RavesError::FileMetadataFailure {
                path: path_str.clone(),
                err,
            })?;

        // check the file's properties
        tracing::debug!("checking file properties...");
        let m = block_in_place(|| {
            rexiv2::Metadata::new_from_path(path).map_err(|_e| RavesError::MediaDoesntExist {
                path: path_str.clone(),
            })
        })?;

        // TODO: figure out how to store user tags

        // mime type
        tracing::debug!("checking mime type for file...");
        let mime = infer::get_from_path(path)
            .map_err(|_e| RavesError::MediaDoesntExist {
                path: path_str.clone(),
            })?
            .ok_or(RavesError::NoMimeData {
                path: path_str.clone(),
            })?;

        // aaaand make the format
        tracing::debug!("creating mime type for media file...");
        let format =
            Format::new_from_mime(mime.mime_type()).ok_or(RavesError::FileNotSupportedMedia {
                path: path_str.clone(),
            })?;

        // match on the type we got
        let specific = match format.media_kind() {
            MediaKind::Photo | MediaKind::AnimatedPhoto => SpecificMetadata::Image {},
            MediaKind::Video => {
                // let's ask ffmpeg what it thinks
                tracing::trace!("video detected. asking ffmpeg to handle...");
                ffmpeg_next::init()?;
                let t = ffmpeg_next::format::input(path).map_err(|e| {
                    RavesError::FfmpegFailedProcessing {
                        path: path_str.clone(),
                        err: e.to_string(),
                    }
                })?;

                // grab the first video stream and see how long it is
                let video_length = t
                    .streams()
                    .find(|s| s.parameters().medium() == media::Type::Video)
                    .map(|s| (Rational::new(s.duration() as i32, 1)) * s.time_base())
                    .map(|s| s.0 as f64 / s.1 as f64)
                    .unwrap_or(0_f64);
                tracing::trace!("video len is {video_length}.");

                SpecificMetadata::Video {
                    length: video_length,
                }
            }
        };

        let meta = Metadata {
            path: path.to_path_buf(),
            resolution: Resolution {
                width: m.get_pixel_width().try_into().unwrap_or(0),
                height: m.get_pixel_height().try_into().unwrap_or(0),
            },
            filesize: Filesize(file_metadata.len()),
            format,
            creation_date: file_metadata.created().ok(),
            modified_date: file_metadata.modified().ok(),
            first_seen_date: SystemTime::now(),
            specific,
            other: None, // TODO
        };

        // ok ok... we have everything else. let's save it now!
        tracing::debug!("saving media to database...");
        let db = RavesDb::connect().await?;
        let v: Vec<Media> = db
            .media_info
            .insert("info")
            .content(Self {
                metadata: meta,
                tags: Vec::new(), // TODO
            })
            .await
            .map_err(|e| DatabaseError::InsertionFailed(e.to_string()))?;

        let constructed = v
            .first()
            .ok_or(DatabaseError::InsertionFailed(
                "didn't get anything from return vec! :p".into(),
            ))?
            .clone();

        Ok(constructed)
    }
}
