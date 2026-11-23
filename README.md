# PS3 Data Recovery

[![GitHub stars](https://img.shields.io/github/stars/crocdeine/ps3-data-recovery.svg?style=social&label=Star)](https://github.com/crocdeine/ps3-data-recovery)
[![GitHub forks](https://img.shields.io/github/forks/crocdeine/ps3-data-recovery.svg?style=social&label=Fork)](https://github.com/crocdeine/ps3-data-recovery/fork)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub issues](https://img.shields.io/github/issues/crocdeine/ps3-data-recovery.svg)](https://github.com/crocdeine/ps3-data-recovery/issues)

`ps3recover` is a single command-line tool for recovering photos, videos, music and save data from a formatted PS3 internal hard drive. It runs on macOS, Windows and Linux, and needs nothing else installed.

> **Status: work in progress.** Finding, sorting and checking files works today on any decrypted image. **Decryption of the PS3 drive is not available yet**: the previous version did not really decrypt anything (it applied a placeholder XOR), so it has been removed while the real decryption is written.

## How it works

The PS3 encrypts its internal drive with keys derived from a console-specific secret, the `eid_root_key`. Formatting the drive does not erase the data, it only makes it unreachable. Recovery takes three stages:

1. **Decrypt** the raw image of the drive with the console's `eid_root_key` *(coming soon)*.
2. **Carve**: scan the decrypted image for known file headers (JPEG, PNG, MP4, PARAM.SFO, PKG…) and extract each file into a folder for its type.
3. **Verify**: open each recovered file and move the broken ones into a `corrupted/` folder.

## What you need

- A raw copy of the whole PS3 internal drive (for example `ps3_image.img`).
- Your console's `eid_root_key` (48 bytes, binary). It is unique to your console: **never share it**.
- Enough free disk space for the decrypted image (as large as the drive) plus the recovered files.
- Optional: [`ffmpeg`](https://ffmpeg.org/) on your `PATH`, to check audio and video files. Without it those files are kept but not checked.

Tip: until you have made the copy, use the PS3 as little as possible. Every new save or install can overwrite deleted files.

## Install

Prebuilt programs for macOS, Windows and Linux will be attached to each [release](https://github.com/crocdeine/ps3-data-recovery/releases).

To build from source, install [Rust](https://rustup.rs/) and run:

```bash
cargo build --release
# the program is now at target/release/ps3recover (ps3recover.exe on Windows)
```

## Usage

```bash
# 1. Decrypt the drive image (not available yet)
ps3recover decrypt ps3_image.img ps3_decrypted.img --key eid_root_key

# 2 + 3. Extract the files, then check them
ps3recover recover ps3_decrypted.img recovered/
```

Recovered files land in `recovered/photos`, `recovered/videos`, `recovered/music`, `recovered/ps3_files`, `recovered/documents` and `recovered/archives`. Each file is named after its position in the image (in hexadecimal), so you can find it again in a hex editor.

The two stages can also be run separately:

```bash
ps3recover carve ps3_decrypted.img recovered/
ps3recover verify recovered/
```

### Using PhotoRec instead

[PhotoRec](https://www.cgsecurity.org/wiki/PhotoRec) can be pointed at the decrypted image with the signatures in [`photorec.sig`](photorec.sig) (copy it to `~/.photorec.sig`). Then sort and check its output:

```bash
ps3recover sort recup_folder/ organized/
ps3recover verify organized/
```

Run `ps3recover --help` or `ps3recover <command> --help` for every option.

## Development

```bash
cargo test     # run the tests
cargo clippy   # lint
cargo fmt      # format
```

File signatures live in [`src/signatures.rs`](src/signatures.rs). After changing them, regenerate the PhotoRec file with `cargo run -- photorec-sig > photorec.sig` (a test checks that it is up to date).

## License

MIT License.
