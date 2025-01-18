use std::os::unix::fs::MetadataExt as _;

use camino::Utf8Path;

use crate::error::RavesError;

use super::MediaBuilder;

impl MediaBuilder {
    /// Adds typical file attributes to `self`.
    #[tracing::instrument(skip(self))]
    pub(super) async fn file(&mut self, path: &Utf8Path) -> Result<(), RavesError> {
        let path_str = path.to_string();

        // err if the file doesn't open
        let metadata = tokio::fs::metadata(path)
            .await
            .inspect_err(|e| tracing::warn!("Failed to open file for metadata. err: {e}"))
            .map_err(|_e| RavesError::MediaDoesntExist { path: path_str })?;
        tracing::debug!("got file metadata!");

        self.filesize = Some(metadata.size() as i64);
        self.creation_date = metadata.created().ok().map(|st| st.into());
        self.modification_date = metadata.modified().ok().map(|st| st.into());
        tracing::debug!("added file metadata to builder!");

        Ok(())
    }
}
