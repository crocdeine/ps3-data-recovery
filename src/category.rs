//! File categories shared by `carve`, `sort` and `verify`.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Photos,
    Videos,
    Music,
    Ps3Files,
    Documents,
    Archives,
    Unknown,
}

impl Category {
    pub const ALL: [Category; 7] = [
        Category::Photos,
        Category::Videos,
        Category::Music,
        Category::Ps3Files,
        Category::Documents,
        Category::Archives,
        Category::Unknown,
    ];

    /// Name of the folder files of this category go into.
    pub fn folder(self) -> &'static str {
        match self {
            Category::Photos => "photos",
            Category::Videos => "videos",
            Category::Music => "music",
            Category::Ps3Files => "ps3_files",
            Category::Documents => "documents",
            Category::Archives => "archives",
            Category::Unknown => "unknown",
        }
    }

    fn extensions(self) -> &'static [&'static str] {
        match self {
            Category::Photos => &["jpg", "jpeg", "png", "bmp", "gif", "mpo"],
            Category::Videos => &["mp4", "avi", "m2ts", "mts"],
            Category::Music => &["mp3", "aac", "wav", "at3"],
            Category::Ps3Files => &[
                "sfo", "pfd", "pkg", "p3t", "edat", "self", "sprx", "rco", "raf", "bin", "dat",
            ],
            Category::Documents => &["xml", "txt"],
            Category::Archives => &["zip", "tar", "gz"],
            Category::Unknown => &[],
        }
    }

    /// Category of a file, judged by its extension (case-insensitive).
    pub fn of_path(path: &Path) -> Category {
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            return Category::Unknown;
        };
        let ext = ext.to_ascii_lowercase();
        Category::ALL
            .into_iter()
            .find(|c| c.extensions().contains(&ext.as_str()))
            .unwrap_or(Category::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorizes_by_extension() {
        assert_eq!(Category::of_path(Path::new("a/IMG.JPG")), Category::Photos);
        assert_eq!(
            Category::of_path(Path::new("PARAM.SFO")),
            Category::Ps3Files
        );
        assert_eq!(Category::of_path(Path::new("x.m2ts")), Category::Videos);
        assert_eq!(Category::of_path(Path::new("noext")), Category::Unknown);
        assert_eq!(Category::of_path(Path::new("x.weird")), Category::Unknown);
    }
}
