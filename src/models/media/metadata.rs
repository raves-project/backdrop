use std::{cmp::Ordering, collections::HashMap};

use fraction::GenericFraction;

/// Metadata "specific" to one type of media.
#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub enum SpecificMetadata {
    #[non_exhaustive]
    Image {},

    #[non_exhaustive]
    AnimatedImage {
        frame_count: u32,
        framerate: Framerate,
    },

    #[non_exhaustive]
    Video {
        /// The video's length in seconds.
        length: f64,
        // TODO: framerate (see below)
        // framerate: Framerate,
    },
}

#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct OtherMetadataValue {
    // note: this is on the value since putting it on the key makes it difficult
    // to actually use in the map lol
    //
    // TODO: maybe just do this on the frontend manually?
    pub user_facing_name: Option<String>,
    pub value: String,
}

impl OtherMetadataValue {
    pub fn new(name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        Self {
            user_facing_name: Some(name.as_ref().to_string()),
            value: value.as_ref().to_string(),
        }
    }
}

/// A representation for uncommon metadata that can only be read.
///
/// Also, it's a `HashMap` newtype to get around the lack of `PartialOrd`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OtherMetadataMap(pub HashMap<String, OtherMetadataValue>);

impl OtherMetadataMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl Default for OtherMetadataMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialOrd for OtherMetadataMap {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.len().partial_cmp(&other.0.len())
    }
}

/// Resolution, currently capped at 65,535 x 65,535.
///
/// Internally uses `u16` values.
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, serde::Serialize, serde::Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    /// Creates a new resolution.
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// A simple enum over the supported types of media.
#[derive(
    Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum MediaKind {
    Photo,
    AnimatedPhoto,
    Video,
}

impl std::fmt::Display for MediaKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaKind::Photo => write!(f, "Photo"),
            MediaKind::AnimatedPhoto => write!(f, "AnimatedPhoto"),
            MediaKind::Video => write!(f, "Video"),
        }
    }
}

/// A representation of a media file's MIME format.
//
// MAINTAINER NOTE: if you change the names of these fields, you also need to
// change the filter/searching modifiers for `Format`!
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, serde::Serialize, serde::Deserialize)]
pub struct Format {
    /// The "kind" of media used in this format. (image, video, animated image, etc..?)
    media_kind: MediaKind,
    /// The specific MIME format used, like "webm" or "avif".
    mime_type: String,
}

impl Format {
    /// Tries to create a new [`Format`] from just a MIME type.
    ///
    /// If the MIME type uses an unsupported media [`MediaKind`], this will
    /// return `None`.
    #[tracing::instrument(skip_all)]
    pub fn new_from_mime<S: AsRef<str>>(mime: S) -> Option<Self> {
        let mime = mime.as_ref();
        tracing::debug!("creating format from mime type `{mime}`...");

        let mut s = mime.split('/');
        let raw_kind = s.next()?;

        // TODO: annoying parsing for animated media.
        // maybe find a library for that...
        let kind = match raw_kind {
            "image" => MediaKind::Photo,
            "video" => MediaKind::Video,
            _ => return None,
        };

        tracing::debug!("got media kind `{kind}` from mime type `{mime}`!");

        Some(Self {
            media_kind: kind,
            mime_type: mime.into(),
        })
    }

    pub fn media_kind(&self) -> MediaKind {
        self.media_kind
    }

    pub fn mime_type(&self) -> String {
        self.mime_type.clone()
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.media_kind {
            MediaKind::Photo | MediaKind::AnimatedPhoto => "photo",
            MediaKind::Video => "video",
        };

        f.write_str(format!("{kind}/{}", self.mime_type).as_str())
    }
}

/// A video's framerate, represented as a fraction.
pub type Framerate = fraction::Fraction;

/// Representation of a file's size in bytes.
///
/// May internally expect IEC units like kibibytes (1024 bytes).
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, serde::Serialize, serde::Deserialize)]
pub struct Filesize(pub u64);

impl From<u64> for Filesize {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

/// Representation of a video's bitrate in kibibytes per second.
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, serde::Serialize, serde::Deserialize)]
pub struct Bitrate(pub u32);

/// The aspect ratio of a media file.
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, serde::Serialize, serde::Deserialize)]
pub struct AspectRatio {
    /// This fraction's numerator is the width, denominator is height.
    ///
    /// e.g. (16/9) is 16 wide, 9 tall.
    frac: Option<GenericFraction<u32>>,
}

impl From<Resolution> for AspectRatio {
    fn from(value: Resolution) -> Self {
        AspectRatio::new(value.width, value.height) // these automatically simplify
    }
}

/// The orientation of a media's viewport.
///
/// Note that this can come from both Resolution and AspectRatio.
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, serde::Serialize, serde::Deserialize)]
pub enum Orientation {
    Portrait,
    Landscape,
    Square,
}

impl From<Resolution> for Orientation {
    fn from(value: Resolution) -> Self {
        match value.width.cmp(&value.height) {
            Ordering::Less => Orientation::Portrait, // width is less than height. tall
            Ordering::Equal => Orientation::Square,  // square
            Ordering::Greater => Orientation::Landscape, // width > height. wide
        }
    }
}

impl From<AspectRatio> for Orientation {
    fn from(value: AspectRatio) -> Self {
        match value.width().cmp(&value.height()) {
            Ordering::Less => Orientation::Portrait, // width is less than height. tall
            Ordering::Equal => Orientation::Square,  // square
            Ordering::Greater => Orientation::Landscape, // width > height. wide
        }
    }
}

impl AspectRatio {
    /// Creates a new aspect ratio.
    ///
    /// Note that passing either number as zero will result in an expected
    /// 0:0 output.
    pub fn new(width: u32, height: u32) -> Self {
        let frac = if width == 0 || height == 0 {
            None
        } else {
            Some(GenericFraction::new::<u32, u32>(width, height))
        };

        Self { frac }
    }

    /// Grabs the width (e.g. "16" in 16:9).
    pub fn width(&self) -> u32 {
        if let Some(frac) = self.frac {
            if let Some(numer) = frac.numer() {
                return *numer;
            }
        }

        0_u32
    }

    /// Grabs the height (e.g. "9" in 16:9).
    pub fn height(&self) -> u32 {
        if let Some(frac) = self.frac {
            if let Some(denom) = frac.denom() {
                return *denom;
            }
        }

        0_u32
    }
}

#[cfg(test)]
mod tests {
    use super::AspectRatio;

    #[test]
    fn aspect_ratios_16_9() {
        let ratio = AspectRatio::new(16, 9);
        assert_eq!(16, ratio.width());
        assert_eq!(9, ratio.height());
    }

    #[test]
    fn zero_aspect_ratios_should_be_zero() {
        // zero height
        let ratio = AspectRatio::new(4, 0);
        assert_eq!(0, ratio.width());
        assert_eq!(0, ratio.height());

        // zero width
        let ratio = AspectRatio::new(0, 9);
        assert_eq!(0, ratio.width());
        assert_eq!(0, ratio.height());

        // all zero, baby
        let ratio = AspectRatio::new(0, 0);
        assert_eq!(0, ratio.width());
        assert_eq!(0, ratio.height());
    }

    #[test]
    fn aspect_ratios_should_simplify() {
        let ratio = AspectRatio::new(32, 18);
        assert_eq!(16, ratio.width());
        assert_eq!(9, ratio.height());
    }
}
