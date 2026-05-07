use std::{env, path::PathBuf};

const TEST_DIR: &str = "Pictures/wallpaper";

fn env_flag(var: &str) -> bool {
    env::var(var)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn startup_dir() -> Option<PathBuf> {
    if env_flag("TEST") {
        Some(dirs::home_dir()?.join(TEST_DIR))
    } else {
        None
    }
}

/// If BENCH=1/true, delegates to `cargo bench` (criterion) and exits.
pub fn bench_if_requested() {
    if !env_flag("BENCH") {
        return;
    }
    // CARGO_MANIFEST_DIR is baked in at compile time — always points to src-tauri/
    let src_tauri = env!("CARGO_MANIFEST_DIR");
    eprintln!("Running criterion benchmarks in {src_tauri}\n");
    let status = std::process::Command::new("cargo")
        .args(["bench", "--color", "always"])
        .current_dir(src_tauri)
        .status()
        .expect("failed to exec `cargo bench`");
    std::process::exit(status.code().unwrap_or(1));
}
