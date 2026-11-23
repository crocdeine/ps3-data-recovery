# PS3 Data Recovery

[![GitHub stars](https://img.shields.io/github/stars/crocdeine/ps3-data-recovery.svg?style=social&label=Star)](https://github.com/crocdeine/ps3-data-recovery)
[![GitHub forks](https://img.shields.io/github/forks/crocdeine/ps3-data-recovery.svg?style=social&label=Fork)](https://github.com/crocdeine/ps3-data-recovery/fork)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub issues](https://img.shields.io/github/issues/crocdeine/ps3-data-recovery.svg)](https://github.com/crocdeine/ps3-data-recovery/issues)

`ps3recover` is a single command-line tool for recovering photos, videos, music and save data from a formatted PS3 internal hard drive. It runs on macOS, Windows and Linux, and needs nothing else installed.

> **Status: work in progress.** Decryption, file carving, sorting and checking all work and are tested on synthetic images. They have not yet been tried on a real PS3 drive, so please [open an issue](https://github.com/crocdeine/ps3-data-recovery/issues) with your console model if you try it.

**New to this?** Follow the step-by-step guide: [English](docs/GUIDE.md) · [Français](docs/GUIDE.fr.md). It covers copying the drive, getting the key from the console, and running the tool on macOS, Windows and Linux.

## How it works

The PS3 encrypts its internal drive with keys derived from a console-specific secret, the `eid_root_key`. Formatting the drive does not erase the data, it only makes it unreachable. Recovery takes three stages:

1. **Decrypt** the raw image of the drive with the console's `eid_root_key`.
2. **Carve**: scan the decrypted image for known file headers (JPEG, PNG, MP4, PARAM.SFO, PKG…) and extract each file into a folder for its type. For most types the tool follows the file's structure to find exactly where it ends, so files come out at their real size and pictures stored inside other files (thumbnails, files in a ZIP) are not split out.
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
# 1. Decrypt the drive image
ps3recover decrypt ps3_image.img ps3_decrypted.img --key eid_root_key

# 2 + 3. Extract the files, then check them
ps3recover recover ps3_decrypted.img recovered/
```

`decrypt` first checks the key on the drive's first sector. If the key is wrong, or the image does not start at the drive's first sector, it stops before writing anything. It also detects the console model on its own; you can force it with `--model fat` or `--model slim`.

### What can be recovered

| Type | Extensions | Exact end found |
|---|---|---|
| Photos | jpg, mpo, png, gif, bmp | yes |
| Videos | mp4, m2ts (PS3 recordings), avi | yes |
| Music | mp3, wav / at3 | yes |
| PS3 files | PARAM.SFO, pkg, self / sprx | yes |
| PS3 files | edat, rco, p3t | no: cut at the next file or a size limit |
| Documents and archives | xml, zip, tar | yes |
| Archives | gz | no |

Carving works on files stored in one piece. A file split across several places on the drive (fragmented) usually comes out damaged, and `verify` moves it to `corrupted/`.

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

## How the PS3 encrypts its drive

| Console | Models | Cipher (per 512-byte sector) |
|---|---|---|
| Fat | CECHAxx to CECHQxx | AES-192-CBC, all-zero IV |
| Slim, Super Slim | CECH-2xxx, 3xxx, 4xxx | AES-128-XTS, tweak = sector number |

The keys come from encrypting two fixed seeds with AES-256-CBC, keyed by the `eid_root_key` (its first 32 bytes are the key, its last 16 bytes the IV). Every pair of bytes on the drive is also swapped. Sources: [`ps3hdd_keygen.sh`](https://github.com/einsteinx2/PS3-Reclaim-HDD-Space/blob/master/ps3hdd_keygen.sh), [psdevwiki: HDD Encryption](https://www.psdevwiki.com/ps3/Talk:HDD_Encryption), [PS3 drive decryption notes](https://gist.github.com/andshrew/1d0b5e35308a6fd9770831e47950be0a). Details are in [`src/decrypt.rs`](src/decrypt.rs).

Super Slim support relies on these sources saying it matches the Slim. If `decrypt` refuses a Super Slim drive with a key you know is right, please open an issue.

## Development

```bash
cargo test     # run the tests
cargo clippy   # lint
cargo fmt      # format
```

The decryption tests use small images encrypted by OpenSSL, not by this tool, so they check it against an independent implementation. [`tests/fixtures/make_fixtures.py`](tests/fixtures/make_fixtures.py) regenerates them.

File signatures live in [`src/signatures.rs`](src/signatures.rs). After changing them, regenerate the PhotoRec file with `cargo run -- photorec-sig > photorec.sig` (a test checks that it is up to date).

### Publishing a release

Bump `version` in `Cargo.toml`, merge it, then run the **Release** workflow from the Actions tab with the matching version (for example `v0.1.0`). It builds the programs for every system, then creates the tag and the GitHub release with them attached.

## License

MIT License.
