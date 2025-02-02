//! A detail is something that will be searched on.
//!
//! For example, in a search for "filekind:video", "video" is the detail.

use std::path::PathBuf;

use crate::models::media::metadata::Framerate;

use jiff::Zoned;

/// the location of media
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct PathDetail(pub PathBuf);

/// date, time, created dt, modified dt, accessed dt, first seen dt
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum DateDetail {
    // TODO: allow dates, times, or both. for now, assume manual conversion
    Created(Zoned),
    Modified(Zoned),
    Accessed(Zoned),
    FirstSeen(Zoned),
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
    Image,
    Video,
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
    TagName(String),
    PersonTagName(String),
    PersonTagWithMarker(String, String),
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
