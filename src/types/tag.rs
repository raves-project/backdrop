//! Represents tags in all their glory.

use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};

pub type TagIdent = Uuid;

/// A "section" for tags. When a tag has a section, it is separated from others
/// by extreme differences.
///
/// For example, it might make absolutely zero sense to sort a vacation and
/// anime artwork using the same tags.
///
/// Instead, separate them with
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TagSection {
    identifer: String,
}

impl Default for TagSection {
    /// Creates THE default `TagSection`, simply titled "default".
    fn default() -> Self {
        Self {
            identifer: String::from("default"),
        }
    }
}

impl TagSection {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    /// A unique name describing this tag. Don't use this to find the tag - EVER.
    /// The name can change, but a tag's UUID is forever static.
    name: String,
    /// A unique identifier. Always use this when referencing the tag externally.
    uuid: TagIdent,
    /// The section this tag belongs to.
    tag_section: Option<TagSection>,
    /// The other tags this tag "implies". For example, tags "christmas" and
    /// "halloween" would both imply the "holiday" tag.
    implies: Vec<TagIdent>,
}
