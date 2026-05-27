use crate::types;
use mlua::{Lua, Table};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub image_dir: Option<PathBuf>,
    pub order: types::SortBy,
    pub number_of_cols: u16,
    pub subdir: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub skip_set_wallpaper: bool,
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
            skip_set_wallpaper: false,
        }
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(PathBuf),

    #[error("Path is a directory, not a file: {0}")]
    NotAFile(PathBuf),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

fn config_path() -> Option<PathBuf> {
    let file = "conf.lua";
    #[cfg(target_os = "macos")]
    {
        let xdg = dirs::home_dir().map(|h| h.join(".config").join("wallpaper").join(file));
        if let Some(ref p) = xdg {
            if p.exists() {
                return xdg;
            }
        }
        dirs::config_dir().map(|d| d.join("wallpaper").join(file))
    }

    #[cfg(not(target_os = "macos"))]
    {
        dirs::config_dir().map(|d| d.join("wallpaper").join(file))
    }
}

fn read(path: &std::path::Path) -> Result<String, ConfigError> {
    if path.is_dir() {
        return Err(ConfigError::NotAFile(path.to_path_buf()));
    }
    if !path.exists() {
        return Err(ConfigError::NotFound(path.to_path_buf()));
    }
    Ok(fs::read_to_string(path)?)
}

/// Load config from `conf.lua`. If a `post_command` function is defined in the
/// table, it is stored in the Lua globals as `_wall_post_command` for later use
/// by `set_wallpaper`. All other fields fall back to defaults if absent.
pub fn load(lua: &Lua) -> Setting {
    let Some(path) = config_path() else {
        log::warn!("Could not determine config directory, using defaults");
        return Setting::default();
    };

    let content = match read(&path) {
        Ok(s) => s,
        Err(ConfigError::NotFound(_)) => {
            log::info!("No config file, using defaults");
            return Setting::default();
        }
        Err(e) => {
            log::warn!("Failed to read config ({e}), using defaults");
            return Setting::default();
        }
    };

    let table: Table = match lua.load(&content).eval() {
        Ok(t) => t,
        Err(e) => {
            log::warn!("Failed to parse Lua config ({e}), using defaults");
            return Setting::default();
        }
    };

    log::info!("Config loaded from {}", path.display());

    if let Ok(Some(func)) = table.get::<Option<mlua::Function>>("post_command") {
        match lua.globals().set("_wall_post_command", func) {
            Ok(_) => log::info!("post_command function registered"),
            Err(e) => log::warn!("Failed to register post_command: {e}"),
        }
    }

    let def = Setting::default();

    let order = table
        .get::<Option<String>>("order")
        .ok()
        .flatten()
        .and_then(|s| match s.as_str() {
            "name" => Some(types::SortBy::Name),
            "name_desc" => Some(types::SortBy::NameDesc),
            "date" => Some(types::SortBy::Date),
            "date_old" => Some(types::SortBy::DateOld),
            "size" => Some(types::SortBy::Size),
            "size_asc" => Some(types::SortBy::SizeAsc),
            _ => None,
        })
        .unwrap_or(def.order);

    Setting {
        image_dir: table
            .get::<Option<String>>("image_dir")
            .ok()
            .flatten()
            .map(PathBuf::from),
        order,
        number_of_cols: table
            .get::<u16>("number_of_cols")
            .unwrap_or(def.number_of_cols),
        subdir: table.get::<bool>("subdir").unwrap_or(def.subdir),
        window_width: table
            .get::<u32>("window_width")
            .unwrap_or(def.window_width),
        window_height: table
            .get::<u32>("window_height")
            .unwrap_or(def.window_height),
        skip_set_wallpaper: table
            .get::<bool>("skip_set_wallpaper")
            .unwrap_or(def.skip_set_wallpaper),
    }
}
