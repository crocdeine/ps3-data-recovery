# PS3 Data Recovery

[![GitHub stars](https://img.shields.io/github/stars/crocdeine/ps3-data-recovery.svg?style=social&label=Star)](https://github.com/crocdeine/ps3-data-recovery)

This project provides a complete workflow to recover deleted files from a formatted internal PS3 hard drive.  
It includes tools and documentation to decrypt the drive and run file carving with a custom PhotoRec signature set or custom Python scripts.

## Overview

The PlayStation 3 uses encrypted internal hard drives. When a drive is formatted (e.g. via a system reinstall), data is not truly erased, but becomes inaccessible without proper decryption.

This project helps you:
1. Decrypt a full disk image using your console's `eid_root_key`.
2. Analyze or recover user data using `PhotoRec` or custom scripts.

## Features

- PS3 disk image decryption (AES-CBC/ECB with correct key and offset).
- Custom `photorec.sig` file with tailored PS3 file signatures.
- Alternative Python carving script for unsupported use cases.
- File sorting script to organize recovered files by type.
- File verification tool to detect and separate corrupted files.
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
  - `ffmpeg`, `python3`, `pip`, `Pillow` (for Python verification script)

### 📦 Python Dependencies

To install the required Python packages for the scripts in the `scripts/` folder, run:

```bash
pip install -r requirements.txt
```

This will install all dependencies needed for:

- `carving_script.py`: File carving using custom logic.
- `verification_script.py`: Integrity check of recovered files (images, videos, audio, text, and archives).
- `sort_recovered_files.py`: Sort files recovered by PhotoRec or carving into categorized folders.

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

### 3.bis Recover deleted files with Python script

Use the `carving_script.py` located in the `script/` folder if PhotoRec does not properly use the `.sig` file.
This script performs basic signature-based carving manually on the decrypted image.

### 4. Sort recovered files

To automatically categorize recovered files by type, use the `sort_recovered_files.py` script:

```bash
python3 script/sort_recovered_files.py
```

This will place files into subdirectories inside a `organized/` folder.

### 5. Verify recovered files

Once files are organized, use the `verification_script.py` to scan all categorized files.
Corrupted files will be automatically moved to a `corrupted/` subfolder in their respective categories.

Run with:
```bash
python3 script/verification_script.py
```

## Files

- `ps3_dump_decrypted`: Decryption binary (must be compiled)
- `photorec.sig`: Custom signatures for PS3 file types
- `decrypt/`: Scripts to help test offsets or check encryption mode
- `script/carving_script.py`: Alternative file carving script in Python
- `script/sort_recovered_files.py`: Script to organize recovered files by type
- `script/verification_script.py`: Script to verify integrity of recovered files

## Warning

🚫 **Do NOT share your `eid_root_key`.** It is unique to your console and should remain private.

---

## License

MIT License.
