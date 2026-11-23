//! Finding where a file ends.
//!
//! Each function here reads a file from its first byte, following the
//! structure of its format, and returns the file's exact length. It returns
//! `None` when the data stops making sense for that format (for example
//! because the file is damaged or fragmented), so that carving can fall back
//! to a cruder method.

use std::io::{self, BufRead, BufReader, Read, Seek};

/// Reads one candidate file, never past `limit` bytes.
pub struct Src<'a> {
    inner: &'a mut dyn SrcIo,
    pos: u64,
    limit: u64,
}

pub trait SrcIo {
    fn read_exact_(&mut self, buf: &mut [u8]) -> io::Result<()>;
    fn skip(&mut self, n: u64) -> io::Result<()>;
    fn peek_byte(&mut self) -> io::Result<Option<u8>>;
}

impl<R: Read + Seek> SrcIo for BufReader<R> {
    fn read_exact_(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.read_exact(buf)
    }
    fn skip(&mut self, n: u64) -> io::Result<()> {
        let n = i64::try_from(n).map_err(|_| io::Error::other("skip too large"))?;
        self.seek_relative(n)
    }
    fn peek_byte(&mut self) -> io::Result<Option<u8>> {
        Ok(self.fill_buf()?.first().copied())
    }
}

/// Result of a read: `None` means "past the limit or the end of the image".
type R<T> = Option<T>;

impl<'a> Src<'a> {
    pub fn new(inner: &'a mut dyn SrcIo, limit: u64) -> Src<'a> {
        Src {
            inner,
            pos: 0,
            limit,
        }
    }

    /// Bytes read so far, which is the file length once its end is found.
    pub fn pos(&self) -> u64 {
        self.pos
    }

    fn fits(&self, n: u64) -> bool {
        self.pos.checked_add(n).is_some_and(|end| end <= self.limit)
    }

    fn bytes<const N: usize>(&mut self) -> R<[u8; N]> {
        let mut b = [0u8; N];
        self.fill(&mut b)?;
        Some(b)
    }

    fn fill(&mut self, buf: &mut [u8]) -> R<()> {
        if !self.fits(buf.len() as u64) {
            return None;
        }
        self.inner.read_exact_(buf).ok()?;
        self.pos += buf.len() as u64;
        Some(())
    }

    fn u8(&mut self) -> R<u8> {
        Some(self.bytes::<1>()?[0])
    }
    fn u16be(&mut self) -> R<u16> {
        Some(u16::from_be_bytes(self.bytes()?))
    }
    fn u32be(&mut self) -> R<u32> {
        Some(u32::from_be_bytes(self.bytes()?))
    }
    fn u64be(&mut self) -> R<u64> {
        Some(u64::from_be_bytes(self.bytes()?))
    }
    fn u32le(&mut self) -> R<u32> {
        Some(u32::from_le_bytes(self.bytes()?))
    }

    fn skip(&mut self, n: u64) -> R<()> {
        if !self.fits(n) {
            return None;
        }
        self.inner.skip(n).ok()?;
        self.pos += n;
        Some(())
    }

    /// `size` if a file of that size fits within the limit.
    fn within(&self, size: u64) -> Option<u64> {
        (size <= self.limit).then_some(size)
    }

    /// The next byte without consuming it; `None` at the limit.
    fn peek(&mut self) -> R<u8> {
        if !self.fits(1) {
            return None;
        }
        self.inner.peek_byte().ok()?
    }
}

/// Signature of an end finder.
pub type EndFn = fn(&mut Src) -> Option<u64>;

// ---------------------------------------------------------------- images

/// JPEG: walk the segments; inside image data, look for the end marker FFD9.
pub fn jpeg(s: &mut Src) -> Option<u64> {
    if s.bytes::<2>()? != [0xFF, 0xD8] {
        return None;
    }
    let mut marker = next_marker(s)?;
    loop {
        match marker {
            0xD9 => return Some(s.pos()),
            0x01 | 0xD0..=0xD7 => marker = next_marker(s)?,
            0xDA => {
                let len = s.u16be()?;
                s.skip(u64::from(len).checked_sub(2)?)?;
                marker = scan_entropy(s)?;
            }
            0xC0..=0xFE => {
                let len = s.u16be()?;
                s.skip(u64::from(len).checked_sub(2)?)?;
                marker = next_marker(s)?;
            }
            _ => return None,
        }
    }
}

fn next_marker(s: &mut Src) -> Option<u8> {
    if s.u8()? != 0xFF {
        return None;
    }
    let mut m = s.u8()?;
    while m == 0xFF {
        m = s.u8()?;
    }
    Some(m)
}

/// Skip compressed image data and return the marker that ends it.
fn scan_entropy(s: &mut Src) -> Option<u8> {
    loop {
        if s.u8()? != 0xFF {
            continue;
        }
        let mut b = s.u8()?;
        while b == 0xFF {
            b = s.u8()?;
        }
        match b {
            0x00 | 0xD0..=0xD7 => continue,
            m => return Some(m),
        }
    }
}

/// PNG: walk the chunks up to IEND.
pub fn png(s: &mut Src) -> Option<u64> {
    if &s.bytes::<8>()? != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    loop {
        let len = s.u32be()?;
        let kind = s.bytes::<4>()?;
        if len > 0x7FFF_FFFF || !kind.iter().all(u8::is_ascii_alphabetic) {
            return None;
        }
        s.skip(u64::from(len) + 4)?; // data + CRC
        if &kind == b"IEND" {
            return Some(s.pos());
        }
    }
}

/// GIF: walk the blocks up to the trailer byte 0x3B.
pub fn gif(s: &mut Src) -> Option<u64> {
    let header = s.bytes::<6>()?;
    if &header != b"GIF87a" && &header != b"GIF89a" {
        return None;
    }
    let screen = s.bytes::<7>()?;
    skip_color_table(s, screen[4])?;
    loop {
        match s.u8()? {
            0x3B => return Some(s.pos()),
            0x2C => {
                let desc = s.bytes::<9>()?;
                skip_color_table(s, desc[8])?;
                s.u8()?; // LZW minimum code size
                skip_sub_blocks(s)?;
            }
            0x21 => {
                s.u8()?; // extension label
                skip_sub_blocks(s)?;
            }
            _ => return None,
        }
    }
}

fn skip_color_table(s: &mut Src, flags: u8) -> Option<()> {
    if flags & 0x80 != 0 {
        s.skip(3 << ((flags & 7) + 1))?;
    }
    Some(())
}

fn skip_sub_blocks(s: &mut Src) -> Option<()> {
    loop {
        match s.u8()? {
            0 => return Some(()),
            n => s.skip(u64::from(n))?,
        }
    }
}

/// BMP: the size is in the header.
pub fn bmp(s: &mut Src) -> Option<u64> {
    let h = s.bytes::<6>()?;
    if &h[..2] != b"BM" {
        return None;
    }
    let size = u64::from(u32::from_le_bytes(h[2..6].try_into().ok()?));
    (size >= 26).then(|| s.within(size))?
}

// ---------------------------------------------------------------- audio / video

/// RIFF (WAV, AT3, AVI): the size is in the header.
pub fn riff(s: &mut Src) -> Option<u64> {
    if &s.bytes::<4>()? != b"RIFF" {
        return None;
    }
    let size = u64::from(s.u32le()?) + 8;
    (size >= 12).then(|| s.within(size))?
}

const MP4_BOXES: &[&[u8; 4]] = &[
    b"ftyp", b"moov", b"mdat", b"free", b"skip", b"wide", b"uuid", b"meta", b"pdin", b"moof",
    b"mfra", b"sidx", b"styp", b"udta", b"pnot", b"prfl", b"junk",
];

/// MP4 / MOV: walk the top-level boxes until one is not a known box.
pub fn mp4(s: &mut Src) -> Option<u64> {
    let mut boxes = 0;
    loop {
        let start = s.pos();
        let Some(size) = s.u32be() else {
            return (boxes >= 2).then_some(start);
        };
        let kind = s.bytes::<4>();
        let Some(kind) = kind.filter(|k| MP4_BOXES.contains(&k)) else {
            return (boxes >= 2).then_some(start);
        };
        if boxes == 0 && &kind != b"ftyp" {
            return None;
        }
        let size = match size {
            0 => return None, // "until the end of the file": unknown
            1 => s.u64be()?.checked_sub(16)?,
            n => u64::from(n).checked_sub(8)?,
        };
        s.skip(size)?;
        boxes += 1;
        if s.peek().is_none() {
            return Some(s.pos());
        }
    }
}

/// M2TS (PS3 video recordings): 192-byte packets, each with 0x47 at offset 4.
pub fn m2ts(s: &mut Src) -> Option<u64> {
    let mut packets = 0u64;
    loop {
        let start = s.pos();
        match s.bytes::<192>() {
            Some(p) if p[4] == 0x47 => packets += 1,
            _ => return (packets >= 3).then_some(start),
        }
    }
}

/// MP3: skip the ID3 tag, then walk the MPEG audio frames.
pub fn mp3(s: &mut Src) -> Option<u64> {
    let tag = s.bytes::<10>()?;
    if &tag[..3] != b"ID3" {
        return None;
    }
    let syncsafe = tag[6..10].iter().try_fold(0u64, |acc, &b| {
        (b < 0x80).then_some(acc << 7 | u64::from(b))
    })?;
    s.skip(syncsafe + if tag[5] & 0x10 != 0 { 10 } else { 0 })?;
    let mut frames = 0u64;
    loop {
        let start = s.pos();
        let Some(header) = s.bytes::<4>() else {
            return (frames > 0).then_some(start);
        };
        if &header[..3] == b"TAG" {
            return s.skip(124).map(|()| s.pos()).or(Some(start));
        }
        match mp3_frame_len(header) {
            Some(len) if s.skip(len - 4).is_some() => frames += 1,
            _ => return (frames > 0).then_some(start),
        }
    }
}

/// Length of an MPEG audio Layer III frame from its 4-byte header.
fn mp3_frame_len(h: [u8; 4]) -> Option<u64> {
    if h[0] != 0xFF || h[1] & 0xE0 != 0xE0 || (h[1] >> 1) & 3 != 1 {
        return None; // not a frame sync, or not Layer III
    }
    let version = (h[1] >> 3) & 3; // 3 = MPEG1, 2 = MPEG2, 0 = MPEG2.5
    const BITRATE_V1: [u64; 16] = [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
    ];
    const BITRATE_V2: [u64; 16] = [
        0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
    ];
    let (bitrates, rates, factor): (&[u64; 16], [u64; 3], u64) = match version {
        3 => (&BITRATE_V1, [44100, 48000, 32000], 144),
        2 => (&BITRATE_V2, [22050, 24000, 16000], 72),
        0 => (&BITRATE_V2, [11025, 12000, 8000], 72),
        _ => return None,
    };
    let bitrate = bitrates[usize::from(h[2] >> 4)] * 1000;
    let rate = *rates.get(usize::from((h[2] >> 2) & 3))?;
    let padding = u64::from((h[2] >> 1) & 1);
    (bitrate > 0).then(|| factor * bitrate / rate + padding)
}

// ---------------------------------------------------------------- PS3 files

/// PARAM.SFO: the end of the data table follows from the entry table.
pub fn sfo(s: &mut Src) -> Option<u64> {
    if &s.bytes::<4>()? != b"\0PSF" {
        return None;
    }
    s.skip(4)?; // version
    let key_table = u64::from(s.u32le()?);
    let data_table = u64::from(s.u32le()?);
    let entries = s.u32le()?;
    if entries == 0 || entries > 1024 || key_table < 0x14 || data_table < key_table {
        return None;
    }
    let mut end = data_table;
    for _ in 0..entries {
        let e = s.bytes::<16>()?;
        let max_len = u64::from(u32::from_le_bytes(e[8..12].try_into().ok()?));
        let offset = u64::from(u32::from_le_bytes(e[12..16].try_into().ok()?));
        end = end.max(data_table + offset + max_len);
    }
    s.within(end)
}

/// PKG: the total size is at offset 0x18.
pub fn pkg(s: &mut Src) -> Option<u64> {
    if &s.bytes::<4>()? != b"\x7FPKG" {
        return None;
    }
    s.skip(0x14)?;
    let size = s.u64be()?;
    (size >= 0x20).then(|| s.within(size))?
}

/// SELF / SPRX: header length (0x10) plus data length (0x18).
pub fn self_file(s: &mut Src) -> Option<u64> {
    if &s.bytes::<4>()? != b"SCE\0" {
        return None;
    }
    s.skip(12)?;
    let header = s.u64be()?;
    let data = s.u64be()?;
    let size = header.checked_add(data)?;
    (header >= 0x20).then(|| s.within(size))?
}

// ---------------------------------------------------------------- documents and archives

/// XML text: ends at the first zero byte.
pub fn text(s: &mut Src) -> Option<u64> {
    loop {
        let start = s.pos();
        match s.u8() {
            Some(0) | None => return (start > 0).then_some(start),
            Some(_) => {}
        }
    }
}

/// ZIP: ends after the "end of central directory" record and its comment.
pub fn zip(s: &mut Src) -> Option<u64> {
    let mut window = 0u32;
    loop {
        window = window << 8 | u32::from(s.u8()?);
        if window == 0x504B_0506 {
            s.skip(16)?;
            let comment = u64::from(u16::from_le_bytes(s.bytes()?));
            s.skip(comment)?;
            return Some(s.pos());
        }
    }
}

/// TAR: walk the 512-byte headers up to the two empty blocks at the end.
pub fn tar(s: &mut Src) -> Option<u64> {
    let mut members = 0;
    loop {
        let block = s.bytes::<512>()?;
        if block.iter().all(|&b| b == 0) {
            // The archive ends with two zero blocks, but some tools write one.
            let after_one = s.pos();
            let second_is_zero = s.bytes::<512>().is_some_and(|b| b.iter().all(|&x| x == 0));
            let end = if second_is_zero { s.pos() } else { after_one };
            return (members > 0).then_some(end);
        }
        if &block[257..262] != b"ustar" {
            return None;
        }
        let size_field = std::str::from_utf8(&block[124..136]).ok()?;
        let size = u64::from_str_radix(size_field.trim_matches(|c: char| c == '\0' || c == ' '), 8)
            .ok()?;
        s.skip(size.div_ceil(512) * 512)?;
        members += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    /// Length found for `data` followed by `trailer` (unrelated bytes).
    fn end_of(f: EndFn, data: &[u8], trailer: &[u8]) -> Option<u64> {
        let mut all = data.to_vec();
        all.extend_from_slice(trailer);
        let mut reader = BufReader::new(Cursor::new(all.clone()));
        f(&mut Src::new(&mut reader, all.len() as u64))
    }

    const NOISE: &[u8] = &[
        0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44,
    ];

    fn encode(format: image::ImageFormat) -> Vec<u8> {
        let img = image::RgbImage::from_fn(37, 23, |x, y| {
            image::Rgb([(x * 7) as u8, (y * 11) as u8, (x ^ y) as u8])
        });
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, format).unwrap();
        out.into_inner()
    }

    #[test]
    fn finds_end_of_images() {
        for (f, fmt) in [
            (jpeg as EndFn, image::ImageFormat::Jpeg),
            (png, image::ImageFormat::Png),
            (gif, image::ImageFormat::Gif),
            (bmp, image::ImageFormat::Bmp),
        ] {
            let data = encode(fmt);
            assert_eq!(end_of(f, &data, NOISE), Some(data.len() as u64), "{fmt:?}");
            // A cut-off file is not given a length.
            assert_eq!(
                end_of(f, &data[..data.len() / 2], &[]),
                None,
                "{fmt:?} truncated"
            );
        }
    }

    #[test]
    fn jpeg_with_embedded_thumbnail_ends_at_outer_image() {
        let thumb = encode(image::ImageFormat::Jpeg);
        let main = encode(image::ImageFormat::Jpeg);
        // Put the thumbnail inside an APP1 segment, as EXIF does.
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xE1];
        data.extend(((thumb.len() + 2) as u16).to_be_bytes());
        data.extend(&thumb);
        data.extend(&main[2..]);
        assert_eq!(end_of(jpeg, &data, NOISE), Some(data.len() as u64));
    }

    #[test]
    fn finds_end_of_mp4() {
        let mut data = Vec::new();
        for (kind, body) in [
            (b"ftyp", &b"isom\0\0\0\0"[..]),
            (b"mdat", &[7u8; 300][..]),
            (b"moov", &[1u8; 40][..]),
        ] {
            data.extend(((body.len() + 8) as u32).to_be_bytes());
            data.extend(kind);
            data.extend(body);
        }
        assert_eq!(end_of(mp4, &data, NOISE), Some(data.len() as u64));
        assert_eq!(end_of(mp4, &data, &[]), Some(data.len() as u64));
    }

    #[test]
    fn finds_end_of_m2ts() {
        let mut data = Vec::new();
        for _ in 0..5 {
            let mut p = [0u8; 192];
            p[4] = 0x47;
            data.extend(p);
        }
        assert_eq!(end_of(m2ts, &data, &[0u8; 192]), Some(data.len() as u64));
    }

    #[test]
    fn finds_end_of_mp3() {
        let mut data = b"ID3\x03\0\0\0\0\0\x0a".to_vec();
        data.extend([0u8; 10]); // tag body
                                // MPEG1 Layer III, 128 kbit/s, 44.1 kHz, no padding: 417 bytes per frame.
        for _ in 0..3 {
            let mut frame = vec![0xFF, 0xFB, 0x90, 0x00];
            frame.resize(417, 0x55);
            data.extend(frame);
        }
        assert_eq!(end_of(mp3, &data, NOISE), Some(data.len() as u64));
    }

    #[test]
    fn finds_end_of_riff() {
        let mut data = b"RIFF\x0c\0\0\0WAVEdata\0\0\0\0".to_vec();
        data.truncate(20);
        assert_eq!(end_of(riff, &data, NOISE), Some(20));
    }

    #[test]
    fn finds_end_of_sfo() {
        // One entry: key "TITLE" with a 16-byte data slot.
        let mut data = b"\0PSF\x01\x01\0\0".to_vec();
        data.extend(0x24u32.to_le_bytes()); // key table
        data.extend(0x2Cu32.to_le_bytes()); // data table
        data.extend(1u32.to_le_bytes());
        data.extend([0, 0, 4, 2]); // key offset, format
        data.extend(5u32.to_le_bytes()); // len
        data.extend(16u32.to_le_bytes()); // max len
        data.extend(0u32.to_le_bytes()); // data offset
        data.extend(b"TITLE\0\0\0");
        data.extend(b"Hello\0\0\0\0\0\0\0\0\0\0\0");
        assert_eq!(data.len(), 0x2C + 16);
        assert_eq!(end_of(sfo, &data, NOISE), Some(data.len() as u64));
    }

    #[test]
    fn finds_end_of_ps3_headers() {
        let mut pkg_data = b"\x7FPKG".to_vec();
        pkg_data.resize(0x18, 0);
        pkg_data.extend(0x100u64.to_be_bytes());
        assert_eq!(end_of(pkg, &pkg_data, &[0u8; 0x100]), Some(0x100));

        let mut self_data = b"SCE\0".to_vec();
        self_data.resize(0x10, 0);
        self_data.extend(0x80u64.to_be_bytes());
        self_data.extend(0x40u64.to_be_bytes());
        assert_eq!(end_of(self_file, &self_data, &[0u8; 0x100]), Some(0xC0));
    }

    #[test]
    fn finds_end_of_archives() {
        let mut z = Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut z);
            w.start_file("a.txt", zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(&[b'x'; 5000]).unwrap();
            w.finish().unwrap();
        }
        let z = z.into_inner();
        assert_eq!(end_of(zip, &z, NOISE), Some(z.len() as u64));

        let mut t = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_ustar();
        header.set_size(700);
        header.set_cksum();
        t.append_data(&mut header, "a.bin", &[9u8; 700][..])
            .unwrap();
        let t = t.into_inner().unwrap();
        assert_eq!(end_of(tar, &t, NOISE), Some(t.len() as u64));
    }

    #[test]
    fn finds_end_of_text() {
        assert_eq!(
            end_of(text, b"<?xml version=\"1.0\"?><a/>", &[0, 0, 1]),
            Some(25)
        );
    }
}
