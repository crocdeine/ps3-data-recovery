//! Decryption of the PS3 internal hard drive.
//!
//! How the PS3 protects its drive:
//! 1. Two drive keys are derived from the console's `eid_root_key`: each is a
//!    fixed 32-byte seed encrypted with AES-256-CBC, using the first 32 bytes
//!    of the `eid_root_key` as the key and its last 16 bytes as the IV.
//! 2. Every 512-byte sector is encrypted on its own:
//!    - Fat models (CECHAxx to CECHQxx): AES-192-CBC with the first 24 bytes
//!      of the data key and an all-zero IV (dm-crypt `aes-cbc-null`).
//!    - Slim and Super Slim models (CECH-2xxx and later): AES-128-XTS with the
//!      first 16 bytes of the data key and of the tweak key, the tweak being
//!      the sector number in little-endian (dm-crypt `aes-xts-plain64`).
//! 3. The encrypted sectors are stored with every pair of bytes swapped.
//!
//! Sources: the seeds and dm-crypt settings in `ps3hdd_keygen.sh`
//! (github.com/einsteinx2/PS3-Reclaim-HDD-Space), which match the seeds on
//! psdevwiki.com/ps3/Talk:HDD_Encryption, and the bswap16-then-XTS order in
//! the PS3 Linux drive decryption notes (gist.github.com/andshrew).
//!
//! A correctly decrypted drive starts with the partition table magic
//! `0FACE0FF` at offset 0x14 and `DEADFACE` at offset 0x1C. This is used to
//! check the key and to detect the console model automatically.

use std::fs::{self, File};
use std::io::{BufWriter, ErrorKind, Read, Write};
use std::path::Path;

use aes::cipher::{BlockDecryptMut, BlockEncryptMut, InnerIvInit, KeyInit, KeyIvInit};
use aes::{Aes128, Aes192, Aes256};
use anyhow::{bail, ensure, Context, Result};
use clap::ValueEnum;
use rayon::prelude::*;
use xts_mode::{get_tweak_default, Xts128};

/// Size of the `eid_root_key` file: a 32-byte key followed by a 16-byte IV.
pub const EID_ROOT_KEY_LEN: usize = 48;
pub const SECTOR: usize = 512;
const READ_CHUNK: usize = 8 * 1024 * 1024;

const ATA_DATA_KEY_SEED: [u8; 32] =
    hex32("D92D65DB057D49E1A66F2274B8BAC50883844ED756CA79516362EA8ADAC60326");
const ATA_TWEAK_KEY_SEED: [u8; 32] =
    hex32("C3B3B5AACC74CD6A48EFABF44DCDF16E379F55F5777D09FBEEDE07058E94BE08");

const TABLE_MAGIC_1: [u8; 4] = [0x0F, 0xAC, 0xE0, 0xFF];
const TABLE_MAGIC_2: [u8; 4] = [0xDE, 0xAD, 0xFA, 0xCE];

/// Which family of console the drive comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Model {
    /// Fat PS3 (CECHAxx to CECHQxx): AES-192-CBC.
    Fat,
    /// Slim or Super Slim PS3 (CECH-2xxx, 3xxx, 4xxx): AES-128-XTS.
    Slim,
}

impl Model {
    pub fn describe(self) -> &'static str {
        match self {
            Model::Fat => "Fat PS3 (AES-192-CBC)",
            Model::Slim => "Slim / Super Slim PS3 (AES-128-XTS)",
        }
    }
}

/// The two drive keys derived from an `eid_root_key`.
#[derive(Clone, PartialEq, Eq)]
pub struct DriveKeys {
    pub data: [u8; 32],
    pub tweak: [u8; 32],
}

impl DriveKeys {
    pub fn derive(eid_root_key: &[u8; EID_ROOT_KEY_LEN]) -> DriveKeys {
        let (key, iv) = eid_root_key.split_at(32);
        let encrypt_seed = |seed: &[u8; 32]| {
            let mut out = *seed;
            cbc::Encryptor::<Aes256>::new(key.into(), iv.into())
                .encrypt_padded_mut::<aes::cipher::block_padding::NoPadding>(&mut out, 32)
                .expect("seed is a whole number of blocks");
            out
        };
        DriveKeys {
            data: encrypt_seed(&ATA_DATA_KEY_SEED),
            tweak: encrypt_seed(&ATA_TWEAK_KEY_SEED),
        }
    }
}

/// Decrypts sectors for one console model.
pub enum SectorCipher {
    Fat(Box<Aes192>),
    Slim(Box<Xts128<Aes128>>),
}

impl SectorCipher {
    pub fn new(model: Model, keys: &DriveKeys) -> SectorCipher {
        match model {
            Model::Fat => SectorCipher::Fat(Box::new(Aes192::new(keys.data[..24].into()))),
            Model::Slim => SectorCipher::Slim(Box::new(Xts128::new(
                Aes128::new(keys.data[..16].into()),
                Aes128::new(keys.tweak[..16].into()),
            ))),
        }
    }

    /// Decrypt whole sectors in place. `data` holds raw sectors as read from
    /// the drive (still byte-swapped); `first_sector` is the number of the
    /// first one on the drive.
    pub fn decrypt(&self, data: &mut [u8], first_sector: u64) {
        assert_eq!(data.len() % SECTOR, 0, "data must be whole sectors");
        data.par_chunks_mut(SECTOR)
            .enumerate()
            .for_each(|(i, sector)| {
                swap_byte_pairs(sector);
                self.decrypt_sector(sector, first_sector + i as u64);
            });
    }

    fn decrypt_sector(&self, sector: &mut [u8], number: u64) {
        match self {
            SectorCipher::Fat(aes) => {
                cbc::Decryptor::<Aes192>::inner_iv_init((**aes).clone(), &[0u8; 16].into())
                    .decrypt_padded_mut::<aes::cipher::block_padding::NoPadding>(sector)
                    .expect("sector is a whole number of blocks");
            }
            SectorCipher::Slim(xts) => {
                xts.decrypt_sector(sector, get_tweak_default(u128::from(number)));
            }
        }
    }
}

/// Swap every pair of bytes (the PS3 stores drive data 16-bit byte-swapped).
fn swap_byte_pairs(data: &mut [u8]) {
    for pair in data.chunks_exact_mut(2) {
        pair.swap(0, 1);
    }
}

/// Does a decrypted first sector look like a PS3 partition table?
pub fn has_partition_magic(sector: &[u8]) -> bool {
    sector.get(0x14..0x18) == Some(&TABLE_MAGIC_1) && sector.get(0x1C..0x20) == Some(&TABLE_MAGIC_2)
}

/// Find which model's cipher turns `first_sector` into a valid partition table.
pub fn detect_model(
    first_sector: &[u8; SECTOR],
    keys: &DriveKeys,
    candidates: &[Model],
) -> Option<Model> {
    candidates.iter().copied().find(|&model| {
        let mut sector = *first_sector;
        SectorCipher::new(model, keys).decrypt(&mut sector, 0);
        has_partition_magic(&sector)
    })
}

/// Read and check the `eid_root_key` file.
pub fn read_key(path: &Path) -> Result<[u8; EID_ROOT_KEY_LEN]> {
    let bytes =
        fs::read(path).with_context(|| format!("cannot read the key file {}", path.display()))?;
    bytes.as_slice().try_into().map_err(|_| {
        anyhow::anyhow!(
            "the key file {} is {} bytes long, but an eid_root_key is exactly {} bytes",
            path.display(),
            bytes.len(),
            EID_ROOT_KEY_LEN
        )
    })
}

/// Decrypt a whole drive image. `model` forces a console model; `None`
/// detects it. Returns the model used.
pub fn run(image: &Path, output: &Path, key: &Path, model: Option<Model>) -> Result<Model> {
    let keys = DriveKeys::derive(&read_key(key)?);
    let mut input =
        File::open(image).with_context(|| format!("cannot open the image {}", image.display()))?;
    let total = input.metadata()?.len();
    ensure!(
        total >= SECTOR as u64 && total % SECTOR as u64 == 0,
        "the image {} is {total} bytes long, which is not a whole number of 512-byte sectors; \
         make sure it is a raw copy of the entire drive",
        image.display()
    );
    if output.exists() {
        bail!(
            "{} already exists; choose another output file or delete it first",
            output.display()
        );
    }

    let mut first = [0u8; SECTOR];
    input.read_exact(&mut first)?;
    let candidates = match model {
        Some(m) => vec![m],
        None => vec![Model::Slim, Model::Fat],
    };
    let Some(model) = detect_model(&first, &keys, &candidates) else {
        bail!(
            "the image could not be decrypted with this key{}. Check that the key \
             comes from the same console as the drive, and that the image is a \
             copy of the whole drive starting at its first sector",
            match model {
                Some(m) => format!(" as a {}", m.describe()),
                None => " (tried both Fat and Slim models)".to_string(),
            }
        );
    };
    println!("Key accepted: drive detected as {}", model.describe());

    let cipher = SectorCipher::new(model, &keys);
    let mut input = File::open(image)?;
    let mut out = BufWriter::new(
        File::create(output).with_context(|| format!("cannot create {}", output.display()))?,
    );
    let mut buf = vec![0u8; READ_CHUNK];
    let mut done: u64 = 0;
    let mut last_percent = None;
    while done < total {
        let filled = read_full(&mut input, &mut buf)?;
        if filled == 0 {
            break;
        }
        cipher.decrypt(&mut buf[..filled], done / SECTOR as u64);
        out.write_all(&buf[..filled])
            .with_context(|| format!("cannot write to {} (is the disk full?)", output.display()))?;
        done += filled as u64;
        let percent = done * 100 / total;
        if last_percent != Some(percent) {
            last_percent = Some(percent);
            eprint!("\rDecrypting: {percent:3}% ({} MiB)", done / (1024 * 1024));
        }
    }
    out.flush()?;
    eprintln!();
    println!("Decryption complete: {}", output.display());
    Ok(model)
}

fn read_full<R: Read>(reader: &mut R, buf: &mut [u8]) -> std::io::Result<usize> {
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

const fn hex32(s: &str) -> [u8; 32] {
    const fn nibble(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'A'..=b'F' => c - b'A' + 10,
            b'a'..=b'f' => c - b'a' + 10,
            _ => panic!("invalid hex digit"),
        }
    }
    let s = s.as_bytes();
    assert!(s.len() == 64);
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = nibble(s[2 * i]) << 4 | nibble(s[2 * i + 1]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE_KEY: &[u8; 48] = include_bytes!("../tests/fixtures/fake_eid_root_key.bin");

    #[test]
    fn derives_the_same_keys_as_openssl() {
        // Expected values computed with `openssl aes-256-cbc -nopad` (see
        // tests/fixtures/make_fixtures.py).
        let keys = DriveKeys::derive(FAKE_KEY);
        assert_eq!(
            keys.data,
            hex32("fd3de2e9866043225a53452f71a2e5d8dedd6deadd6a4749c9adf952c2638ae5")
        );
        assert_eq!(
            keys.tweak,
            hex32("3915f1c9277d50d48f4281a142653d14b2835ce759a38f7428ca7003efd43199")
        );
    }

    #[test]
    fn rejects_key_of_wrong_length() {
        let dir = tempfile::tempdir().unwrap();
        let key = dir.path().join("eid_root_key");
        fs::write(&key, [0u8; 47]).unwrap();
        let err = read_key(&key).unwrap_err().to_string();
        assert!(err.contains("47 bytes"), "{err}");
    }

    #[test]
    fn byte_swap_swaps_pairs() {
        let mut d = [1, 2, 3, 4];
        swap_byte_pairs(&mut d);
        assert_eq!(d, [2, 1, 4, 3]);
    }
}
