//! Represents tags in all their glory.

use uuid::Uuid;

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
    id: Uuid,
}

impl Default for TagSection {
    /// Creates THE default `TagSection`, simply titled "default".
    fn default() -> Self {
        Self {
            name: String::from("Default"),
            id: Uuid::nil(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub struct Tag {
    /// A unique name describing this tag.
    ///
    /// Don't use this to find the tag - EVER.
    /// The name can change, but a tag's UUID is forever static.
    pub name: String,
    /// A unique identifier.
    ///
    /// Always use this when referencing the tag externally.
    pub uuid: Uuid,
    /// The section this tag belongs to.
    pub tag_section: Option<Uuid>,
    /// The other tags this tag "implies". For example, tags "christmas" and
    /// "halloween" would both imply the "holiday" tag.
    pub implies: Vec<Uuid>,
}

impl Tag {
    /// Creates a new tag **representation** for testing.
    ///
    /// It will not be stored in the database or anything like that.
    #[doc(hidden)]
    pub fn new_testing(name: impl AsRef<str>) -> Self {
        Self {
            name: name.as_ref().to_string(),
            uuid: Uuid::new_v4(),
            tag_section: Some(Uuid::nil()),
            implies: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Deserialize)]
pub struct TagRecord {
    pub tag: Tag,
    pub id: Uuid,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Deserialize)]
pub struct TagSectionRecord {
    pub section: TagSection,
    pub id: Uuid,
}
