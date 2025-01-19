use super::details::{DateDetail, FormatDetail, KindDetail, TagDetail};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum DateTimeModifier {
    Before(DateDetail),
    During(DateDetail),
    After(DateDetail),
}

/// A collection modifier directly queries a media based on its metadata.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum CollectionModifier {
    Tag(TagDetail),
    Album(String),
    Literal(String),
    DateTime(DateTimeModifier),
    Format(FormatDetail),
    Kind(KindDetail),
    Orientation(OrientationDetail),
}

/// A modifier that applies `OR`/`NOT`` logic to modifier expressions.
///
/// Note that `AND` is implied by the search itself. It isn't provided here
/// for that reason.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum BooleanModifier {
    Not(Box<Expr>),
    Any(Vec<Expr>),
    // Related(Box<Expr>), // TODO: implement this! it's cool
}

/// Miscellaneous modifiers that react to simple media properties.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum OtherModifier {
    Favorite,
    Untagged,
    Undated,
}

/// A specific piece of a search that evaluates to a single boolean value, true
/// or false.
///
/// An expression is queried by evaluating all the modifiers it owns.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Expr {
    Collection(CollectionModifier),
    Boolean(BooleanModifier),
    Other(OtherModifier),
}

pub struct PreExecutionQuery {
    pub query: String,
    // pub parameters: Vec<Value>, // FIXME: no clue what i was cookin here. `surrealql::Value`..?
}

// /// A modifier must become a query to be used.
// ///
// /// All modifiers must implement this trait!
// pub trait ToQuery {
//     /// Converts the modifier into a query for use in querying the database.
//     ///
//     /// This assumes that each modifier can become a query string.
//     fn to_query(&self) -> Result<Query, surrealdb::error::Db>;
// }
