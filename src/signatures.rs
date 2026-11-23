//! Known file headers ("magic numbers") used to spot files in a raw image.
//!
//! Each entry says which bytes a file of that type starts with, how big such a
//! file can reasonably get, and, where the header records it, how to read the
//! file's exact size.

use crate::category::Category;

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
const GIB: u64 = 1024 * MIB;

/// Reads a file's exact size from its first sector.
pub type SizeFn = fn(&[u8]) -> Option<u64>;

pub struct Signature {
    /// Extension given to recovered files.
    pub ext: &'static str,
    pub category: Category,
    /// Bytes expected at `offset` from the start of the file.
    pub magic: &'static [u8],
    pub offset: usize,
    /// Extra check on the first sector, to cut down on false matches.
    pub check: Option<fn(&[u8]) -> bool>,
    /// Exact file size, when the header records it.
    pub size: Option<SizeFn>,
    /// Carving never copies more than this many bytes for one file.
    pub max_size: u64,
}

impl Signature {
    /// Does `sector` (the first bytes of a candidate file) match this signature?
    pub fn matches(&self, sector: &[u8]) -> bool {
        let end = self.offset + self.magic.len();
        sector.len() >= end
            && &sector[self.offset..end] == self.magic
            && self.check.is_none_or(|check| check(sector))
    }
}

fn riff_kind(sector: &[u8], kind: &[u8; 4]) -> bool {
    sector.get(8..12) == Some(kind)
}

fn riff_size(sector: &[u8]) -> Option<u64> {
    let len = u32::from_le_bytes(sector.get(4..8)?.try_into().ok()?);
    Some(u64::from(len) + 8)
}

fn bmp_check(sector: &[u8]) -> bool {
    // Reserved bytes are zero and the pixel data offset fits a normal header.
    sector.len() >= 14
        && sector[6..10] == [0, 0, 0, 0]
        && (26..=1024).contains(&u32::from_le_bytes(sector[10..14].try_into().unwrap()))
}

fn bmp_size(sector: &[u8]) -> Option<u64> {
    Some(u64::from(u32::from_le_bytes(
        sector.get(2..6)?.try_into().ok()?,
    )))
}

fn m2ts_check(sector: &[u8]) -> bool {
    // M2TS is a sequence of 192-byte packets: a 4-byte timestamp then a 0x47 sync byte.
    [4usize, 196, 388]
        .iter()
        .all(|&i| sector.get(i) == Some(&0x47))
}

fn pkg_size(sector: &[u8]) -> Option<u64> {
    Some(u64::from_be_bytes(sector.get(0x18..0x20)?.try_into().ok()?))
}

#[rustfmt::skip]
pub const SIGNATURES: &[Signature] = &[
    // Photos
    Signature { ext: "jpg", category: Category::Photos, magic: &[0xFF, 0xD8, 0xFF, 0xE0], offset: 0, check: None, size: None, max_size: 50 * MIB },
    Signature { ext: "jpg", category: Category::Photos, magic: &[0xFF, 0xD8, 0xFF, 0xE1], offset: 0, check: None, size: None, max_size: 50 * MIB },
    Signature { ext: "jpg", category: Category::Photos, magic: &[0xFF, 0xD8, 0xFF, 0xDB], offset: 0, check: None, size: None, max_size: 50 * MIB },
    Signature { ext: "mpo", category: Category::Photos, magic: &[0xFF, 0xD8, 0xFF, 0xE2], offset: 0, check: None, size: None, max_size: 50 * MIB },
    Signature { ext: "png", category: Category::Photos, magic: b"\x89PNG\r\n\x1a\n", offset: 0, check: None, size: None, max_size: 50 * MIB },
    Signature { ext: "gif", category: Category::Photos, magic: b"GIF8", offset: 0, check: None, size: None, max_size: 50 * MIB },
    Signature { ext: "bmp", category: Category::Photos, magic: b"BM", offset: 0, check: Some(bmp_check), size: Some(bmp_size), max_size: 100 * MIB },
    // Videos
    Signature { ext: "mp4", category: Category::Videos, magic: b"ftyp", offset: 4, check: None, size: None, max_size: 4 * GIB },
    Signature { ext: "avi", category: Category::Videos, magic: b"RIFF", offset: 0, check: Some(|s| riff_kind(s, b"AVI ")), size: Some(riff_size), max_size: 4 * GIB },
    Signature { ext: "m2ts", category: Category::Videos, magic: &[0x47], offset: 4, check: Some(m2ts_check), size: None, max_size: 4 * GIB },
    // Music (ATRAC3 .at3 files are RIFF/WAVE too, so they come out as .wav)
    Signature { ext: "wav", category: Category::Music, magic: b"RIFF", offset: 0, check: Some(|s| riff_kind(s, b"WAVE")), size: Some(riff_size), max_size: 2 * GIB },
    Signature { ext: "mp3", category: Category::Music, magic: b"ID3", offset: 0, check: None, size: None, max_size: 200 * MIB },
    // PS3 files
    Signature { ext: "sfo", category: Category::Ps3Files, magic: b"\0PSF", offset: 0, check: None, size: None, max_size: 64 * KIB },
    Signature { ext: "pkg", category: Category::Ps3Files, magic: b"\x7FPKG", offset: 0, check: None, size: Some(pkg_size), max_size: 32 * GIB },
    Signature { ext: "self", category: Category::Ps3Files, magic: b"SCE\0", offset: 0, check: None, size: None, max_size: 256 * MIB },
    Signature { ext: "edat", category: Category::Ps3Files, magic: b"NPD\0", offset: 0, check: None, size: None, max_size: 4 * GIB },
    Signature { ext: "rco", category: Category::Ps3Files, magic: b"\0PRF", offset: 0, check: None, size: None, max_size: 64 * MIB },
    Signature { ext: "p3t", category: Category::Ps3Files, magic: b"P3TF", offset: 0, check: None, size: None, max_size: 256 * MIB },
    // Documents
    Signature { ext: "xml", category: Category::Documents, magic: b"<?xml ", offset: 0, check: None, size: None, max_size: 16 * MIB },
    // Archives
    Signature { ext: "zip", category: Category::Archives, magic: b"PK\x03\x04", offset: 0, check: None, size: None, max_size: 4 * GIB },
    Signature { ext: "gz", category: Category::Archives, magic: &[0x1F, 0x8B, 0x08], offset: 0, check: None, size: None, max_size: 4 * GIB },
    Signature { ext: "tar", category: Category::Archives, magic: b"ustar", offset: 257, check: None, size: None, max_size: 4 * GIB },
];

/// First signature matching the start of `sector`, if any.
pub fn identify(sector: &[u8]) -> Option<&'static Signature> {
    SIGNATURES.iter().find(|s| s.matches(sector))
}

/// The signature list in PhotoRec's custom signature format (`photorec.sig`).
///
/// Entries that need an extra check, or whose magic is shorter than 3 bytes,
/// are left out: PhotoRec cannot apply the check and would match far too often.
pub fn photorec_sig() -> String {
    let mut out = String::from(
        "# PhotoRec custom signatures for PS3 drives.\n\
         # Generated from src/signatures.rs: edit that file, then run\n\
         #   cargo run -- photorec-sig > photorec.sig\n",
    );
    let mut seen = Vec::new();
    for sig in SIGNATURES {
        if sig.check.is_some() || sig.magic.len() < 3 {
            continue;
        }
        let hex: String = sig.magic.iter().map(|b| format!("{b:02x}")).collect();
        let line = format!("{} {} 0x{hex}\n", sig.ext, sig.offset);
        if !seen.contains(&line) {
            out.push_str(&line);
            seen.push(line);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sector(prefix: &[u8]) -> Vec<u8> {
        let mut s = prefix.to_vec();
        s.resize(512, 0);
        s
    }

    #[test]
    fn identifies_common_types() {
        assert_eq!(
            identify(&sector(&[0xFF, 0xD8, 0xFF, 0xE1])).unwrap().ext,
            "jpg"
        );
        assert_eq!(identify(&sector(b"\0PSF\x01\x01")).unwrap().ext, "sfo");
        assert_eq!(identify(&sector(b"\0\0\0\x18ftypmp42")).unwrap().ext, "mp4");
        assert_eq!(
            identify(&sector(b"RIFF\x10\0\0\0WAVEfmt ")).unwrap().ext,
            "wav"
        );
        assert_eq!(
            identify(&sector(b"RIFF\x10\0\0\0AVI LIST")).unwrap().ext,
            "avi"
        );
    }

    #[test]
    fn photorec_sig_file_is_up_to_date() {
        assert_eq!(
            include_str!("../photorec.sig"),
            photorec_sig(),
            "photorec.sig is out of date: run `cargo run -- photorec-sig > photorec.sig`"
        );
    }

    #[test]
    fn empty_sector_matches_nothing() {
        assert!(identify(&[0u8; 512]).is_none());
    }

    #[test]
    fn riff_reports_exact_size() {
        assert_eq!(riff_size(b"RIFF\x24\0\0\0WAVE"), Some(0x24 + 8));
    }

    #[test]
    fn loose_bmp_header_is_rejected() {
        assert!(identify(&sector(b"BMxxxxxxxxxxxx")).is_none());
    }
}
