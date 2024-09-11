//! Represents tags in all their glory.

use surrealdb::sql::Thing;
use uuid::Uuid;

pub type TagIdent = Uuid;

/// A "section" for tags. When a tag has a section, it is separated from others
/// by extreme differences.
///
/// For example, it might make absolutely zero sense to sort a vacation and
/// anime artwork using the same tags.
///
/// Instead, separate them with sections! "Beautiful" will have a very different
/// meaning to any vacation-loving neckbeard. 🤓🫵
#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub struct TagSection {
    name: String,
}

impl Default for TagSection {
    /// Creates THE default `TagSection`, simply titled "default".
    fn default() -> Self {
        Self {
            name: String::from("default"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub struct Tag {
    /// A unique name describing this tag.
    ///
    /// Don't use this to find the tag - EVER.
    /// The name can change, but a tag's UUID is forever static.
    name: String,
    /// A unique identifier.
    ///
    /// Always use this when referencing the tag externally.
    uuid: TagIdent,
    /// The section this tag belongs to.
    tag_section: Option<TagSection>,
    /// The other tags this tag "implies". For example, tags "christmas" and
    /// "halloween" would both imply the "holiday" tag.
    implies: Vec<TagIdent>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Deserialize)]
pub struct TagRecord {
    pub tag: Tag,
    pub id: Thing,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Deserialize)]
pub struct TagSectionRecord {
    pub section: TagSection,
    pub id: Thing,
}
