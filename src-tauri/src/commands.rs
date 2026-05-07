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
use std::sync::OnceLock;
use tauri::{Emitter, State};
use tauri_plugin_notification::NotificationExt;

static THUMB_POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();

fn thumb_pool() -> &'static rayon::ThreadPool {
    THUMB_POOL.get_or_init(|| {
        let n = std::thread::available_parallelism()
            .map(|n| n.get().clamp(2, 8))
            .unwrap_or(4);
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .thread_name(|i| format!("thumb-{i}"))
            .stack_size(2 * 1024 * 1024) // 2 MB; default 8 MB is excessive for decode+resize
            .build()
            .expect("failed to build thumb pool")
    })
}

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

    std::thread::spawn(move || {
        let loaded = AtomicUsize::new(0);
        let skipped = AtomicUsize::new(0);

        thumb_pool().install(|| {
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

const POSTCMD: &str = "[postcmd]";

fn parse_notify(cmd: &str) -> Option<&str> {
    let s = cmd.trim();
    s.strip_prefix("${{notify '").and_then(|s| s.strip_suffix("'}}"))
}

#[tauri::command]
pub fn set_wallpaper(path: String, state: State<Mutex<config::Setting>>, app: tauri::AppHandle) -> Result<(), String> {
    wp::set_from_path(&path).map_err(|e| e.to_string())?;

    let cmds = state.lock().unwrap().post_command.cmds.clone();
    if !cmds.is_empty() {
        std::thread::spawn(move || {
            #[cfg(unix)]
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

            log::info!("{POSTCMD} running {} command(s) for {path}", cmds.len());
            #[cfg(unix)]
            log::debug!("{POSTCMD} shell: {shell}");

            for (i, raw) in cmds.iter().enumerate() {
                let cmd_str = raw.replace("${{wallpaper}}", &path);
                let n = i + 1;

                if let Some(body) = parse_notify(&cmd_str) {
                    log::info!("{POSTCMD} [{n}] notify → {body}");
                    match app.notification().builder().title("wallpaper").body(body).show() {
                        Ok(_)  => log::info!("{POSTCMD} [{n}] notification sent"),
                        Err(e) => log::warn!("{POSTCMD} [{n}] notification failed: {e}"),
                    }
                    continue;
                }

                log::info!("{POSTCMD} [{n}] $ {cmd_str}");

                let expr = {
                    #[cfg(unix)]
                    { duct::cmd!(&shell, "-l", "-c", &cmd_str) }
                    #[cfg(not(unix))]
                    { duct::cmd!("cmd", "/C", &cmd_str) }
                };

                match expr.stderr_to_stdout().read() {
                    Ok(out) if out.trim().is_empty() => log::info!("{POSTCMD} [{n}] done"),
                    Ok(out) => log::info!("{POSTCMD} [{n}] output:\n{out}"),
                    Err(e)  => log::warn!("{POSTCMD} [{n}] error: {e}"),
                }
            }
        });
    }

    Ok(())
}
