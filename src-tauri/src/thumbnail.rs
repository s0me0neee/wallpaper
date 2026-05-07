use image::codecs::jpeg::JpegEncoder;
use std::{
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

const MAX_CACHE_AGE_SECS: u64 = 30 * 24 * 3600; // 30 days
const MAX_CACHE_SIZE_MB: u64 = 200;

pub const THUMB_W: u32 = 1000;
pub const THUMB_H: u32 = 600;
pub const THUMB_QUALITY: u8 = 92;

fn fnv1a(data: &[u8]) -> u64 {
    data.iter().fold(14695981039346656037u64, |h, &b| {
        (h ^ b as u64).wrapping_mul(1099511628211)
    })
}

fn cache_path(img_path: &Path, mtime_secs: u64) -> Option<PathBuf> {
    let dir = dirs::cache_dir()?.join("wallpaper/thumbnails");
    std::fs::create_dir_all(&dir).ok()?;
    let hash = fnv1a(img_path.as_os_str().as_encoded_bytes());
    Some(dir.join(format!(
        "{hash:016x}_{mtime_secs}_{THUMB_W}x{THUMB_H}_q{THUMB_QUALITY}.jpg"
    )))
}

pub fn generate(path: &Path) -> Option<Vec<u8>> {
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

/// Runs at startup (background thread). Three passes:
/// 1. Duplicate versions — same image hash, different mtime in filename: keep newest, delete rest.
/// 2. Age — delete entries not written in MAX_CACHE_AGE_SECS.
/// 3. Size — if still over MAX_CACHE_SIZE_MB, delete oldest first.
pub fn cleanup() {
    let Some(dir) = dirs::cache_dir().map(|d| d.join("wallpaper/thumbnails")) else { return };
    if !dir.exists() {
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    struct Entry {
        path: PathBuf,
        hash_prefix: String, // first 16 hex chars — identifies the source image
        mtime: u64,          // mtime of the cache file itself (creation time proxy)
        size: u64,
    }

    let mut entries: Vec<Entry> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let name = path.file_name()?.to_str()?.to_owned();
            let hash_prefix = name.get(..16)?.to_owned();
            let meta = path.metadata().ok()?;
            let mtime = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();
            Some(Entry { path, hash_prefix, mtime, size: meta.len() })
        })
        .collect();

    let before = entries.len();

    // Pass 1: per-hash, keep only the newest version (source image was re-saved).
    entries.sort_by(|a, b| a.hash_prefix.cmp(&b.hash_prefix).then(b.mtime.cmp(&a.mtime)));
    let mut last_hash = String::new();
    entries.retain(|e| {
        if e.hash_prefix == last_hash {
            let _ = std::fs::remove_file(&e.path);
            false
        } else {
            last_hash = e.hash_prefix.clone();
            true
        }
    });

    // Pass 2: age eviction.
    entries.retain(|e| {
        if now.saturating_sub(e.mtime) > MAX_CACHE_AGE_SECS {
            let _ = std::fs::remove_file(&e.path);
            false
        } else {
            true
        }
    });

    // Pass 3: size eviction — delete oldest until under the limit.
    let mut total: u64 = entries.iter().map(|e| e.size).sum();
    let limit = MAX_CACHE_SIZE_MB * 1024 * 1024;
    if total > limit {
        entries.sort_by_key(|e| e.mtime); // oldest first
        for e in &entries {
            if total <= limit {
                break;
            }
            if std::fs::remove_file(&e.path).is_ok() {
                total -= e.size;
            }
        }
    }

    let removed = before - entries.len();
    if removed > 0 {
        log::info!("Cache cleanup: removed {removed} entries, {} remaining", entries.len());
    }
}

pub fn get(path: &Path) -> Option<Vec<u8>> {
    let mtime = path
        .metadata().ok()?
        .modified().ok()?
        .duration_since(UNIX_EPOCH).ok()?
        .as_secs();

    let cache = cache_path(path, mtime)?;

    if cache.exists() {
        if let Ok(data) = std::fs::read(&cache) {
            return Some(data);
        }
    }

    let data = generate(path)?;
    let _ = std::fs::write(&cache, &data);
    Some(data)
}
