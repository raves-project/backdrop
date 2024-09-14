use async_watcher::error;
use core::error::Error;
use pisserror::Error;

use crate::config::Config;

/// Stick this at the end of bug warnings/errors.
///
/// It helps users find out where to report bugs when looking at logs.
pub async fn bug_msg() -> String {
    format!(
        "this is a bug, so please report it! you can do so by heading to this git repo: {}",
        Config::read().await.bug_report_info.repo
    )
}

#[derive(Debug, Error)]
pub enum RavesError {
    #[error("The database has encountered an error. See: `{_0}`")]
    DatabaseError(#[from] DatabaseError),

    #[error("The media file at `{path}` was expected to exist, but didn't.")]
    MediaDoesntExist { path: String },

    #[error("The media file at `{path}` does not appear to contain MIME (file type) data.")]
    NoMimeData { path: String },

    #[error("The media file at `{path}` is not a supported media file.")]
    FileNotSupportedMedia { path: String },

    // metadata
    #[error("The media file at `{_0}` was missing required metadata: {_1}")]
    FileMissingMetadata(String, String),

    #[error("(KAMADAK) An error occured when parsing metadata for file at `{_0}`. See: `{_1}`.")]
    KamadakExifError(String, kamadak_exif::Error),

    #[error("(NOM) An error occured when parsing metadata for file at `{_0}`. See: `{_1}`.")]
    NomExifError(String, nom_exif::Error),

    #[error("An error occured when reading the image at `{_0}`. See: `{_1}`.")]
    ImageError(String, image::ImageError),

    #[error("An error occured when reading the MP4 video at `{_0}`. See: `{_1}`.")]
    Mp4parseError(String, mp4parse::Error),

    #[error("An error occured when parsing the Matroska video at `{_0}`. See: `{_1}`.")]
    MatroskaError(String, matroska::Error),

    #[error("Failed to get file metadata for the media file at `{path}`. Err: `{err}`.")]
    FileMetadataFailure { path: String, err: std::io::Error },

    #[error("An error occurred when processing media thumbnail data. See: `{_0}`")]
    MediaThumbnail(#[from] ThumbnailError),

    #[error("A `tokio` task unexpectedly panicked. See: `{_0}`")]
    TokioJoinError(#[from] tokio::task::JoinError),

    #[error("FFmpeg failed to process the given file at `{path}`. See: `{err}`")]
    FfmpegFailedProcessing { path: String, err: String },

    #[error("FFmpeg failed to initialize. See: `{_0}`.")]
    FfmpegCantInit(#[from] ffmpeg_next::Error),

    #[error("FFmpeg doesn't have the required filter functionality.")]
    FfmpegMissingFilterFunctionality,

    #[error("Failed to find selected FFmpeg filter.")]
    FfmpegNoSelectedFilter,

    #[error("FFmpeg didn't find a good video stream for this video file. Path: `{_0}`.")]
    FfmpegNoGoodVideoStreams(String),
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("General database error. See: {_0}")]
    GeneralDatabaseError(#[from] surrealdb::Error),

    #[error("Failed to connect to the database. See: {_0}")]
    ConnectionError(String),

    #[error("Couldn't continue with database insertion. See: {_0}")]
    InsertionFailed(String),

    #[error("Failed to complete database query. See: {_0}")]
    QueryFailed(surrealdb::Error),

    #[error("Empty response when attempting to query database. Path: `{_0}`")]
    EmptyResponse(String),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    /// during fs read from disk
    #[error("Failed to read config file. See: `{_0}`")]
    ReadFailed(#[from] tokio::io::Error),

    /// parsing
    #[error("Failed to parse config file. See: `{_0}`")]
    ParseFailed(#[from] toml::de::Error),

    /// when we read from disk, the paths should be equal
    #[error("The config file had Android file paths that didn't match the newest ones.")]
    PathMismatch,
}

#[derive(Debug, Error)]
pub enum ThumbnailError {
    #[error("Info database did not contain the expected media file metadata. Path: `{_0}`")]
    MediaNotFound(String),

    #[error("The `image` crate failed to parse this media file into an image. Err: `{_0}`, path: `{_1}`.")]
    ImageParsingFailed(image::ImageError, String),

    #[error(
        "Thumbnail creation succeeded, but writing to disk failed. Err: `{_0}`, path: `{_1}`."
    )]
    ThumbnailSaveFailure(std::io::Error, String),

    #[error("Failed to create the thumbnail output file. Err: `{_0}`, path: `{_1}`.")]
    FileCreationFailed(std::io::Error, String),

    #[error("FFmpeg never found a good thumbnail for the video at path `{_0}`.")]
    FfmpegNoSelectedFilter(String),
}
