use std::{env, path::PathBuf, process::exit};

const TEST_VAR: &str = "TEST";
const TEST_PIC_DIR: PathBuf = dirs::home_dir().unwrap().join("Pictures/wallpaper");
fn test_mode() -> bool {
    if let Ok(v) = env::var(TEST_VAR) {
        log::error!("Can't read TEST env var");
        exit(1);
    }
}
