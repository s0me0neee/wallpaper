use crate::types;
use std::{fs, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Setting {
    #[serde(default)]
    pub image_dir: Option<PathBuf>,
    #[serde(default)]
    pub order: types::SortBy,
    #[serde(default = "default_cols")]
    pub number_of_cols: u16,
    #[serde(default)]
    pub subdir: bool,
    #[serde(default = "default_win_w")]
    pub window_width: u32,
    #[serde(default = "default_win_h")]
    pub window_height: u32,
    // not persisted yet — kept for future use
    #[serde(skip)]
    #[allow(dead_code)]
    pub post_command: Option<types::PostCommand>,
    #[serde(skip)]
    #[allow(dead_code)]
    pub backend: Option<Backend>,
}

fn default_cols() -> u16 {
    4
}
fn default_win_w() -> u32 {
    720
}
fn default_win_h() -> u32 {
    520
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            image_dir: None,
            order: types::SortBy::default(),
            number_of_cols: default_cols(),
            subdir: false,
            window_width: default_win_w(),
            window_height: default_win_h(),
            post_command: None,
            backend: None,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum Backend {
    MacOSWallpaper,
    MacOSNative,
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(PathBuf),

    #[error("Path is a directory, not a file: {0}")]
    NotAFile(PathBuf),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    TomlSerError(#[from] toml::ser::Error),

    #[error("Deserialization error: {0}")]
    TomlDeError(#[from] toml::de::Error),
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("wallpaper").join("config.toml"))
}

fn ensure_file(path: &std::path::Path) -> Result<(), ConfigError> {
    if path.is_dir() {
        return Err(ConfigError::NotAFile(path.to_path_buf()));
    }
    if !path.exists() {
        return Err(ConfigError::NotFound(path.to_path_buf()));
    }
    Ok(())
}

pub fn load() -> Setting {
    let Some(path) = config_path() else {
        log::warn!("Could not determine config directory, using defaults");
        return Setting::default();
    };

    match read(&path) {
        Ok(s) => {
            log::info!("Config loaded from {}", path.display());
            s
        }
        Err(ConfigError::NotFound(_)) => {
            log::info!("No config file yet, using defaults");
            Setting::default()
        }
        Err(e) => {
            log::warn!("Failed to load config ({e}), using defaults");
            Setting::default()
        }
    }
}

fn read(path: &PathBuf) -> Result<Setting, ConfigError> {
    ensure_file(path)?;
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

pub fn save(setting: &Setting) -> Result<(), ConfigError> {
    let path = config_path().ok_or_else(|| {
        ConfigError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not determine config directory",
        ))
    })?;

    if path.is_dir() {
        return Err(ConfigError::NotAFile(path));
    }

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            log::debug!("Creating config dir: {}", parent.display());
            fs::create_dir_all(parent)?;
        }
    }

    let toml = toml::to_string_pretty(setting)?;
    fs::write(&path, toml)?;
    log::info!("Config saved to {}", path.display());
    Ok(())
}
