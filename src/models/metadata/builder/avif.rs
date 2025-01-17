use avif_parse::read_avif;
use camino::Utf8Path;
use sqlx::types::Json;

use crate::{
    error::RavesError,
    models::metadata::{
        types::{Format, MediaKind},
        SpecificMetadata,
    },
};

use super::MediaBuilder;

impl MediaBuilder {
    /// Applies EXIF data from `image` to `self`.
    #[tracing::instrument(skip(self, path))]
    pub(super) async fn apply_avif(
        &mut self,
        path: impl AsRef<Utf8Path>,
        format: Format,
    ) -> Result<(), RavesError> {
        // cast path
        let path = path.as_ref();

        // apply format
        let media_kind = format.media_kind();
        self.format = Some(Json(format));

        // grab data from avif.
        //
        // note: this spawns a blocking task, which tokio is chill with.
        // i hold hope for a newfangled async api
        let avif_path = path.to_path_buf();
        let avif_data = tokio::task::spawn_blocking(move || parse_avif(&avif_path)).await??;
        let useful_metadata = avif_data.primary_item_metadata()?;

        // resolution
        self.width_px = Some(useful_metadata.max_frame_width.get());
        self.height_px = Some(useful_metadata.max_frame_height.get());
        tracing::debug!("got resolution from `exif-parse`!");

        // specific
        self.specific_metadata = Some(Json(match media_kind {
            MediaKind::Photo => SpecificMetadata::Image {},
            MediaKind::Video => panic!("gave avif parser video data!"),
            MediaKind::AnimatedPhoto => unimplemented!(),
        }));

        Ok(())
    }
}

/// Attempts to parse the given file as AVIF.
fn parse_avif(path: &Utf8Path) -> Result<avif_parse::AvifData, RavesError> {
    let mut file = std::fs::File::open(path)
        .inspect_err(|e| tracing::warn!("Failed to open AVIF file for `avif-parse`. err: {e}"))
        .map_err(|_| RavesError::MediaDoesntExist {
            path: path.to_string(),
        })?;

    Ok(read_avif(&mut file)
        .inspect_err(|e| tracing::warn!("`avif-parse` failed to read the given file. err: {e}"))?)
}
