use crate::types::SortBy;
use std::{
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp"];

struct FileMeta {
    path: PathBuf,
    modified: u64,
    size: u64,
}

fn is_image(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
            .unwrap_or(false)
}

pub fn scan(dir: &Path, sort_by: SortBy) -> Result<Vec<PathBuf>, String> {
    let mut entries: Vec<FileMeta> = std::fs::read_dir(dir)
        .map_err(|e| {
            log::error!("Failed to read {}: {}", dir.display(), e);
            e.to_string()
        })?
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if !is_image(&path) {
                return None;
            }
            let meta = path.metadata().ok()?;
            let modified = meta
                .modified()
                .ok()?
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_secs();
            Some(FileMeta {
                path,
                modified,
                size: meta.len(),
            })
        })
        .collect();

    match sort_by {
        SortBy::Name => entries.sort_by(|a, b| a.path.cmp(&b.path)),
        SortBy::NameDesc => entries.sort_by(|a, b| b.path.cmp(&a.path)),
        SortBy::Date => entries.sort_by_key(|b| std::cmp::Reverse(b.modified)),
        SortBy::DateOld => entries.sort_by_key(|a| a.modified),
        SortBy::Size => entries.sort_by_key(|b| std::cmp::Reverse(b.size)),
        SortBy::SizeAsc => entries.sort_by_key(|a| a.size),
    }

    Ok(entries.into_iter().map(|e| e.path).collect())
}
