//! Handles thumbnails for media.
//!
//! This includes generating thumbnails for media (and caching them), alongside
//! grabbing thumbnails from media created by a camera or device.

use camino::Utf8PathBuf;
use ffmpeg_next::{
    codec::context::Context,
    filter::{self, Graph},
    format::input,
    frame,
};
use image::imageops::FilterType;
use std::io::BufWriter;
use uuid::Uuid;

use ffmpeg_next::util::frame::video::Video;

use crate::{
    config::Config,
    database::DATABASE,
    error::{RavesError, ThumbnailError},
    models::media::{metadata::SpecificMetadata, Media},
};

#[derive(
    Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize, sqlx::FromRow,
)]
pub struct Thumbnail {
    /// a UNIQUE path to the thumbnail file.
    path: String,

    /// the id of the original media file in the database.
    media_id: Uuid,
}

impl Thumbnail {
    const SIZE: u32 = 512;

    /// Creates a new thumbnail representation given an image ID.
    pub async fn new(media_id: &Uuid) -> Self {
        // note: the path to a thumbnail is static from its id.
        let path = Self::make_path(media_id).await;
        Self {
            path: path.to_string(),
            media_id: *media_id,
        }
    }

    /// Makes a real thumbnail file for this representation. It'll be saved to disk.
    pub async fn create(&self) -> Result<(), RavesError> {
        // avoid recreating thumbnails
        if self.path().exists() {
            tracing::trace!("attempted to create thumbnail, but it already exists");
            return Ok(());
        }

        // welp. we're starting from scratch.
        // let's start by grabbing the media metadata from the db
        let media_ext = self.get_media().await?;
        let media = media_ext.clone();

        // ok we have the media. let's use it
        let thumbnail_buffer = match media.specific_metadata.0.clone() {
            SpecificMetadata::Image { .. } => {
                // let's read it into a buffer
                tokio::fs::read(&media.path)
                    .await
                    .map_err(|_e| RavesError::MediaDoesntExist {
                        path: media.path.clone(),
                    })?
            }

            SpecificMetadata::Video { .. } => {
                // let's use ffmpeg to grab a decent-looking frame
                // here, we're looking for parts of the video where things kinda change and aren't so static
                tokio::task::spawn_blocking(move || -> Result<Vec<u8>, RavesError> {
                    ffmpeg_next::init()?;

                    // let's start out by finding that change
                    let mut input = input(&media.path)?;
                    let input_stream = input
                        .streams()
                        .best(ffmpeg_next::media::Type::Video)
                        .ok_or(RavesError::FfmpegNoGoodVideoStreams(media.path.clone()))?;
                    let codec = Context::from_parameters(input_stream.parameters().to_owned())?;
                    let mut decoder = codec.decoder().video()?;
                    let input_stream_index = input_stream.index();

                    let mut filter = Graph::new();
                    filter.add(
                        &ffmpeg_next::filter::find("select")
                            .ok_or(RavesError::FfmpegMissingFilterFunctionality)?,
                        "select",
                        r#"gt(scene\,0.4)"#,
                    )?;
                    let args = format!(
                        "video_size={}x{}:pix_fmt={}:time_base={}:pixel_aspect={}",
                        decoder.width(),
                        decoder.height(),
                        input.format().name(),
                        decoder.time_base(),
                        decoder.aspect_ratio()
                    );

                    filter.add(&filter::find("buffer").unwrap(), "in", &args)?;
                    filter.add(
                        &filter::find("select").unwrap(),
                        "select",
                        "gt(scene\\,0.4)",
                    )?;
                    filter.add(&filter::find("buffersink").unwrap(), "out", "")?;
                    filter.output("out", 0)?;
                    filter.input("in", 0)?;
                    filter.validate()?;

                    let mut scene_frame: Option<Video> = None;

                    let mut packets = input.packets();

                    for (stream, packet) in &mut packets {
                        if stream.index() == input_stream_index {
                            decoder.send_packet(&packet)?;
                            let mut video_frame = frame::Video::empty();

                            while let Ok(()) = decoder.receive_frame(&mut video_frame) {
                                filter
                                    .get("in")
                                    .ok_or(RavesError::FfmpegNoSelectedFilter)?
                                    .source()
                                    .add(&video_frame)?;

                                let mut filtered_frame = frame::Video::empty();
                                while let Ok(()) = filter
                                    .get("out")
                                    .ok_or(RavesError::FfmpegNoSelectedFilter)?
                                    .sink()
                                    .frame(&mut filtered_frame)
                                {
                                    if scene_frame.is_none() {
                                        scene_frame = Some(filtered_frame.clone());
                                    }
                                }
                            }
                        }
                    }

                    // we should have a scene frame now. let's modify and save!
                    Ok(scene_frame
                        .ok_or(ThumbnailError::FfmpegNoSelectedFilter(media.path.clone()))?
                        .data(0)
                        .to_vec())
                })
                .await??
            }

            SpecificMetadata::AnimatedImage { .. } => unimplemented!(),
        };

        // ok. let's use that buffer now
        self.save_from_buffer(&thumbnail_buffer, &media_ext).await?;

        // all done! let's brag
        tracing::trace!(
            "successfully generated thumbnail for media file at `{}`!",
            media_ext.path.clone()
        );

        todo!()
    }

    /// Grabs the path to the thumbnail.
    pub fn path(&self) -> Utf8PathBuf {
        Utf8PathBuf::from(self.path.clone())
    }

    /// Represents this thumbnail's path as a string.
    pub fn path_str(&self) -> String {
        self.path().to_string()
    }

    /// Grabs the ID of the original media file.
    pub fn media_id(&self) -> &Uuid {
        &self.media_id
    }

    pub async fn save_from_buffer(&self, buf: &[u8], media: &Media) -> Result<(), RavesError> {
        let thumbnail = {
            let img = image::load_from_memory(buf)
                .map_err(|e| ThumbnailError::ImageParsingFailed(e, media.path.clone()))?;

            img.resize_to_fill(Self::SIZE, Self::SIZE, FilterType::Nearest)
        };

        let file = std::fs::File::create(self.path())
            .map_err(|e| ThumbnailError::ThumbnailSaveFailure(e, self.path.clone()))?;
        let mut writer = BufWriter::new(file);

        let path_str = self.path.clone();

        // let's save it as blessed avif
        tokio::task::spawn_blocking(move || -> Result<(), ThumbnailError> {
            thumbnail
                .write_to(&mut writer, image::ImageFormat::Avif)
                .map_err(|e| ThumbnailError::ImageParsingFailed(e, path_str))
        })
        .await??;

        Ok(())
    }
}

impl Thumbnail {
    /// Makes a unique thumbnail path from an media file's unique ID.
    async fn make_path(media_id: &Uuid) -> Utf8PathBuf {
        let filename = Utf8PathBuf::from(format!("{media_id}.thumbnail"));
        Config::read()
            .await
            .cache_dir
            .clone()
            .join("thumbnails")
            .join(filename)
    }

    /// Returns the media file representation that this thumbnail is for.
    async fn get_media(&self) -> Result<Media, RavesError> {
        let mut conn = DATABASE.acquire().await?;

        let media = sqlx::query_as::<_, Media>("SELECT * FROM info WHERE id = $1")
            .bind(self.media_id)
            .fetch_one(&mut *conn)
            .await?;

        Ok(media)
    }
}

impl Media {
    /// Returns the thumbnail from the database for this media file.
    #[tracing::instrument]
    pub async fn get_thumbnail(&self, _id: &Uuid) -> Result<Thumbnail, RavesError> {
        // see if we have a thumbnail in the database
        if let Some(thumbnail) = self.database_get_thumbnail().await? {
            return Ok(thumbnail);
        }

        // the file doesn't have one either! let's make one ;D
        let thumbnail = Thumbnail::new(&self.id).await;
        thumbnail.create().await?; // this makes the file
        Ok(thumbnail)
    }

    /// Tries to grab the thumbnail from the database, if it's there.
    #[tracing::instrument]
    async fn database_get_thumbnail(&self) -> Result<Option<Thumbnail>, RavesError> {
        let mut conn = DATABASE.acquire().await?;

        let thumbnail =
            sqlx::query_as::<_, Thumbnail>("SELECT * FROM thumbnail WHERE media_id = $1")
                .bind(self.id.to_string())
                .fetch_one(&mut *conn)
                .await?;

        Ok(Some(thumbnail))
    }
}
