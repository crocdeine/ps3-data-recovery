# PS3 Data Recovery

This project provides a complete workflow to recover deleted files from a formatted internal PS3 hard drive.  
It includes tools and documentation to decrypt the drive and run file carving with a custom PhotoRec signature set.

## Overview

The PlayStation 3 uses encrypted internal hard drives. When a drive is formatted (e.g. via a system reinstall), data is not truly erased, but becomes inaccessible without proper decryption.

This project helps you:
1. Decrypt a full disk image using your console's `eid_root_key`.
2. Analyze or recover user data using `PhotoRec`.

## Features

- PS3 disk image decryption (AES-CBC/ECB with correct key and offset).
- Custom `photorec.sig` file with tailored PS3 file signatures.
- Clean CLI workflow and documentation.

---

## Requirements

- A full raw image of your PS3 internal drive (e.g. `ps3_image.img`).
- Your console's `eid_root_key` (not provided).
- Linux system (recommended).
- Dependencies:
  - `gcc`, `make`
  - `xxd`, `hexdump` (for CLI analysis)
  - `PhotoRec` (part of TestDisk)
  - `openssl` (used internally for AES)

## Usage

### 1. Build the decryption tool

```bash
cd ps3-hdd-reader-linux
make
```

### 2. Decrypt your image

```bash
sudo ./ps3_dump_decrypted   -m cbc   -o 0   "/path/to/ps3_image.img"   "/path/to/ps3_decrypted.img"   "./eid_root_key"
```

- `-m`: AES mode (`cbc` or `ecb`)
- `-o`: offset (default 0)
- Provide correct paths accordingly.

> Ensure the `eid_root_key` is 48 bytes and in binary format.

### 3. Recover deleted files with PhotoRec

Run PhotoRec using the custom signature file:

```bash
photorec /d recovery_output /sig photorec.sig /cmd ps3_decrypted.img
```

## Files

- `ps3_dump_decrypted`: Decryption binary (must be compiled)
- `photorec.sig`: Custom signatures for PS3 file types
- `tools/`: Scripts to help test offsets or check encryption mode

## Warning

🚫 **Do NOT share your `eid_root_key`.** It is unique to your console and should remain private.

---

## License

MIT License.
