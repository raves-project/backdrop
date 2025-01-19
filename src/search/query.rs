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

            // FIXME: based on album uuid. that part's easy.
            // but how do we choose the table?
            CollectionModifier::Album(_album_uuid) => todo!(),

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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, env::temp_dir};

    use chrono::DateTime;
    use sea_query::{Asterisk, Cond, SqliteQueryBuilder};
    use sea_query_binder::SqlxBinder as _;
    use sqlx::{pool::PoolConnection, Sqlite};
    use uuid::Uuid;

    use crate::{
        database::{self, InsertIntoTable, DATABASE},
        models::{
            media::{
                metadata::{Format, OtherMetadataMap, OtherMetadataValue, SpecificMetadata},
                Media,
            },
            tags::Tag,
        },
        search::{
            details::{
                Comparison, DateDetail, FormatDetail, KindDetail, OrientationDetail, TagDetail,
            },
            modifiers::{CollectionModifier, DateTimeModifier, ToQuery as _},
            query::Info,
        },
    };

    #[tokio::test]
    async fn collection_mod_orientation() {
        let mut conn = setup_db().await;

        // find all the square ones.
        {
            // there should only be one.
            let square_mod = CollectionModifier::Orientation(OrientationDetail::Square);

            // make the actual statement w/ the modifier
            let (select, values) = sea_query::Query::select()
                .column(Asterisk) // jesus christ
                .from(Info::Table)
                .cond_where(Cond::all().add(square_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            // check that it's right
            assert_eq!(
                r#"SELECT * FROM "info" WHERE "width_px" = "height_px""#, select,
                "select statements should match"
            );

            // query dat mf
            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .expect("select didnt err");

            // ensure it's just vade lol
            assert_eq!(res.len(), 1, "vec length");
            let vade_media = res.first().expect("there is a first option");

            assert_eq!(
                vade_media.id,
                Uuid::from_u128(2),
                "vade square media uuid match"
            );
            assert_eq!(
                vade_media.format,
                Format::new_from_mime("image/png").unwrap().into(),
                "vade square media format match"
            );
        }

        // now, query for horizontal orientation. there should be two entries
        {
            let hoz_mod = CollectionModifier::Orientation(OrientationDetail::Landscape);
            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(Cond::all().add(hoz_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .expect("select didnt err");

            assert_eq!(res.len(), 3);
        }

        // finally, there should be one for vertical
        {
            let vert_mod = CollectionModifier::Orientation(OrientationDetail::Portrait);
            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(Cond::all().add(vert_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .expect("select didnt err");

            assert_eq!(res.len(), 1);
            assert!(res.first().unwrap().path.contains("a.jpg"))
        }
    }

    /// Tests the tag collection modifiers.
    #[tokio::test]
    async fn collection_mod_tags() {
        let mut conn: PoolConnection<Sqlite> = setup_db().await;

        // tag count
        {
            let tag_ct_mod = CollectionModifier::Tag(TagDetail::Count(3, Comparison::Equal));

            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(Cond::all().add(tag_ct_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            assert_eq!(
                r#"SELECT * FROM "info" WHERE json_array_length("tags") = ?"#,
                select,
            );
            assert_eq!(
                values.0 .0.first().unwrap(),
                &sea_query::Value::TinyUnsigned(Some(3))
            );

            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .unwrap();

            // there should be exactly one entry
            assert_eq!(res.len(), 1);

            assert!(res
                .first()
                .unwrap()
                .clone()
                .tags
                .0
                .into_iter()
                .any(|tag| &tag.name == "dittodill"));
        }

        // TODO: test other tag detail queries when tables are implemented!
    }

    /// Checks the collection modifier that searches for text.
    ///
    /// Currently, this just checks the path of the file, but it should also
    /// look in comment/description/etc. fields of any attached metadata.
    #[tokio::test]
    async fn collection_mod_literal() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
        let mut conn: PoolConnection<Sqlite> = setup_db().await;

        // try searching for "A.JPG" (in caps)
        let literal_search_mod = CollectionModifier::Literal("a.jpg".into());

        let (select, values) = sea_query::Query::select()
            .column(Asterisk)
            .from(Info::Table)
            .cond_where(Cond::all().add(literal_search_mod.to_query()))
            .build_sqlx(SqliteQueryBuilder);

        assert_eq!(r#"SELECT * FROM "info" WHERE "path" LIKE ?"#, select);
        assert_eq!(
            values.0 .0.first().unwrap(),
            &sea_query::Value::String(Some(Box::new("%a.jpg%".into())))
        );

        let res = sqlx::query_as_with::<_, Media, _>(&select, values)
            .fetch_all(&mut *conn)
            .await
            .unwrap();

        // only one entry
        assert_eq!(res.len(), 1);
    }

    /// Checks the DateTime CollectionModifiers.
    #[tokio::test]
    #[expect(clippy::inconsistent_digit_grouping, reason = "unix time fmting")]
    async fn collection_mod_datetime() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
        let mut conn: PoolConnection<Sqlite> = setup_db().await;

        // we'll try before + created here:
        {
            let before_created_datetime_mod =
                CollectionModifier::DateTime(DateTimeModifier::Before(DateDetail::Created(
                    DateTime::from_timestamp_nanos(1737137976_000_000_000 + 1),
                )));

            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(Cond::all().add(before_created_datetime_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            assert_eq!(r#"SELECT * FROM "info" WHERE "creation_date" < ?"#, select);

            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .unwrap();

            assert_eq!(res.len(), 3);
            assert_eq!(res.first().unwrap().format.0.mime_type(), "image/png");
        }

        // after + modified:
        {
            let after_modified_datetime_mod = CollectionModifier::DateTime(
                DateTimeModifier::After(DateDetail::Modified(DateTime::from_timestamp_nanos(0))),
            );

            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(Cond::all().add(after_modified_datetime_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            assert_eq!(
                r#"SELECT * FROM "info" WHERE "modification_date" > ?"#,
                select
            );

            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .unwrap();

            assert_eq!(res.len(), 4);
        }

        // after + first_seen:
        {
            let after_first_seen_datetime_mod =
                CollectionModifier::DateTime(DateTimeModifier::After(DateDetail::FirstSeen(
                    DateTime::from_timestamp_nanos(1551731451_000_000_000),
                )));

            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(Cond::all().add(after_first_seen_datetime_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            assert_eq!(
                r#"SELECT * FROM "info" WHERE "first_seen_date" > ?"#,
                select
            );

            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .unwrap();

            assert_eq!(res.len(), 3, "not unix epoch (5) and this exact time (1)");
        }

        // before and after at the same time should reduce the query's domain:
        {
            let after_seen = CollectionModifier::DateTime(DateTimeModifier::After(
                DateDetail::FirstSeen(DateTime::from_timestamp_nanos(1551731451_000_000_000)),
            ));

            let before_seen = CollectionModifier::DateTime(DateTimeModifier::Before(
                DateDetail::FirstSeen(DateTime::from_timestamp_nanos(1737126002_000_000_000)),
            ));

            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(
                    Cond::all()
                        .add(after_seen.to_query())
                        .add(before_seen.to_query()),
                )
                .build_sqlx(SqliteQueryBuilder);

            assert_eq!(
                r#"SELECT * FROM "info" WHERE "first_seen_date" > ? AND "first_seen_date" < ?"#,
                select
            );

            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .unwrap();

            assert_eq!(res.len(), 2, "after 5, before 2, after 1. => [3, 4]");

            assert!(
                res.iter().any(|media| media.id == Uuid::from_u128(3)),
                "has 3"
            );
            assert!(
                res.iter().any(|media| media.id == Uuid::from_u128(4)),
                "has 4"
            );
        }
    }

    #[tokio::test]
    async fn collection_mod_format() {
        let mut conn: PoolConnection<Sqlite> = setup_db().await;

        // there should only be one entry w/ ext "mp4":
        {
            let mp4_ext_mod = CollectionModifier::Format(FormatDetail::Extension("mp4".into()));

            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(Cond::all().add(mp4_ext_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            assert_eq!(r#"SELECT * FROM "info" WHERE "path" LIKE ?"#, select);
            assert_eq!(
                values.0 .0.first().unwrap(),
                &sea_query::Value::String(Some(Box::new(String::from("%mp4"))))
            );

            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .unwrap();

            assert_eq!(res.len(), 1, "should only be one mp4 ext");
        }

        // again, one with MIME type "video/mp4":
        {
            let mp4_mime_mod =
                CollectionModifier::Format(FormatDetail::MimeType("Video/mp4".into()));

            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(Cond::all().add(mp4_mime_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            assert_eq!(
                r#"SELECT * FROM "info" WHERE ("format" ->> ?) LIKE ?"#,
                select
            );
            assert_eq!(
                values.0 .0.first().unwrap(),
                &sea_query::Value::String(Some(Box::new(String::from("mime_type"))))
            ); // json_array_length
            assert_eq!(
                values.0 .0.get(1).unwrap(),
                &sea_query::Value::String(Some(Box::new(String::from("Video/mp4"))))
            );

            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .unwrap();

            assert_eq!(res.len(), 1, "should only be one with mp4 mime type");
        }

        // three with `png` extensions:
        {
            let png_ext_mod = CollectionModifier::Format(FormatDetail::Extension("PnG".into()));

            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(Cond::all().add(png_ext_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            assert_eq!(r#"SELECT * FROM "info" WHERE "path" LIKE ?"#, select);
            assert_eq!(
                values.0 .0.first().unwrap(),
                &sea_query::Value::String(Some(Box::new(String::from("%PnG"))))
            );
            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .unwrap();

            // note: we take it from the file extension, not the MIME here!
            assert_eq!(res.len(), 2, "two pngs");
        }
    }

    #[tokio::test]
    async fn collection_mod_kind() {
        let mut conn: PoolConnection<Sqlite> = setup_db().await;

        // one video:
        {
            let video_kind_mod = CollectionModifier::Kind(KindDetail::Video);

            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(Cond::all().add(video_kind_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            assert_eq!(r#"SELECT * FROM "info" WHERE ("format" ->> ?) = ?"#, select);

            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .unwrap();

            assert_eq!(res.len(), 1, "only one video");
        }

        // four photos:
        {
            let photo_kind_mod = CollectionModifier::Kind(KindDetail::Photo);

            let (select, values) = sea_query::Query::select()
                .column(Asterisk)
                .from(Info::Table)
                .cond_where(Cond::all().add(photo_kind_mod.to_query()))
                .build_sqlx(SqliteQueryBuilder);

            assert_eq!(r#"SELECT * FROM "info" WHERE ("format" ->> ?) = ?"#, select);

            let res = sqlx::query_as_with::<_, Media, _>(&select, values)
                .fetch_all(&mut *conn)
                .await
                .unwrap();

            assert_eq!(res.len(), 4, "four photos");
        }
    }

    /// creates a database with some entries in it...
    #[expect(clippy::inconsistent_digit_grouping, reason = "easier to read")]
    async fn setup_db() -> PoolConnection<Sqlite> {
        database::DB_FOLDER_PATH
            .set(temp_dir().try_into().unwrap())
            .unwrap();

        let mut conn = DATABASE.acquire().await.unwrap();

        let media_1 = Media {
            id: Uuid::from_u128(1),
            path: "/home/barrett/Videos/eceg_ditto_dill.mp4".into(),
            filesize: 1024 * 1024 * 512, // 512 MiB
            format: Format::new_from_mime("video/mp4")
                .expect("format creation")
                .into(),
            creation_date: Some(DateTime::from_timestamp_nanos(1737308081_000_000_000)),
            modification_date: Some(DateTime::from_timestamp_nanos(1737308098_000_000_000)),
            first_seen_date: DateTime::from_timestamp_nanos(1551731451_000_000_000),
            width_px: 1920,
            height_px: 1080,
            specific_metadata: SpecificMetadata::Video { length: 147.0 }.into(),
            other_metadata: Some(
                OtherMetadataMap(HashMap::from([
                    (
                        "uploader".into(),
                        OtherMetadataValue::new("Uploader", "DittoDill"),
                    ),
                    (
                        "category_id".into(),
                        OtherMetadataValue::new("Video Category ID", "24"),
                    ),
                ]))
                .into(),
            ),
            tags: vec![
                Tag::new_testing("dittodill"),
                Tag::new_testing("music"),
                Tag::new_testing("legend"),
            ]
            .into(),
        };

        let media_2 = Media {
            id: Uuid::from_u128(2),
            path: "/home/barrett/Downloads/vade.png".into(),
            filesize: (1024 * 34) + (230), // 34.2 KiB
            format: Format::new_from_mime("image/png").unwrap().into(),
            creation_date: Some(DateTime::from_timestamp_nanos(1737137976_000_000_000)),
            modification_date: Some(DateTime::from_timestamp_nanos(1737137976_000_000_000)),
            first_seen_date: DateTime::from_timestamp_nanos(1737126002_000_000_000),
            width_px: 174,
            height_px: 174,
            specific_metadata: SpecificMetadata::Image {}.into(),
            other_metadata: None,
            tags: vec![].into(),
        };

        let media_3 = Media {
            id: Uuid::from_u128(3),
            path: "/home/barrett/Downloads/a.jpg".into(),
            filesize: 1024 * 60, // 60 KiB
            format: Format::new_from_mime("image/jpeg").unwrap().into(),
            creation_date: Some(DateTime::from_timestamp_nanos(1730329781_000_000_000)),
            modification_date: Some(DateTime::from_timestamp_nanos(1730329781_000_000_000)),
            first_seen_date: DateTime::from_timestamp_nanos(1730329781_000_000_000),
            width_px: 1824,
            height_px: 1993,
            specific_metadata: SpecificMetadata::Image {}.into(),
            other_metadata: None,
            tags: vec![].into(),
        };

        let media_4 = Media {
            id: Uuid::from_u128(4),
            path: "/home/barrett/Pictures/2024-02-09 14-53-52.mkv-00:00:08.500.png".into(),
            filesize: 1024 * 765, // 765 KiB
            format: Format::new_from_mime("image/png").unwrap().into(),
            creation_date: Some(DateTime::from_timestamp_nanos(1725306903_000_000_000)),
            modification_date: Some(DateTime::from_timestamp_nanos(1725306903_000_000_000)),
            first_seen_date: DateTime::from_timestamp_nanos(1725286951_000_000_000),
            width_px: 1454,
            height_px: 750,
            specific_metadata: SpecificMetadata::Image {}.into(),
            other_metadata: None,
            tags: vec![].into(),
        };

        // a bunch of null-ish values >:)
        let media_5 = Media {
            id: Uuid::nil(),
            path: "".into(),
            filesize: 1024 * 765, // 765 KiB
            format: Format::new_from_mime("image/png").unwrap().into(),
            creation_date: None,
            modification_date: None,
            first_seen_date: DateTime::UNIX_EPOCH,
            width_px: 1,
            height_px: 0,
            specific_metadata: SpecificMetadata::Image {}.into(),
            other_metadata: None,
            tags: vec![].into(),
        };

        let m = [media_1, media_2, media_3, media_4, media_5];

        for media in m {
            media
                .make_insertion_query()
                .execute(&mut *conn)
                .await
                .unwrap();
        }

        conn
    }
}
