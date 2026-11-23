//! Decryption of the PS3 internal drive.
//!
//! The previous C program only XOR-ed the image with the key, which is not how
//! the PS3 encrypts its drive, so its output was never readable. The real
//! decryption (deriving the drive keys from the `eid_root_key`, then AES with
//! the console's sector layout) is planned for the next step of the rewrite.
//! Until then this command checks its inputs and stops with a clear message,
//! rather than producing a file that looks decrypted but is not.

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

/// Size of the `eid_root_key` file: a 32-byte key followed by a 16-byte IV.
pub const EID_ROOT_KEY_LEN: usize = 48;

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

pub fn run(image: &Path, _output: &Path, key: &Path) -> Result<()> {
    if !image.is_file() {
        bail!("the disk image {} does not exist", image.display());
    }
    read_key(key)?;
    bail!(
        "decryption is not implemented yet: it is being rewritten so that it \
         matches how the PS3 really encrypts its drive. The previous version \
         produced unreadable output, so it has been removed."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_key_of_wrong_length() {
        let dir = tempfile::tempdir().unwrap();
        let key = dir.path().join("eid_root_key");
        fs::write(&key, [0u8; 47]).unwrap();
        let err = read_key(&key).unwrap_err().to_string();
        assert!(err.contains("47 bytes"), "{err}");
    }

    #[test]
    fn accepts_48_byte_key() {
        let dir = tempfile::tempdir().unwrap();
        let key = dir.path().join("eid_root_key");
        fs::write(&key, [7u8; 48]).unwrap();
        assert_eq!(read_key(&key).unwrap(), [7u8; 48]);
    }
}
