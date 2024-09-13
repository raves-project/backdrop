use std::{path::PathBuf, sync::OnceLock};

use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::error::{bug_msg, ConfigError};

pub type SharedConfig = RwLock<Config>;

// this will be initialized by the app itself
pub static CONFIG: OnceLock<SharedConfig> = OnceLock::new();

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// The paths that we'll be watching for new files.
    pub watched_paths: Vec<PathBuf>,

    /// Path to the app's data directory.
    pub data_dir: PathBuf,

    /// Path to the app's cache directory.
    pub cache_dir: PathBuf,

    /// Information for automatically reporting bugs.
    pub bug_report_info: BugReportInfo,
}

impl Config {
    pub fn new(
        watched_paths: Vec<PathBuf>,
        data_dir: PathBuf,
        cache_dir: PathBuf,
        bug_report_info: BugReportInfo,
    ) -> Self {
        Self {
            watched_paths,
            data_dir,
            cache_dir,
            bug_report_info,
        }
    }

    /// Attempts to read a previous `Config` from disk.
    ///
    /// Note that this may fail across versions, requiring new configs.
    pub async fn from_disk(data_dir: PathBuf) -> Result<Self, ConfigError> {
        // read the config from disk
        let s = tokio::fs::read_to_string(data_dir.join("shared_prefs/config.toml"))
            .await
            .map_err(ConfigError::ReadFailed)?;

        // parse with `toml` crate
        let s: Self = toml::from_str(s.as_str()).map_err(ConfigError::ParseFailed)?;

        // ensure paths are equal
        if s.data_dir != data_dir {
            tracing::error!(
                "loaded config from disk, but it had some weird paths. {}",
                bug_msg().await
            );
            return Err(ConfigError::PathMismatch);
        }

        Ok(s)
    }

    /// Use this EXACTLY ONCE to initialize the config.
    ///
    /// The app should be the only one calling this.
    pub async fn init_config(
        watched_paths: &[PathBuf],
        data_dir: PathBuf,
        cache_dir: PathBuf,
        bug_report_info: BugReportInfo,
    ) {
        if CONFIG.get().is_none() {
            let conf = RwLock::new(Config {
                watched_paths: watched_paths.into(),
                data_dir,
                cache_dir,
                bug_report_info,
            });

            CONFIG
                .set(conf)
                .expect("the config should not be configured yet");
        } else {
            tracing::error!(
                "attempted to init the config, but the config is already running. {}",
                bug_msg().await
            )
        }
    }

    /// Grabs the config for reading.
    ///
    /// Note that while you're reading the config, others cannot write to it.
    /// DO NOT HOLD ONTO IT FOR A LONG TIME.
    pub async fn read() -> RwLockReadGuard<'static, Config> {
        CONFIG
            .get()
            .expect("should have initialized already")
            .read()
            .await
    }

    pub async fn write() -> RwLockWriteGuard<'static, Config> {
        CONFIG
            .get()
            .expect("should have initialized already")
            .write()
            .await
    }
}

/// Some info to help with bug reporting.
///
/// I really want this for telling users where to report bugs.
#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct BugReportInfo {
    pub app_name: String,
    pub app_version: String,

    pub target_triple: String,
    pub build_time: String,

    /// the device string (e.g. `Google Pixel 6 Pro (raven)`)
    pub device: String,
    /// that big string you get from an Android's `Build.DISPLAY` field
    pub display: String,

    pub commit: String,
    pub repo: String,
}

// TODO: add things like available tabs, etc.
pub struct AppAppearance {}
