use extension::sqlite::SqliteExpr;
use sea_query::*;

use crate::{models::media::metadata::MediaKind, search::details::Comparison};

use super::{
    details::{DateDetail, FormatDetail, OrientationDetail, TagDetail},
    modifiers::{CollectionModifier, DateTimeModifier, ToQuery},
};

/// the media table
#[derive(Iden)]
pub enum Info {
    Table,
    Id,
    Path,
    Album,
    Filesize,
    Format,
    CreationDate,
    ModificationDate,
    FirstSeenDate,
    WidthPx,
    HeightPx,
    SpecificMetadata,
    OtherMetadata,
    Tags,
}

/// The default array length function in SQLite.
struct JsonArrayLenFunction;

impl Iden for JsonArrayLenFunction {
    fn unquoted(&self, s: &mut dyn core::fmt::Write) {
        write!(s, "json_array_length").unwrap()
    }
}

impl ToQuery for CollectionModifier {
    #[tracing::instrument]
    fn to_query(self) -> SimpleExpr {
        match self {
            // based on the kind, we'll check various attributes of the media's tags.
            //
            // use `SqliteExpr::col(ColType::Variant).get_json_field("name")`
            CollectionModifier::Tag(tag_detail) => match tag_detail {
                TagDetail::TagUuid(uuid) => {
                    tracing::debug!("Looking for tag with UUID `{uuid}`");
                    Expr::col(Info::Id).eq(uuid)
                }
                TagDetail::PersonTagUuid(uuid) => {
                    tracing::warn!("LPerson tag with UUID is unimplemented."); // TODO
                    Expr::col(Info::Id).eq(uuid)
                }
                TagDetail::PersonTagWithMarker(uuid, _marker_uuid) => {
                    tracing::warn!("Person tag with marker is unimplemented. (uuid: {uuid}, marker uuid: {_marker_uuid})"); // TODO
                    Expr::col(Info::Id).eq(uuid)
                }
                TagDetail::Count(ct, cmp) => {
                    tracing::debug!("Looking for media with {cmp} {ct} tags!");

                    let fn_call = SimpleExpr::FunctionCall(
                        Func::cust(JsonArrayLenFunction).arg(Expr::col(Info::Tags)),
                    );

                    match cmp {
                        Comparison::Less => fn_call.lt(ct),
                        Comparison::LessOrEqual => fn_call.lte(ct),
                        Comparison::Equal => fn_call.eq(ct),
                        Comparison::GreaterOrEqual => fn_call.gte(ct),
                        Comparison::Greater => fn_call.gt(ct),
                    }
                }
            },

            // based on containing folder!
            CollectionModifier::Album(path) => {
                tracing::debug!("Checking for media file with album name: `{path}`...");
                Expr::col(Info::Album).like(path)
            }

            // ez pz, just add a 'LIKE' clause with `.like(<lit>)`
            CollectionModifier::Literal(lit) => {
                tracing::debug!("Checking for literal: `{lit}`");
                Expr::col(Info::Path).like(format!("%{lit}%"))
            }

            // yeah that's not bad. might be difficult to express dates in the
            // orm-ish syntax, though?
            CollectionModifier::DateTime(dt_modifier) => {
                let get_col_from_detail = |dd: DateDetail| {
                    tracing::debug!("Given date detail: {dd:?}");
                    match dd {
                        DateDetail::Created(date_time) => {
                            (Expr::col(Info::CreationDate), date_time)
                        }
                        DateDetail::Modified(date_time) => {
                            (Expr::col(Info::ModificationDate), date_time)
                        }
                        DateDetail::FirstSeen(date_time) => {
                            (Expr::col(Info::FirstSeenDate), date_time)
                        }
                    }
                };

                match dt_modifier {
                    DateTimeModifier::Before(dd) => {
                        let (col, time) = get_col_from_detail(dd);
                        col.lt(Value::ChronoDateTimeUtc(Some(Box::new(time))))
                    }

                    // TODO: DateTimeModifier::Between ...
                    //
                    DateTimeModifier::After(dd) => {
                        let (col, time) = get_col_from_detail(dd);
                        col.gt(Value::ChronoDateTimeUtc(Some(Box::new(time))))
                    }
                }
            }

            CollectionModifier::Format(format_detail) => {
                tracing::debug!("Asked to check for Format.");

                match format_detail {
                    FormatDetail::MimeType(mime_type) => {
                        tracing::debug!("Looking at format's MIME type. given: `{mime_type}`");

                        Expr::col(Info::Format)
                            .cast_json_field("mime_type")
                            .like(mime_type)
                    }

                    FormatDetail::Extension(file_ext) => {
                        tracing::debug!("Checking format's extension. given: `{file_ext}`");

                        // ensure correct formatting of extension. note that `LIKE` is
                        // case-insensitive :)
                        let file_ext = {
                            let mut s = String::with_capacity(file_ext.len() + 1);

                            // IMPORTANT! this does the 'end of string' checking in SQLite
                            s.push('%');

                            // add the other part to the end, in lowercase + no whitespace
                            s.push_str(file_ext.trim());
                            s
                        };

                        tracing::debug!("Made formatted extension: `{file_ext}`");
                        Expr::col(Info::Path).like(file_ext)
                    }
                }
            }

            CollectionModifier::Kind(kind_detail) => {
                tracing::debug!("Asked to check by kind: `{kind_detail:?}`");

                // we'll use json for this
                Expr::col(Info::Format)
                    .cast_json_field("media_kind")
                    .eq(MediaKind::from(kind_detail.clone()).to_string())
            }

            // hoz: width_px > height_px
            // vert: height_px > width_px
            // square: width_px = height_px
            CollectionModifier::Orientation(orientation_detail) => match orientation_detail {
                OrientationDetail::Landscape => {
                    tracing::debug!("Orientation detail (landscape)...");
                    Expr::col(Info::WidthPx).gt(Expr::col(Info::HeightPx))
                }
                OrientationDetail::Portrait => {
                    tracing::debug!("Orientation detail (portrait)...");
                    Expr::col(Info::HeightPx).gt(Expr::col(Info::WidthPx))
                }
                OrientationDetail::Square => {
                    tracing::debug!("Orientation detail (square)...");
                    Expr::col(Info::WidthPx).eq(Expr::col(Info::HeightPx))
                }
            },
        }
    }
}
