//! Helps to connect to the database.

use std::{
    str::FromStr,
    sync::{LazyLock, OnceLock},
};

use camino::Utf8PathBuf;
use sqlx::{
    query::Query,
    sqlite::{SqliteArguments, SqliteConnectOptions},
    Pool, Sqlite,
};

pub const INFO_TABLE: &str = "info";
pub const THUMBNAILS_TABLE: &str = "thumbnail";

/// A path to the folder containing the backend's database.
///
/// DO NOT set this to the database file - it will fail to initialize.
pub static DB_FOLDER_PATH: OnceLock<Utf8PathBuf> = OnceLock::new();

/// The database pool.
///
/// You MUST set the [`DB_FOLDER_PATH`] before attempting to access this.
/// Otherwise, the backend will panic!
pub static DATABASE: LazyLock<Pool<Sqlite>> = LazyLock::new(|| {
    const RAVES_DB_FILE: &str = "raves.sqlite";

    // try to get the folder path (hoping the user has set the OnceLock static)
    let Some(raves_db_folder) = DB_FOLDER_PATH.get() else {
        tracing::error!("Attempted to access the database before initializing the path!");
        tracing::error!("Since we don't know where the database is, the backend will now panic.");
        panic!("No database folder path given.");
    };

    // ensure the path exists
    match raves_db_folder.try_exists() {
        Ok(true) => (),
        Ok(false) => {
            tracing::error!("The given database folder does not exist!");
            panic!("Database folder doesn't exist.");
        }
        Err(e) => {
            tracing::error!("Failed to check if database folder exists! err: {e}");
            tracing::warn!("This might be because of file permissions.");
            panic!("Couldn't check if database folder exists. err: {e}");
        }
    }

    let options =
        SqliteConnectOptions::from_str(&format!("sqlite://{raves_db_folder}/{RAVES_DB_FILE}"))
            .inspect_err(|e| {
                tracing::error!(
                    "Failed to parse database string. The provided path may be incorrect. err: {e}"
                )
            })
            .expect("database opts str")
            .create_if_missing(true);

    // connect to the pool
    let pool = sqlx::Pool::<Sqlite>::connect_lazy_with(options);
    // we'll also run migrations here real quick
    _ = futures::executor::block_on(sqlx::migrate!("./migrations").run(&pool)).inspect_err(|e| {
        tracing::error!(
            "Database connection succeeded, but migrating the database failed! err: {e}"
        )
    });

    pool
});

pub trait InsertIntoTable {
    /// This function provides the query that we'll execute to insert this type
    /// into the table defined above.
    ///
    /// This only constructs a query - it does not execute it!
    fn make_insertion_query(&self) -> Query<'_, Sqlite, SqliteArguments<'_>>;
}
