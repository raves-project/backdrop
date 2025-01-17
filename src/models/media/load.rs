use camino::{Utf8Path, Utf8PathBuf};

use crate::{
    database::{InsertIntoTable, DATABASE, INFO_TABLE},
    error::{DatabaseError, RavesError},
    models::media::{builder::MediaBuilder, hash::MediaHash},
};

use super::Media;

/** Loads a media file.

This function is the internal implementation of the [`Media::load`] function.
It should not be called from any other location.

## Pipeline

This has its own little 'pipeline'. In short:

1. Check that the given path exists on disk.
2. Expand the path to be "canonicalized".
3. Look for an existing database entry with this path.
4. Based on if:
    - Entry exists: hash both and compare.
        - Hash the new file.
        - Use the old entry's hash.
        - Based on their equality:
            - Equal: Return the old media file from cache.
            - Unequal: Load from disk.
    - Entry does not exist: Just load the file from disk.
*/
#[tracing::instrument]
pub(super) async fn load_internal(path: &Utf8Path) -> Result<super::Media, RavesError> {
    // check that its path exists
    ensure_exists(path).await?;

    // canonicalize it
    let path = canonicalize_path(path).await;

    // grab our hash
    let new_hash = MediaHash::hash_file(&path).await?;

    // if there's an old entry, we'll try to reuse it
    if let Some(old_entry) = from_database(&path).await? {
        tracing::trace!("Found an old entry!");
        let old_entry_hash = MediaHash::hash_file(&old_entry.path).await?;

        // when the hashes match, we'll just return the old media!
        if old_entry_hash == new_hash {
            tracing::debug!("Old and new entries had the same hash! Returning early...");
            return Ok(old_entry);
        } else {
            tracing::trace!("Old and new entries differed! Recomputing metadata!");
        }
    } else {
        tracing::trace!("No previous entry was located.");
    }

    // load from disk + save hash to table
    tracing::debug!("Generating metadata from disk...");
    let media = from_disk(&path).await?;

    // save hash to table
    {
        // construct hash structure for db
        let hash = MediaHash {
            media_id: media.id,
            hash: new_hash.as_bytes().into(),
        };

        // insert it
        hash.add_to_table().await?;
    }

    // finally, return the media :)
    Ok(media)
}

/// Checks to ensure that the media file exists.
///
/// ## Errors
///
/// Will error if the function can't actually check if the path exists.
#[tracing::instrument]
async fn ensure_exists(path: &Utf8Path) -> Result<(), RavesError> {
    match tokio::fs::try_exists(path).await {
        Ok(true) => {
            tracing::trace!("The requested media file exists on disk.");
            return Ok(());
        }
        Ok(false) => {
            tracing::warn!("File does not exist on disk!");
            return Err(RavesError::MediaDoesntExist {
                path: path.to_string(),
            });
        }
        Err(e) => {
            tracing::error!("Failed to check if file exists on disk! err: {e}");
            return Err(RavesError::FailedToOpenMediaFile {
                path: path.to_path_buf(),
                error: e,
            });
        }
    }
}

/// Attempts to turn relative paths (`project/my_image.avif`) to absolute ones
/// with no links (like `/home/barrett/projects/my_image.avif`).
///
/// If it fails, it'll just return the original path.
#[tracing::instrument]
async fn canonicalize_path(path: &Utf8Path) -> Utf8PathBuf {
    path.canonicalize_utf8()
        .inspect_err(|e| tracing::warn!("Failed to canon-ize path. err: {e}"))
        .unwrap_or_else(|_| path.to_path_buf())
}

/// Attempts to grab a media file with `path` from the database. This returns
/// an Option, as that might not be around.
///
/// ## Errors
///
/// Might error if the database connection fails.
#[tracing::instrument]
async fn from_database(path: &Utf8Path) -> Result<Option<Media>, RavesError> {
    // grab db connection
    let mut conn = DATABASE
        .acquire()
        .await
        .inspect_err(|e| tracing::error!("Failed to connect to database! err: {e}"))?;

    // query for an entry with matching path
    sqlx::query_as::<_, Media>(&format!(
        "SELECT * FROM {INFO_TABLE} WHERE path = $1 LIMIT 1"
    ))
    .bind(path.to_string())
    .fetch_optional(&mut *conn)
    .await
    .inspect_err(|e| {
        tracing::warn!("Failed to query database for old version of media file! err: {e}")
    })
    .map_err(|e| e.into())
}

/// Grabs metadata from disk using the `MediaBuilder` API. It'll then cache it
/// into the database.
///
/// ## Errors
///
/// Might fail if the media file isn't compatible or the database fails to
/// connect.
#[tracing::instrument]
async fn from_disk(path: &Utf8Path) -> Result<Media, RavesError> {
    // grab the media file metadata
    tracing::trace!("Feeding media file path to MediaBuilder...");
    let media = MediaBuilder::default().apply(path).await?;

    // cache in database
    {
        let mut conn = DATABASE
            .acquire()
            .await
            .inspect_err(|e| tracing::error!("Failed to connect to database! err: {e}"))?;

        media
            .make_insertion_query()
            .execute(&mut *conn)
            .await
            .inspect_err(|e| {
                tracing::warn!("Failed to insert media into database. err: {e}, media: {media:#?}");
            })
            .map_err(|e| DatabaseError::InsertionFailed(e.to_string()))?;
    }

    // return the media
    Ok(media)
}
