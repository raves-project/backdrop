#[cfg(test)]
mod tests {

    mod query {
        use std::{collections::HashMap, env::temp_dir};

        use chrono::DateTime;
        use sea_query::{Asterisk, Cond, SqliteQueryBuilder};
        use sea_query_binder::SqlxBinder as _;
        use sqlx::{pool::PoolConnection, Sqlite};
        use uuid::Uuid;

        use backdrop::{
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
                let after_modified_datetime_mod =
                    CollectionModifier::DateTime(DateTimeModifier::After(DateDetail::Modified(
                        DateTime::from_timestamp_nanos(0),
                    )));

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
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .init();

            let db_folder = temp_dir().join(Uuid::new_v4().to_string());
            tokio::fs::create_dir_all(&db_folder).await.unwrap();

            database::DB_FOLDER_PATH
                .set(db_folder.try_into().unwrap())
                .unwrap();

            let mut conn = DATABASE.acquire().await.unwrap();

            let media_1 = Media {
                id: Uuid::from_u128(1),
                path: "/home/barrett/Videos/eceg_ditto_dill.mp4".into(),
                album: "/home/barrett/Videos".into(),
                filesize: 1024 * 1024 * 512, // 512 MiB
                format: Format::new_from_mime("video/mp4")
                    .expect("format creation")
                    .into(),
                creation_date: Some(DateTime::from_timestamp_nanos(1737308081_000_000_000)),
                modification_date: Some(DateTime::from_timestamp_nanos(1737308098_000_000_000)),
                first_seen_date: DateTime::from_timestamp_nanos(1551731451_000_000_000),
                width_px: 1920,
                height_px: 1080,
                specific_metadata: SpecificMetadata::new_video(147.0).into(),
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
                album: "/home/barrett/Downloads".into(),
                filesize: (1024 * 34) + (230), // 34.2 KiB
                format: Format::new_from_mime("image/png").unwrap().into(),
                creation_date: Some(DateTime::from_timestamp_nanos(1737137976_000_000_000)),
                modification_date: Some(DateTime::from_timestamp_nanos(1737137976_000_000_000)),
                first_seen_date: DateTime::from_timestamp_nanos(1737126002_000_000_000),
                width_px: 174,
                height_px: 174,
                specific_metadata: SpecificMetadata::new_image().into(),
                other_metadata: None,
                tags: vec![].into(),
            };

            let media_3 = Media {
                id: Uuid::from_u128(3),
                path: "/home/barrett/Downloads/a.jpg".into(),
                album: "/home/barrett/Downloads".into(),
                filesize: 1024 * 60, // 60 KiB
                format: Format::new_from_mime("image/jpeg").unwrap().into(),
                creation_date: Some(DateTime::from_timestamp_nanos(1730329781_000_000_000)),
                modification_date: Some(DateTime::from_timestamp_nanos(1730329781_000_000_000)),
                first_seen_date: DateTime::from_timestamp_nanos(1730329781_000_000_000),
                width_px: 1824,
                height_px: 1993,
                specific_metadata: SpecificMetadata::new_image().into(),
                other_metadata: None,
                tags: vec![].into(),
            };

            let media_4 = Media {
                id: Uuid::from_u128(4),
                path: "/home/barrett/Pictures/2024-02-09 14-53-52.mkv-00:00:08.500.png".into(),
                album: "/home/barrett/Pictures".into(),
                filesize: 1024 * 765, // 765 KiB
                format: Format::new_from_mime("image/png").unwrap().into(),
                creation_date: Some(DateTime::from_timestamp_nanos(1725306903_000_000_000)),
                modification_date: Some(DateTime::from_timestamp_nanos(1725306903_000_000_000)),
                first_seen_date: DateTime::from_timestamp_nanos(1725286951_000_000_000),
                width_px: 1454,
                height_px: 750,
                specific_metadata: SpecificMetadata::new_image().into(),
                other_metadata: None,
                tags: vec![].into(),
            };

            // a bunch of null-ish values >:)
            let media_5 = Media {
                id: Uuid::nil(),
                album: "/".into(),
                path: "/nil.notpng.farts".into(),
                filesize: 1024 * 765, // 765 KiB
                format: Format::new_from_mime("image/png").unwrap().into(),
                creation_date: None,
                modification_date: None,
                first_seen_date: DateTime::UNIX_EPOCH,
                width_px: 1,
                height_px: 0,
                specific_metadata: SpecificMetadata::new_image().into(),
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
}
