mod commands;
mod config;
mod scanner;
mod test;
pub mod thumbnail;
mod types;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    test::bench_if_requested();
    std::thread::spawn(thumbnail::cleanup);

    let cfg = config::load();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Debug)
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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(cfg))
        .setup(|app| {
            let cfg = app.state::<Mutex<config::Setting>>().lock().unwrap().clone();
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize {
                    width: cfg.window_width as f64,
                    height: cfg.window_height as f64,
                }));
                let _ = win.center();
                // Clear WKWebView's default opaque background — fixes white corners on macOS
                #[cfg(target_os = "macos")]
                let _ = win.set_background_color(None);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_load_images,
            commands::get_startup_dir,
            commands::get_config,
            commands::save_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
