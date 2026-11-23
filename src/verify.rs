//! Check recovered files and set aside the ones that cannot be opened.
//!
//! Images and archives are checked with built-in decoders, so nothing extra
//! needs to be installed. Audio and video are checked with `ffmpeg` when it is
//! installed; otherwise they are counted as "not checked" and left in place.

use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::fsutil::move_into;

pub const CORRUPTED: &str = "corrupted";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Valid,
    Corrupted,
    NotChecked,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub valid: u64,
    pub corrupted: u64,
    pub not_checked: u64,
}

/// Check every file in the category folders of `folder`, moving broken ones
/// into `<category>/corrupted/`.
pub fn run(folder: &Path) -> Result<Summary> {
    let has_ffmpeg = ffmpeg_available();
    if !has_ffmpeg {
        eprintln!("Note: ffmpeg was not found, so audio and video files will not be checked.");
    }
    let mut summary = Summary::default();
    for category_dir in subdirs(folder)? {
        for file in files_in(&category_dir)? {
            match check(&file, has_ffmpeg) {
                Verdict::Valid => summary.valid += 1,
                Verdict::NotChecked => summary.not_checked += 1,
                Verdict::Corrupted => {
                    summary.corrupted += 1;
                    println!("[corrupted] {}", file.display());
                    move_into(&file, &category_dir.join(CORRUPTED))
                        .with_context(|| format!("cannot move {}", file.display()))?;
                }
            }
        }
    }
    println!(
        "Check complete: {} valid, {} corrupted, {} not checked",
        summary.valid, summary.corrupted, summary.not_checked
    );
    Ok(summary)
}

/// Check one file, based on its extension.
pub fn check(path: &Path, has_ffmpeg: bool) -> Verdict {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ok = match ext.as_str() {
        "jpg" | "jpeg" | "mpo" | "png" | "gif" | "bmp" => image_ok(path),
        "zip" => zip_ok(path),
        "gz" => gz_ok(path),
        "tar" => tar_ok(path),
        "sfo" => starts_with(path, b"\0PSF"),
        "xml" | "txt" => text_ok(path),
        "mp4" | "avi" | "m2ts" | "mts" | "mp3" | "aac" | "wav" | "at3" if has_ffmpeg => {
            ffmpeg_ok(path)
        }
        _ => return Verdict::NotChecked,
    };
    if ok {
        Verdict::Valid
    } else {
        Verdict::Corrupted
    }
}

fn image_ok(path: &Path) -> bool {
    image::ImageReader::open(path)
        .and_then(|r| r.with_guessed_format())
        .map_err(image::ImageError::IoError)
        .and_then(|r| r.decode())
        .is_ok()
}

fn zip_ok(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let Ok(mut archive) = zip::ZipArchive::new(BufReader::new(file)) else {
        return false;
    };
    (0..archive.len()).all(|i| {
        archive
            .by_index(i)
            .map(|mut entry| io::copy(&mut entry, &mut io::sink()).is_ok())
            .unwrap_or(false)
    })
}

fn gz_ok(path: &Path) -> bool {
    File::open(path)
        .map(|f| {
            io::copy(
                &mut flate2::read::MultiGzDecoder::new(BufReader::new(f)),
                &mut io::sink(),
            )
            .is_ok()
        })
        .unwrap_or(false)
}

fn tar_ok(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let mut archive = tar::Archive::new(BufReader::new(file));
    let Ok(entries) = archive.entries() else {
        return false;
    };
    let mut count = 0;
    for entry in entries {
        let Ok(mut e) = entry else { return false };
        if io::copy(&mut e, &mut io::sink()).is_err() {
            return false;
        }
        count += 1;
    }
    count > 0
}

fn starts_with(path: &Path, magic: &[u8]) -> bool {
    let mut head = vec![0u8; magic.len()];
    File::open(path)
        .and_then(|mut f| f.read_exact(&mut head))
        .map(|()| head == magic)
        .unwrap_or(false)
}

fn text_ok(path: &Path) -> bool {
    // Carved text files are padded with zeros up to a whole sector.
    fs::read(path)
        .map(|bytes| {
            let end = bytes.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
            end > 0 && std::str::from_utf8(&bytes[..end]).is_ok()
        })
        .unwrap_or(false)
}

fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn ffmpeg_ok(path: &Path) -> bool {
    Command::new("ffmpeg")
        .args(["-v", "error", "-xerror", "-i"])
        .arg(path)
        .args(["-f", "null", "-"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn subdirs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = read_dir(dir)?.into_iter().filter(|p| p.is_dir()).collect();
    dirs.sort();
    Ok(dirs)
}

fn files_in(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = read_dir(dir)?.into_iter().filter(|p| p.is_file()).collect();
    files.sort();
    Ok(files)
}

fn read_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    fs::read_dir(dir)
        .with_context(|| format!("cannot read folder {}", dir.display()))?
        .map(|e| e.map(|e| e.path()).map_err(Into::into))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn tiny_png() -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        image::RgbImage::new(2, 2)
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    fn tiny_zip() -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        let mut z = zip::ZipWriter::new(&mut out);
        z.start_file("a.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(b"hello").unwrap();
        z.finish().unwrap();
        out.into_inner()
    }

    #[test]
    fn moves_only_broken_files() {
        let dir = tempfile::tempdir().unwrap();
        let photos = dir.path().join("photos");
        let archives = dir.path().join("archives");
        fs::create_dir_all(&photos).unwrap();
        fs::create_dir_all(&archives).unwrap();

        let png = tiny_png();
        fs::write(photos.join("good.png"), &png).unwrap();
        fs::write(photos.join("bad.png"), &png[..png.len() / 2]).unwrap();
        let zip = tiny_zip();
        fs::write(archives.join("good.zip"), &zip).unwrap();
        fs::write(archives.join("bad.zip"), &zip[..10]).unwrap();
        fs::write(archives.join("unknown.bin"), b"??").unwrap();

        let summary = run(dir.path()).unwrap();
        assert_eq!(summary.valid, 2);
        assert_eq!(summary.corrupted, 2);
        assert_eq!(summary.not_checked, 1);
        assert!(photos.join("good.png").is_file());
        assert!(photos.join("corrupted/bad.png").is_file());
        assert!(archives.join("corrupted/bad.zip").is_file());
        assert!(archives.join("unknown.bin").is_file());
    }

    #[test]
    fn text_with_zero_padding_is_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.xml");
        let mut data = b"<?xml version=\"1.0\"?><a/>".to_vec();
        data.resize(512, 0);
        fs::write(&path, data).unwrap();
        assert_eq!(check(&path, false), Verdict::Valid);
    }
}
