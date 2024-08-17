//! Types that are really the bedrock of the app.

use metadata::Metadata;

pub mod animated_image;
pub mod image;
pub mod metadata;
pub mod tags;
pub mod video;

/// Some kind of media file (image, animated image, video, etc.)
pub struct Media<M: Metadata> {
    metadata: M,
}

// TODO!!!
// impl Media (i.e. Image, AnimatedImage, Video. not `Media` directly)
// - fn thumbnail() -> Image;
