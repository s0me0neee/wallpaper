mod commands;
mod config;
mod plugins;
mod scanner;
mod test;
pub mod thumbnail;
mod types;

use plugins::mac_rounded_corners;
use tauri_plugin_cli::CliExt;

use std::sync::Mutex;
use tauri::Manager;

fn verbosity_level() -> log::LevelFilter {
    let args: Vec<String> = std::env::args().collect();
    if args[1..].iter().any(|a| a == "-q" || a == "--quiet") {
        return log::LevelFilter::Error;
    }
    let mut v = 0usize;
    for a in &args[1..] {
        if a == "--verbose" {
            v += 1;
        } else if let Some(flags) = a.strip_prefix('-').filter(|s| !s.starts_with('-')) {
            v += flags.chars().filter(|c| *c == 'v').count();
        }
    }
    match v {
        0 => log::LevelFilter::Info,
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    test::bench_if_requested();
    std::thread::spawn(thumbnail::cleanup);

    let cfg = config::load();
    let level = verbosity_level();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(level)
                .format(|out, message, record| {
                    let level = match record.level() {
                        log::Level::Error => "\x1b[31;1mERROR\x1b[0m",
                        log::Level::Warn => "\x1b[33;1m WARN\x1b[0m",
                        log::Level::Info => "\x1b[32;1m INFO\x1b[0m",
                        log::Level::Debug => "\x1b[36mDEBUG\x1b[0m",
                        log::Level::Trace => "\x1b[37mTRACE\x1b[0m",
                    };
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let secs = now % 86400;
                    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
                    let file = record.file().unwrap_or("?");
                    let line = record.line().unwrap_or(0);
                    out.finish(format_args!(
                        "\x1b[2m{h:02}:{m:02}:{s:02} {file}:{line}\x1b[0m [{level}] {message}"
                    ))
                })
                .build(),
        )
        .plugin(tauri_plugin_cli::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(Mutex::new(cfg))
        .setup(|app| {
            // --- CLI mode: `wall <image>` ---
            if let Ok(matches) = app.cli().matches() {
                if let Some(arg) = matches.args.get("image") {
                    if let serde_json::Value::String(path) = &arg.value {
                        match wp::set_from_path(path) {
                            Ok(_) => log::info!("wallpaper set → {path}"),
                            Err(e) => {
                                log::error!("failed to set wallpaper: {e}");
                                std::process::exit(1);
                            }
                        }
                        std::process::exit(0);
                    }
                }
            }

            // --- GUI mode ---
            let cfg = app.state::<Mutex<config::Setting>>().lock().unwrap().clone();
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize {
                    width: cfg.window_width as f64,
                    height: cfg.window_height as f64,
                }));
                let _ = win.center();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_load_images,
            commands::get_startup_dir,
            commands::get_config,
            commands::save_config,
            commands::set_wallpaper,
            mac_rounded_corners::enable_rounded_corners,
            mac_rounded_corners::enable_modern_window_style,
            mac_rounded_corners::reposition_traffic_lights,
            mac_rounded_corners::focus_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
