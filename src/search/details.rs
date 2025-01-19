//! A detail is something that will be searched on.
//!
//! For example, in a search for "filekind:video", "video" is the detail.

use std::path::PathBuf;

use crate::models::media::metadata::{Framerate, MediaKind};

use chrono::{DateTime, Utc};

/// the location of media
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct PathDetail(pub PathBuf);

/// date, time, created dt, modified dt, accessed dt, first seen dt
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum DateDetail {
    // TODO: allow dates, times, or both. for now, assume manual conversion
    Created(DateTime<Utc>),
    Modified(DateTime<Utc>),
    FirstSeen(DateTime<Utc>),
}

/// "webm", "avif", etc.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum FormatDetail {
    MimeType(String),
    Extension(String),
}

/// "video", "image", etc.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum KindDetail {
    Photo,
    AnimatedPhoto,
    Video,
}

impl From<KindDetail> for MediaKind {
    fn from(value: KindDetail) -> Self {
        match value {
            KindDetail::Photo => Self::Photo,
            KindDetail::AnimatedPhoto => Self::AnimatedPhoto,
            KindDetail::Video => Self::Video,
        }
    }
}

/// fps of a video
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct FramerateDetail(pub Framerate);

/// - how many tags
/// - tagged/untagged
/// - has specific tag
/// - has any Person tag(s)
/// - has Person tag with marker tag
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum TagDetail {
    TagUuid(String),
    PersonTagUuid(String),
    PersonTagWithMarker(String, String),

    /// The number of tags on a media file.
    Count(u8, Comparison),
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Comparison {
    Less,
    LessOrEqual,
    Equal,
    GreaterOrEqual,
    Greater,
}

/// "landscape", "portrait", "square"
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum OrientationDetail {
    Landscape,
    Portrait,
    Square,
}

/// an exact file size in bytes
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct FileSizeDetail(pub u64);

/// compressed or lossless
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum CompressionDetail {
    Lossless, // :D
    Lossy,    // >:P
}
