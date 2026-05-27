use crate::{
    config, scanner, test, thumbnail,
    types::{ImageEntry, LoadDone, SortBy},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rayon::prelude::*;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};
use tauri::{Emitter, Runtime, State};

/// Makes WKWebView (not a form control) the first responder so hjkl/arrow keys
/// work immediately without clicking the grid first.
#[tauri::command]
pub fn focus_window<R: Runtime>(
    _app: tauri::AppHandle<R>,
    window: tauri::WebviewWindow<R>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        window
            .with_webview(|webview| unsafe {
                use cocoa::base::{id, nil};
                #[allow(unused_imports)]
                use objc::{msg_send, sel, sel_impl};
                let ns_window: id = webview.ns_window() as id;
                let wkwebview: id = webview.inner() as id;
                if let Some(cls) = objc::runtime::Class::get("NSApplication") {
                    let ns_app: id = objc::msg_send![cls, sharedApplication];
                    let _: () =
                        objc::msg_send![ns_app, activateIgnoringOtherApps: cocoa::base::YES];
                }
                let _: () = objc::msg_send![ns_window, makeKeyAndOrderFront: nil];
                let _: () = objc::msg_send![ns_window, makeFirstResponder: wkwebview];
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn start_load_images(
    dir: Option<String>,
    sort_by: Option<SortBy>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir_path: PathBuf = match dir {
        Some(d) => PathBuf::from(d),
        None => {
            dirs::picture_dir().ok_or_else(|| "Could not find pictures directory".to_string())?
        }
    };

    log::info!("Scanning: {}", dir_path.display());

    let entries = scanner::scan(&dir_path, sort_by.unwrap_or_default())?;
    let total = entries.len();
    log::info!("Found {} image(s)", total);

    let threads = std::thread::available_parallelism()
        .map(|n| n.get().clamp(2, 8))
        .unwrap_or(4);

    std::thread::spawn(move || {
        let pool = match rayon::ThreadPoolBuilder::new().num_threads(threads).build() {
            Ok(p) => p,
            Err(e) => {
                log::error!("failed to build thread pool: {e}");
                app.emit(
                    "load-done",
                    LoadDone {
                        loaded: 0,
                        skipped: total,
                    },
                )
                .ok();
                return;
            }
        };

        let loaded = AtomicUsize::new(0);
        let skipped = AtomicUsize::new(0);

        pool.install(|| {
            entries.par_iter().enumerate().for_each(|(index, entry)| {
                let name = entry
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();

                match thumbnail::get(&entry.path) {
                    Some(buf) => {
                        let n = loaded.fetch_add(1, Ordering::Relaxed) + 1;
                        log::debug!("[{n}/{total}] {name}");
                        app.emit(
                            "thumbnail",
                            ImageEntry {
                                index,
                                path: entry.path.to_string_lossy().into_owned(),
                                thumbnail: format!(
                                    "data:image/jpeg;base64,{}",
                                    STANDARD.encode(&buf)
                                ),
                                modified: entry.modified,
                                size: entry.size,
                            },
                        )
                        .ok();
                    }
                    None => {
                        skipped.fetch_add(1, Ordering::Relaxed);
                        log::warn!("Skipped: {name}");
                    }
                }
            });
        });

        let n_loaded = loaded.load(Ordering::Relaxed);
        let n_skipped = skipped.load(Ordering::Relaxed);
        log::info!("Done — {n_loaded} loaded, {n_skipped} skipped");
        app.emit(
            "load-done",
            LoadDone {
                loaded: n_loaded,
                skipped: n_skipped,
            },
        )
        .ok();
    });

    Ok(())
}

#[tauri::command]
pub fn get_startup_dir() -> Option<String> {
    test::startup_dir().map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn get_config(state: State<Mutex<config::Setting>>) -> config::Setting {
    state.lock().unwrap().clone()
}

/// Updates the in-memory config. The Lua config file is user-managed and is not
/// written back; changes made here persist only for the current session.
#[tauri::command]
pub fn save_config(
    setting: config::Setting,
    state: State<Mutex<config::Setting>>,
) -> Result<(), String> {
    *state.lock().unwrap() = setting;
    Ok(())
}

const POSTCMD: &str = "[postcmd]";

#[tauri::command]
pub fn set_wallpaper(
    path: String,
    state: State<Mutex<config::Setting>>,
    lua_state: State<Mutex<mlua::Lua>>,
) -> Result<(), String> {
    let skip = state.lock().unwrap().skip_set_wallpaper;

    if skip {
        log::info!("skip_set_wallpaper=true — skipping wp::set_from_path");
    } else {
        wp::set_from_path(&path).map_err(|e| e.to_string())?;
    }

    let lua = lua_state.lock().unwrap();
    match lua
        .globals()
        .get::<Option<mlua::Function>>("_wall_post_command")
    {
        Ok(Some(func)) => {
            log::info!("{POSTCMD} calling Lua post_command(\"{path}\")");
            if let Err(e) = func.call::<()>(path.as_str()) {
                log::warn!("{POSTCMD} error: {e}");
            }
        }
        Ok(None) => {}
        Err(e) => log::warn!("{POSTCMD} error getting function: {e}"),
    }

    Ok(())
}
