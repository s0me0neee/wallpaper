use image::codecs::jpeg::JpegEncoder;
use std::{
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

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

fn generate(path: &Path) -> Option<Vec<u8>> {
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
