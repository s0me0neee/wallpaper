mod commands;
mod config;
mod plugins;
mod scanner;
mod test;
pub mod thumbnail;
mod types;

use mlua::Lua;
use plugins::mac_rounded_corners;
use tauri_plugin_cli::CliExt;

use std::sync::Mutex;
use tauri::Manager;

fn check_early_flags() {
    let args: Vec<String> = std::env::args().collect();
    let has = |flags: &[&str]| args[1..].iter().any(|a| flags.contains(&a.as_str()));

    if has(&["-h", "--help"]) {
        print!(concat!(
            "wall ",
            env!("CARGO_PKG_VERSION"),
            "\n",
            "Floating wallpaper picker and CLI setter\n",
            "\n",
            "USAGE:\n",
            "    wall [OPTIONS] [IMAGE]\n",
            "\n",
            "ARGS:\n",
            "    <IMAGE>    Path to image to set as wallpaper\n",
            "\n",
            "OPTIONS:\n",
            "    -h, --help       Print this help\n",
            "    -V, --version    Print version\n",
            "    -v, --verbose    Increase log verbosity (-v debug, -vv trace)\n",
            "    -q, --quiet      Suppress all output\n",
        ));
        std::process::exit(0);
    }

    if has(&["-V", "--version"]) {
        println!("wall {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }
}

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
    // Dev builds: all logs by default; release (GUI + CLI): info by default.
    #[cfg(debug_assertions)]
    let base = log::LevelFilter::Trace;
    #[cfg(not(debug_assertions))]
    let base = log::LevelFilter::Info;

    match v {
        0 => base,
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    check_early_flags();
    test::bench_if_requested();
    std::thread::spawn(thumbnail::cleanup);

    let start = std::time::Instant::now();
    let lua = Lua::new();
    let cfg = config::load(&lua);
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
.manage(Mutex::new(cfg))
        .manage(Mutex::new(lua))
        .setup(move |app| {
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
                        log::debug!("startup: {:.2?}", start.elapsed());
                        std::process::exit(0);
                    }
                }
            }

            // --- GUI mode ---
            let cfg = app
                .state::<Mutex<config::Setting>>()
                .lock()
                .unwrap()
                .clone();
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
            commands::focus_window,
            mac_rounded_corners::enable_rounded_corners,
            mac_rounded_corners::enable_modern_window_style,
            mac_rounded_corners::reposition_traffic_lights,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |_app, event| {
            if let tauri::RunEvent::Exit = event {
                log::info!("uptime: {:.2?}", start.elapsed());
            }
        });
}
