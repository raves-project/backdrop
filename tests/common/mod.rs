//! The parent of the other tests.
//!
//! Mostly to import the setup stuff below.

use camino::Utf8PathBuf;

use std::{
    env::temp_dir,
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
};

use backdrop::{
    config::{BugReportInfo, Config, CONFIG},
    database,
    error::bug_msg,
};
use tracing_subscriber::{filter, layer::SubscriberExt as _, util::SubscriberInitExt as _, Layer};
use uuid::Uuid;

/// args for setup
#[allow(dead_code, reason = "it's used in the other tests")]
pub struct Setup {
    pub port: u16,
    pub watched_folders: Vec<Utf8PathBuf>,
}

impl Setup {
    #[allow(dead_code, reason = "it's used in the other tests")]
    pub fn new(port: u16) -> Self {
        Self {
            port,
            watched_folders: vec!["tests/assets/".into()],
        }
    }
}

/// call this at the top of any new test func! :)
#[allow(dead_code, reason = "it's used in the other tests")]
pub async fn setup(args: Setup) {
    // create tokio-console logger + server
    let tokio_console_layer = console_subscriber::ConsoleLayer::builder()
        .with_default_env()
        .server_addr(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), args.port))
        .spawn();

    // start logging (plus the tokio-console stuff)
    tracing_subscriber::registry()
        .with(tokio_console_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_filter(filter::EnvFilter::from_str("DEBUG,sqlx=INFO").unwrap()),
        )
        .init();

    // initialize the config (required)
    init_config_testing(&args.watched_folders).await;

    // setup the database location (also required)
    {
        let db_temp_dir = Utf8PathBuf::try_from(temp_dir())
            .unwrap()
            .join(Uuid::new_v4().to_string())
            .join("_raves_db");

        tokio::fs::create_dir_all(&db_temp_dir)
            .await
            .expect("create db temp dir");

        database::DB_FOLDER_PATH
            .set(db_temp_dir)
            .expect("db folder path should be unset");
    }
}

/// Initializes the config static with testing values.
pub async fn init_config_testing(watched_paths: &[Utf8PathBuf]) {
    if CONFIG.get().is_none() {
        Config::init_config(
            watched_paths,
            temp_dir().try_into().unwrap(),
            temp_dir().try_into().unwrap(),
            new_bug_report_info_testing(),
        )
        .await;
    } else {
        tracing::error!(
            "attempted to init the config, but the config is already running. {}",
            bug_msg().await
        )
    }
}

/// Sample bug report information for usage in tests, to decrease
/// verbosity.
pub fn new_bug_report_info_testing() -> BugReportInfo {
    BugReportInfo {
        app_name: "bug report info testing info".to_string(),
        app_version: "0.1.0".to_string(),
        device: "desktop".to_string(),
        display: "lineage_and_some_other_stuff".to_string(),
        target_triple: "x86_64-farts-gnu".to_string(),
        commit: "unknown".to_string(),
        repo: "https://github.com/onkoe/backdrop".to_string(),
        build_time: "unknown".to_string(),
    }
}
