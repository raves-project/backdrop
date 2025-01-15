//! Helps to connect to the database.

use std::sync::LazyLock;

use sqlx::{Pool, Sqlite};

pub const INFO_TABLE: &str = "info";
pub const THUMBNAILS_TABLE: &str = "thumbnail";

pub static DATABASE: LazyLock<Pool<Sqlite>> = LazyLock::new(|| {
    const RAVES_DB_FILE_NAME: &str = "raves.sqlite";

    let pool =
        sqlx::Pool::<Sqlite>::connect_lazy(constcat::concat!("sqlite://", RAVES_DB_FILE_NAME))
            .inspect_err(|e| tracing::error!("Failed to connect to media info database. err: {e}"))
            .expect("err connecting to db");

    // we'll also run migrations here real quick
    _ = futures::executor::block_on(sqlx::migrate!("./migrations").run(&pool)).inspect_err(|e| {
        tracing::error!(
            "Database connection succeeded, but migrating the database failed! err: {e}"
        )
    });

    pool
});
