use base64::{engine::general_purpose::STANDARD, Engine};
use image::codecs::jpeg::JpegEncoder;
use rayon::prelude::*;
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::UNIX_EPOCH,
};
use tauri::Emitter;

#[derive(Serialize, Clone)]
pub struct ImageEntry {
    pub path: String,
    pub thumbnail: String,
}

#[derive(Serialize, Clone)]
pub struct LoadDone {
    pub loaded: usize,
    pub skipped: usize,
}

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp"];

const THUMB_W: u32 = 1000;
const THUMB_H: u32 = 600;
const THUMB_QUALITY: u8 = 92;

// Stable, deterministic hash for cache keys (no extra deps).
fn fnv1a(data: &[u8]) -> u64 {
    data.iter().fold(14695981039346656037u64, |h, &b| {
        (h ^ b as u64).wrapping_mul(1099511628211)
    })
}

fn cache_path(img_path: &Path, mtime_secs: u64) -> Option<PathBuf> {
    let dir = dirs::cache_dir()?.join("wallpaper/thumbnails");
    std::fs::create_dir_all(&dir).ok()?;
    let hash = fnv1a(img_path.as_os_str().as_encoded_bytes());
    // Embed dimensions + quality so changing them auto-invalidates old cache entries.
    Some(dir.join(format!(
        "{hash:016x}_{mtime_secs}_{THUMB_W}x{THUMB_H}_q{THUMB_QUALITY}.jpg"
    )))
}

// Decode + resize the full image, freeing it before the encode buffer is allocated.
fn generate_thumbnail(path: &Path) -> Option<Vec<u8>> {
    let thumb = {
        let img = image::open(path).ok()?;
        img.resize(THUMB_W, THUMB_H, image::imageops::FilterType::Lanczos3)
    }; // full-res image freed here

    let mut buf = Vec::new();
    JpegEncoder::new_with_quality(&mut buf, THUMB_QUALITY)
        .encode_image(&thumb)
        .ok()?;
    Some(buf)
}

fn thumbnail_cached(path: &Path) -> Option<Vec<u8>> {
    let mtime = path
        .metadata()
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();

    if let Some(cache) = cache_path(path, mtime) {
        if cache.exists() {
            if let Ok(data) = std::fs::read(&cache) {
                return Some(data);
            }
        }

        let data = generate_thumbnail(path)?;
        let _ = std::fs::write(&cache, &data); // best-effort; ignore write errors
        Some(data)
    } else {
        generate_thumbnail(path)
    }
}

#[tauri::command]
pub fn start_load_images(dir: Option<String>, app: tauri::AppHandle) -> Result<(), String> {
    let dir_path: PathBuf = match dir {
        Some(d) => PathBuf::from(d),
        None => dirs::picture_dir()
            .ok_or_else(|| "Could not find pictures directory".to_string())?,
    };

    log::info!("Scanning: {}", dir_path.display());

    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir_path)
        .map_err(|e| {
            log::error!("Failed to read {}: {}", dir_path.display(), e);
            e.to_string()
        })?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
                    .unwrap_or(false)
        })
        .collect();

    paths.sort();
    let total = paths.len();
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
            paths.par_iter().for_each(|path| {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();

                match thumbnail_cached(path) {
                    Some(buf) => {
                        let n = loaded.fetch_add(1, Ordering::Relaxed) + 1;
                        log::debug!("[{n}/{total}] {name}");
                        let entry = ImageEntry {
                            path: path.to_string_lossy().into_owned(),
                            thumbnail: format!(
                                "data:image/jpeg;base64,{}",
                                STANDARD.encode(&buf)
                            ),
                        };
                        app.emit("thumbnail", entry).ok();
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
            LoadDone { loaded: n_loaded, skipped: n_skipped },
        )
        .ok();
    });

    Ok(())
}
