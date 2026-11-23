//! Signature-based file carving.
//!
//! The image is read one 512-byte sector at a time. When a sector starts with a
//! known file header, a new file begins there. The file then grows sector by
//! sector until one of these happens:
//! - its exact size, read from its header, is reached;
//! - another known header starts, which marks the beginning of the next file;
//! - it reaches the maximum size allowed for its type;
//! - the image ends.
//!
//! This assumes files are stored in one piece, which is often but not always
//! true. Fragmented files come out damaged and are caught by `verify`.

use std::fs::{self, File};
use std::io::{self, BufWriter, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::signatures::{identify, Signature};

pub const SECTOR: usize = 512;
const READ_CHUNK: usize = 8 * 1024 * 1024;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub files: u64,
    pub bytes_scanned: u64,
}

struct OpenFile {
    out: BufWriter<File>,
    written: u64,
    limit: u64,
}

impl OpenFile {
    fn remaining(&self) -> u64 {
        self.limit - self.written
    }

    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        let n = data.len().min(self.remaining() as usize);
        self.out.write_all(&data[..n])?;
        self.written += n as u64;
        Ok(())
    }
}

/// Carve files out of `image` into category folders under `out_dir`.
pub fn run(image: &Path, out_dir: &Path) -> Result<Summary> {
    let file =
        File::open(image).with_context(|| format!("cannot open the image {}", image.display()))?;
    let total = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut last_percent = None;
    let summary = carve(file, out_dir, |scanned| {
        let percent = (scanned * 100).checked_div(total);
        if percent.is_some() && percent != last_percent {
            last_percent = percent;
            eprint!(
                "\rScanning: {:3}% ({} MiB)",
                percent.unwrap_or(0),
                scanned / (1024 * 1024)
            );
        }
    })?;
    eprintln!();
    println!(
        "Carving complete: {} file(s) recovered into {}",
        summary.files,
        out_dir.display()
    );
    Ok(summary)
}

/// Carve files from any byte stream. `progress` is called with the number of
/// bytes scanned so far, once per chunk read.
pub fn carve<R: Read>(
    mut reader: R,
    out_dir: &Path,
    mut progress: impl FnMut(u64),
) -> Result<Summary> {
    let mut summary = Summary::default();
    let mut current: Option<OpenFile> = None;
    let mut buf = vec![0u8; READ_CHUNK];

    loop {
        let filled = read_full(&mut reader, &mut buf)?;
        if filled == 0 {
            break;
        }
        for sector in buf[..filled].chunks(SECTOR) {
            if let Some(sig) = identify(sector) {
                finish(current.take())?;
                let path = output_path(out_dir, sig, summary.bytes_scanned)?;
                let limit = sig
                    .size
                    .and_then(|size| size(sector))
                    .filter(|&n| n > 0)
                    .map_or(sig.max_size, |n| n.min(sig.max_size));
                let out = BufWriter::new(
                    File::create(&path)
                        .with_context(|| format!("cannot create {}", path.display()))?,
                );
                current = Some(OpenFile {
                    out,
                    written: 0,
                    limit,
                });
                summary.files += 1;
            }
            if let Some(open) = current.as_mut() {
                open.write(sector)?;
                if open.remaining() == 0 {
                    finish(current.take())?;
                }
            }
            summary.bytes_scanned += sector.len() as u64;
        }
        progress(summary.bytes_scanned);
    }
    finish(current)?;
    Ok(summary)
}

fn output_path(out_dir: &Path, sig: &Signature, offset: u64) -> Result<PathBuf> {
    let dir = out_dir.join(sig.category.folder());
    fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    // Naming files after their position in the image keeps names unique and
    // lets you find a file again in a hex editor.
    Ok(dir.join(format!("{offset:012x}.{}", sig.ext)))
}

fn finish(file: Option<OpenFile>) -> io::Result<()> {
    if let Some(mut f) = file {
        f.out.flush()?;
    }
    Ok(())
}

/// Like `read`, but keeps reading until `buf` is full or the stream ends.
fn read_full<R: Read>(reader: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn padded(data: &[u8]) -> Vec<u8> {
        let mut v = data.to_vec();
        v.resize(data.len().div_ceil(SECTOR) * SECTOR, 0);
        v
    }

    #[test]
    fn splits_files_at_next_header_and_uses_exact_sizes() {
        let jpeg = padded(&[0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3]);
        let mut wav = b"RIFF\x0c\0\0\0WAVEdata".to_vec(); // RIFF says 12 + 8 = 20 bytes
        wav.resize(SECTOR * 2, 0xAA);
        let mut image = vec![0u8; SECTOR]; // leading empty sector
        image.extend(&jpeg);
        image.extend(&wav);

        let dir = tempfile::tempdir().unwrap();
        let summary = carve(image.as_slice(), dir.path(), |_| {}).unwrap();
        assert_eq!(summary.files, 2);
        assert_eq!(summary.bytes_scanned, image.len() as u64);

        let jpg = fs::read(dir.path().join("photos/000000000200.jpg")).unwrap();
        assert_eq!(jpg, jpeg);
        let wav_out = fs::read(dir.path().join("music/000000000400.wav")).unwrap();
        assert_eq!(wav_out.len(), 20);
    }

    #[test]
    fn image_without_headers_yields_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let summary = carve(&[0u8; 4096][..], dir.path(), |_| {}).unwrap();
        assert_eq!(summary.files, 0);
    }
}
