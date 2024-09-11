use surrealdb::{engine::local::Db, method::Query, sql::Query as SqlQuery};

use super::{
    details::TagDetail,
    modifiers::{CollectionModifier, ToQuery},
};

use surrealdb::sql::parse;

/// FIXME: this is a really basic impl. it's not a security risk or anything, but it's likely
/// bad to use format strings here. that might allow for injection if someone didn't know.

impl ToQuery for CollectionModifier {
    fn to_query(&self) -> Result<SqlQuery, surrealdb::error::Db> {
        match self {
            CollectionModifier::Tag(tag_detail) => match tag_detail {
                TagDetail::TagName(name) | TagDetail::PersonTagName(name) => {
                    // NOTE: there are no "person" tags yet, so these are the same
                    parse(format!("SELECT * FROM info WHERE media.tags CONTAINS '{name}'").as_str())
                }

                TagDetail::PersonTagWithMarker(_, _) => unimplemented!(),

                TagDetail::Count(ct, cmp) => {
                    let cmp = match cmp {
                        super::details::Comparison::Less => "<",
                        super::details::Comparison::LessOrEqual => "<=",
                        super::details::Comparison::Equal => "=",
                        super::details::Comparison::GreaterOrEqual => ">=",
                        super::details::Comparison::Greater => ">",
                    };

                    parse(
                        format!("SELECT * FROM info WHERE array::len(media.tags) {cmp} {ct}")
                            .as_str(),
                    )
                }
            },

            // we'll sort by the folder it's contained in
            CollectionModifier::Album(name) => {
                // FIXME: this isn't correct. placeholder until we parse it manually
                // note that fixing it might require us to directly query here..!
                parse(format!("SELECT * FROM info WHERE path CONTAINS '{name}'").as_str())
            }

            CollectionModifier::Literal(s) => {
                parse(format!("SELECT * FROM info WHERE media.name = '{s}'").as_str())
            } // FIXME: this should only search the filename

            CollectionModifier::DateTime(_) => todo!(),

            CollectionModifier::Format(_) => todo!(),

            CollectionModifier::Kind(_) => todo!(),

            CollectionModifier::Orientation(_) => todo!(),
        }
    }
}

pub trait ToQuery2 {
    /// Takes in an (unexecuted) query and adds additional clauses on it.
    fn to_query(query: Query<'_, Db>) -> Query<'_, Db>;
}
