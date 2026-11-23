use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use ps3recover::{carve, decrypt, signatures, sort, verify};

/// Recover user data from a formatted PS3 internal hard drive.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Decrypt a raw PS3 disk image using the console's eid_root_key.
    Decrypt {
        /// Encrypted disk image (a raw copy of the whole PS3 drive).
        image: PathBuf,
        /// Where to write the decrypted image.
        output: PathBuf,
        /// File holding the 48-byte eid_root_key (binary).
        #[arg(short, long)]
        key: PathBuf,
    },
    /// Find files in a decrypted image and save them sorted by type.
    Carve {
        /// Decrypted disk image.
        image: PathBuf,
        /// Folder where recovered files are written.
        output: PathBuf,
    },
    /// Sort a folder of recovered files (for example PhotoRec output) by type.
    Sort {
        /// Folder holding the recovered files.
        input: PathBuf,
        /// Folder where sorted files are moved (default: <input>/organized).
        output: Option<PathBuf>,
    },
    /// Check sorted files and move broken ones into a `corrupted` subfolder.
    Verify {
        /// Folder produced by `carve` or `sort`.
        folder: PathBuf,
    },
    /// Run `carve` then `verify` in one go.
    Recover {
        /// Decrypted disk image.
        image: PathBuf,
        /// Folder where recovered files are written.
        output: PathBuf,
    },
    /// Print the signature list in PhotoRec's `photorec.sig` format.
    PhotorecSig,
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Decrypt { image, output, key } => decrypt::run(&image, &output, &key),
        Command::Carve { image, output } => carve::run(&image, &output).map(|_| ()),
        Command::Sort { input, output } => {
            let output = output.unwrap_or_else(|| input.join("organized"));
            sort::run(&input, &output).map(|_| ())
        }
        Command::Verify { folder } => verify::run(&folder).map(|_| ()),
        Command::Recover { image, output } => {
            carve::run(&image, &output)?;
            verify::run(&output).map(|_| ())
        }
        Command::PhotorecSig => {
            print!("{}", signatures::photorec_sig());
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
