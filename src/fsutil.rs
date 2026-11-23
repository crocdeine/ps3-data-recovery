//! Small file-system helpers that behave the same on macOS, Linux and Windows.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Move `from` into the folder `dir`, keeping its file name.
///
/// If a file with that name already exists, a numbered name is used instead
/// (`photo.jpg` becomes `photo_1.jpg`), so nothing is ever overwritten.
/// Falls back to copy-then-delete when the destination is on another drive.
pub fn move_into(from: &Path, dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let name = from
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let dest = free_name(&dir.join(name));
    if fs::rename(from, &dest).is_err() {
        fs::copy(from, &dest)?;
        fs::remove_file(from)?;
    }
    Ok(dest)
}

/// `path` itself if it does not exist yet, otherwise `stem_N.ext` for the first free N.
pub fn free_name(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path.extension().and_then(|e| e.to_str());
    (1u64..)
        .map(|n| {
            let name = match ext {
                Some(ext) => format!("{stem}_{n}.{ext}"),
                None => format!("{stem}_{n}"),
            };
            path.with_file_name(name)
        })
        .find(|p| !p.exists())
        .expect("ran out of file names")
}
