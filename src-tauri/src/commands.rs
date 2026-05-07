use crate::{
    config,
    scanner,
    test,
    thumbnail,
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
use tauri::{Emitter, State};

#[tauri::command]
pub fn start_load_images(
    dir: Option<String>,
    sort_by: Option<SortBy>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir_path: PathBuf = match dir {
        Some(d) => PathBuf::from(d),
        None => dirs::picture_dir()
            .ok_or_else(|| "Could not find pictures directory".to_string())?,
    };

    log::info!("Scanning: {}", dir_path.display());

    let entries = scanner::scan(&dir_path, sort_by.unwrap_or_default())?;
    let total = entries.len();
    log::info!("Found {} image(s)", total);

    let threads = std::thread::available_parallelism()
        .map(|n| n.get().clamp(2, 6))
        .unwrap_or(4);

    std::thread::spawn(move || {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("failed to build thread pool");

        let loaded = AtomicUsize::new(0);
        let skipped = AtomicUsize::new(0);

        pool.install(|| {
            entries.par_iter().enumerate().for_each(|(index, entry)| {
                let name = entry.path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();

                match thumbnail::get(&entry.path) {
                    Some(buf) => {
                        let n = loaded.fetch_add(1, Ordering::Relaxed) + 1;
                        log::debug!("[{n}/{total}] {name}");
                        app.emit("thumbnail", ImageEntry {
                            index,
                            path: entry.path.to_string_lossy().into_owned(),
                            thumbnail: format!("data:image/jpeg;base64,{}", STANDARD.encode(&buf)),
                            modified: entry.modified,
                            size: entry.size,
                        })
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
        app.emit("load-done", LoadDone { loaded: n_loaded, skipped: n_skipped }).ok();
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

#[tauri::command]
pub fn save_config(setting: config::Setting, state: State<Mutex<config::Setting>>) -> Result<(), String> {
    config::save(&setting).map_err(|e| e.to_string())?;
    *state.lock().unwrap() = setting;
    Ok(())
}

#[tauri::command]
pub fn set_wallpaper(path: String) -> Result<(), String> {
    wp::set_from_path(&path).map_err(|e| e.to_string())
}
