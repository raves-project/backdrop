//! Types that are really the bedrock of the app.

use metadata::Metadata;

pub mod animated_image;
pub mod image;
pub mod metadata;
pub mod tag;
pub mod video;

/// Some kind of media file (image, animated image, video, etc.)
pub struct Media<Meta: Metadata> {
    metadata: Meta,
}
