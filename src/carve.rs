//! Signature-based file carving.
//!
//! The image is read one 512-byte sector at a time. When a sector starts with a
//! known file header, a new file begins there. Then:
//! - For most types, the file's structure is followed to find exactly where it
//!   ends (see [`crate::formats`]). The file is saved at that exact length and
//!   the scan resumes right after it, so pictures or files stored *inside* it
//!   (a JPEG's thumbnail, a file inside a ZIP) are not cut out separately.
//! - When that is not possible (the type has no end finder, or the structure
//!   is broken because the file is damaged or fragmented), the file grows
//!   sector by sector until the next known header, its type's size limit, or
//!   the end of the image. Such files may be incomplete; `verify` sorts them out.
//!
//! Carving assumes files are stored in one piece. The PS3 file system usually
//! does that for files written in one go, but not always.

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::formats::Src;
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
    fs::create_dir_all(out_dir).with_context(|| format!("cannot create {}", out_dir.display()))?;
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

/// Carve files from any seekable byte source. `progress` is called with the
/// position reached in the image, once per chunk read.
pub fn carve<R: Read + Seek>(
    reader: R,
    out_dir: &Path,
    mut progress: impl FnMut(u64),
) -> Result<Summary> {
    let mut reader = BufReader::with_capacity(64 * 1024, reader);
    let total = reader.seek(SeekFrom::End(0))?;
    let mut summary = Summary::default();
    let mut current: Option<OpenFile> = None;
    let mut buf = vec![0u8; READ_CHUNK];
    let mut pos: u64 = 0; // position in the image of buf[0]

    while pos < total {
        reader.seek(SeekFrom::Start(pos))?;
        let filled = read_full(&mut reader, &mut buf)?;
        if filled == 0 {
            break;
        }
        let mut next = pos + filled as u64;
        for (i, sector) in buf[..filled].chunks(SECTOR).enumerate() {
            let at = pos + (i * SECTOR) as u64;
            if let Some(sig) = identify(sector) {
                finish(current.take())?;
                let path = output_path(out_dir, sig, at)?;
                if let Some(len) = exact_length(&mut reader, sig, at, total)? {
                    copy_out(&mut reader, at, len, &path)?;
                    summary.files += 1;
                    next = (at + len).div_ceil(SECTOR as u64) * SECTOR as u64;
                    break;
                }
                let out = BufWriter::new(
                    File::create(&path)
                        .with_context(|| format!("cannot create {}", path.display()))?,
                );
                current = Some(OpenFile {
                    out,
                    written: 0,
                    limit: sig.max_size,
                });
                summary.files += 1;
            }
            if let Some(open) = current.as_mut() {
                open.write(sector)?;
                if open.remaining() == 0 {
                    finish(current.take())?;
                }
            }
        }
        pos = next;
        summary.bytes_scanned = pos.min(total);
        progress(summary.bytes_scanned);
    }
    finish(current)?;
    Ok(summary)
}

/// Exact length of the file starting at `at`, if its type has an end finder
/// and the data follows that type's structure.
fn exact_length<R: Read + Seek>(
    reader: &mut BufReader<R>,
    sig: &Signature,
    at: u64,
    total: u64,
) -> io::Result<Option<u64>> {
    let Some(end) = sig.end else {
        return Ok(None);
    };
    reader.seek(SeekFrom::Start(at))?;
    let limit = sig.max_size.min(total - at);
    let len = end(&mut Src::new(reader, limit));
    Ok(len.filter(|&n| n > 0 && n <= limit))
}

fn copy_out<R: Read + Seek>(
    reader: &mut BufReader<R>,
    at: u64,
    len: u64,
    path: &Path,
) -> Result<()> {
    reader.seek(SeekFrom::Start(at))?;
    let mut out = BufWriter::new(
        File::create(path).with_context(|| format!("cannot create {}", path.display()))?,
    );
    let copied = io::copy(&mut reader.by_ref().take(len), &mut out)?;
    out.flush()?;
    if copied != len {
        anyhow::bail!("the image ended while copying {}", path.display());
    }
    Ok(())
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
    use std::io::{Cursor, Write};

    fn pad_to_sector(v: &mut Vec<u8>) {
        v.resize(v.len().div_ceil(SECTOR) * SECTOR, 0);
    }

    fn encode(format: image::ImageFormat) -> Vec<u8> {
        let img =
            image::RgbImage::from_fn(40, 30, |x, y| image::Rgb([x as u8 * 5, y as u8 * 7, 99]));
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, format).unwrap();
        out.into_inner()
    }

    fn run_on(image: Vec<u8>) -> (tempfile::TempDir, Summary) {
        let dir = tempfile::tempdir().unwrap();
        let summary = carve(Cursor::new(image), dir.path(), |_| {}).unwrap();
        (dir, summary)
    }

    #[test]
    fn saves_files_at_their_exact_length() {
        let jpeg = encode(image::ImageFormat::Jpeg);
        let png = encode(image::ImageFormat::Png);
        let mut image = vec![0u8; SECTOR]; // leading empty sector
        image.extend(&jpeg);
        image.extend([0x5A; 100]); // leftover bytes after the JPEG
        pad_to_sector(&mut image);
        let png_at = image.len();
        image.extend(&png);
        pad_to_sector(&mut image);

        let (dir, summary) = run_on(image.clone());
        assert_eq!(summary.files, 2);
        assert_eq!(summary.bytes_scanned, image.len() as u64);
        let photos = dir.path().join("photos");
        assert_eq!(fs::read(photos.join("000000000200.jpg")).unwrap(), jpeg);
        assert_eq!(
            fs::read(photos.join(format!("{png_at:012x}.png"))).unwrap(),
            png
        );
    }

    #[test]
    fn does_not_cut_out_files_stored_inside_an_archive() {
        // A ZIP holding an uncompressed PNG that happens to sit on a sector boundary.
        let png = encode(image::ImageFormat::Png);
        let mut z = Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut z);
            let stored = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            let name = "p".repeat(SECTOR - 30); // local header is 30 bytes + name
            w.start_file(name, stored).unwrap();
            w.write_all(&png).unwrap();
            w.finish().unwrap();
        }
        let z = z.into_inner();
        assert_eq!(&z[SECTOR..SECTOR + 4], b"\x89PNG");
        let mut image = z.clone();
        pad_to_sector(&mut image);

        let (dir, summary) = run_on(image);
        assert_eq!(summary.files, 1);
        assert_eq!(
            fs::read(dir.path().join("archives/000000000000.zip")).unwrap(),
            z
        );
        assert!(!dir.path().join("photos").exists());
    }

    #[test]
    fn damaged_file_falls_back_to_next_header() {
        // A JPEG cut short, followed directly by a PNG.
        let jpeg = encode(image::ImageFormat::Jpeg);
        let mut image = jpeg[..600].to_vec();
        pad_to_sector(&mut image);
        let cut_len = image.len();
        image.extend(encode(image::ImageFormat::Png));
        pad_to_sector(&mut image);

        let (dir, summary) = run_on(image.clone());
        assert_eq!(summary.files, 2);
        let jpg = fs::read(dir.path().join("photos/000000000000.jpg")).unwrap();
        assert_eq!(jpg, image[..cut_len]);
    }

    #[test]
    fn image_without_headers_yields_nothing() {
        let (_dir, summary) = run_on(vec![0u8; 4096]);
        assert_eq!(summary.files, 0);
        assert_eq!(summary.bytes_scanned, 4096);
    }
}
