//! Sort a folder of recovered files (for example PhotoRec's `recup_dir.*`
//! folders) into category folders.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::category::Category;
use crate::fsutil::move_into;

/// Move every file found under `input` into `output/<category>/`.
/// Returns how many files were moved.
pub fn run(input: &Path, output: &Path) -> Result<u64> {
    let files = list_files(input, output)?;
    let mut moved = 0;
    for file in &files {
        let dir = output.join(Category::of_path(file).folder());
        move_into(file, &dir).with_context(|| format!("cannot move {}", file.display()))?;
        moved += 1;
    }
    println!(
        "Sorting complete: {moved} file(s) moved into {}",
        output.display()
    );
    Ok(moved)
}

/// All files under `root`, skipping anything inside `exclude`.
fn list_files(root: &Path, exclude: &Path) -> Result<Vec<PathBuf>> {
    let exclude = exclude.canonicalize().ok();
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if exclude.is_some() && dir.canonicalize().ok() == exclude {
            continue;
        }
        let entries =
            fs::read_dir(&dir).with_context(|| format!("cannot read folder {}", dir.display()))?;
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_into_category_folders_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path();
        fs::create_dir_all(input.join("recup_dir.1")).unwrap();
        fs::create_dir_all(input.join("recup_dir.2")).unwrap();
        fs::write(input.join("recup_dir.1/f1.jpg"), "a").unwrap();
        fs::write(input.join("recup_dir.2/f1.jpg"), "b").unwrap();
        fs::write(input.join("recup_dir.2/PARAM.SFO"), "c").unwrap();
        fs::write(input.join("recup_dir.2/mystery"), "d").unwrap();
        let output = input.join("organized");

        assert_eq!(run(input, &output).unwrap(), 4);
        assert!(output.join("photos/f1.jpg").is_file());
        assert!(output.join("photos/f1_1.jpg").is_file());
        assert!(output.join("ps3_files/PARAM.SFO").is_file());
        assert!(output.join("unknown/mystery").is_file());

        // Running again does not touch files already sorted.
        assert_eq!(run(input, &output).unwrap(), 0);
    }
}
