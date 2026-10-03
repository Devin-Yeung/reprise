use std::fmt;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio_util::io::ReaderStream;

use crate::image::ImageId;

/// A caller-provided Docker save and the image ID derived from its config bytes.
pub(crate) struct SavedImage {
    file: File,
    id: ImageId,
}

impl SavedImage {
    /// Read a single-image uncompressed Docker save.
    ///
    /// Blocking. The caller runs this off the async executor. On success the file
    /// is positioned at the start for one upload. The fixture must stay immutable
    /// while this file is open.
    pub(crate) fn open(path: &Path) -> Result<Self, ArchiveError> {
        let mut file = File::open(path).map_err(|source| ArchiveError::Open {
            path: path.to_owned(),
            source,
        })?;
        let config = read_config(&mut file)?;
        file.rewind().map_err(ArchiveError::Read)?;
        Ok(Self {
            file,
            id: ImageId::from_config_bytes(&config),
        })
    }

    pub(crate) fn id(&self) -> &ImageId {
        &self.id
    }

    /// Stream the opened fixture from the start.
    ///
    /// The clone shares this file's offset, so upload once per [`Self::open`].
    pub(crate) fn upload_stream(&self) -> Result<ReaderStream<tokio::fs::File>, std::io::Error> {
        let file = self.file.try_clone()?;
        Ok(ReaderStream::new(tokio::fs::File::from_std(file)))
    }
}

/// The path is not one readable single-image Docker save.
#[derive(Debug)]
pub enum ArchiveError {
    Open {
        path: PathBuf,
        source: std::io::Error,
    },
    Read(std::io::Error),
    BadManifest(serde_json::Error),
    /// `manifest.json` did not list exactly one image.
    NotSingleImage {
        count: usize,
    },
    MissingEntry {
        name: String,
    },
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open { path, source } => write!(f, "open {}: {source}", path.display()),
            Self::Read(source) => write!(f, "read archive: {source}"),
            Self::BadManifest(source) => write!(f, "parse Docker save manifest: {source}"),
            Self::NotSingleImage { count } => {
                write!(f, "Docker save contains {count} images; expected 1")
            }
            Self::MissingEntry { name } => write!(f, "missing {name}"),
        }
    }
}

impl std::error::Error for ArchiveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Open { source, .. } => Some(source),
            Self::Read(source) => Some(source),
            Self::BadManifest(source) => Some(source),
            Self::NotSingleImage { .. } | Self::MissingEntry { .. } => None,
        }
    }
}

#[derive(Deserialize)]
struct ManifestImage {
    #[serde(rename = "Config")]
    config: String,
}

fn read_config(file: &mut File) -> Result<Vec<u8>, ArchiveError> {
    // Two passes avoid retaining layer payloads or depending on manifest order.
    let manifest = read_entry(file, "manifest.json")?;
    let images: Vec<ManifestImage> =
        serde_json::from_slice(&manifest).map_err(ArchiveError::BadManifest)?;
    match images.as_slice() {
        [image] => read_entry(file, &image.config),
        _ => Err(ArchiveError::NotSingleImage {
            count: images.len(),
        }),
    }
}

fn read_entry(file: &mut File, name: &str) -> Result<Vec<u8>, ArchiveError> {
    file.rewind().map_err(ArchiveError::Read)?;
    let mut archive = tar::Archive::new(file);
    for entry in archive.entries().map_err(ArchiveError::Read)? {
        let mut entry = entry.map_err(ArchiveError::Read)?;
        if entry.path_bytes().as_ref() == name.as_bytes() {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).map_err(ArchiveError::Read)?;
            return Ok(bytes);
        }
    }
    Err(ArchiveError::MissingEntry {
        name: name.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use futures_util::TryStreamExt;
    use sha2::{Digest, Sha256};

    use super::*;

    struct Scratch {
        path: PathBuf,
        _dir: PathBuf,
    }

    impl Scratch {
        fn archive(files: &[(&str, &[u8])]) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let dir = std::env::temp_dir().join(format!(
                "reprise-saved-image-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&dir).unwrap();
            let path = dir.join("image.tar");
            let mut builder = tar::Builder::new(File::create(&path).unwrap());
            for (name, bytes) in files {
                let mut header = tar::Header::new_gnu();
                header.set_mode(0o644);
                header.set_size(bytes.len() as u64);
                builder.append_data(&mut header, *name, *bytes).unwrap();
            }
            builder.finish().unwrap();
            Self { path, _dir: dir }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self._dir);
        }
    }

    fn manifest(config: &str, tags: &[&str]) -> String {
        let tags = tags
            .iter()
            .map(|tag| format!("\"{tag}\""))
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"[{{"Config":"{config}","RepoTags":[{tags}],"Layers":[]}}]"#)
    }

    fn id_of(config: &[u8]) -> String {
        format!("sha256:{:x}", Sha256::digest(config))
    }

    fn open_err(path: &Path) -> ArchiveError {
        match SavedImage::open(path) {
            Err(error) => error,
            Ok(_) => panic!("expected {} to fail", path.display()),
        }
    }

    #[test]
    fn image_id_is_the_config_bytes() {
        let config = b"{\"architecture\":\"amd64\"} \n";
        let manifest = manifest("config.json", &["example:one", "example:two"]);
        let scratch = Scratch::archive(&[
            ("manifest.json", manifest.as_bytes()),
            ("config.json", config),
        ]);

        let saved = SavedImage::open(&scratch.path).expect("open single-image save");
        assert_eq!(saved.id().as_str(), id_of(config));
    }

    #[test]
    fn rejects_an_archive_that_is_not_one_image() {
        let scratch = Scratch::archive(&[("manifest.json", b"[]")]);
        assert!(matches!(
            open_err(&scratch.path),
            ArchiveError::NotSingleImage { count: 0 }
        ));

        let manifest = r#"[{"Config":"a.json"},{"Config":"b.json"}]"#;
        let scratch = Scratch::archive(&[
            ("manifest.json", manifest.as_bytes()),
            ("a.json", b"{}"),
            ("b.json", b"{}"),
        ]);
        assert!(matches!(
            open_err(&scratch.path),
            ArchiveError::NotSingleImage { count: 2 }
        ));
    }

    #[test]
    fn rejects_a_missing_or_unreadable_manifest() {
        let scratch = Scratch::archive(&[("config.json", b"{}")]);
        assert!(matches!(
            open_err(&scratch.path),
            ArchiveError::MissingEntry { name } if name == "manifest.json"
        ));

        let scratch = Scratch::archive(&[("manifest.json", b"{")]);
        assert!(matches!(
            open_err(&scratch.path),
            ArchiveError::BadManifest(_)
        ));

        let missing = scratch.path.with_file_name("missing.tar");
        assert!(matches!(
            open_err(&missing),
            ArchiveError::Open { path, .. } if path == missing
        ));
    }

    #[test]
    fn rejects_a_missing_config_entry() {
        let manifest = manifest("config.json", &[]);
        let scratch = Scratch::archive(&[("manifest.json", manifest.as_bytes())]);
        assert!(matches!(
            open_err(&scratch.path),
            ArchiveError::MissingEntry { name } if name == "config.json"
        ));
    }

    #[tokio::test]
    async fn upload_stream_starts_at_the_archive_start() {
        let config = b"{}\n";
        let manifest = manifest("config.json", &["example:local"]);
        let scratch = Scratch::archive(&[
            ("manifest.json", manifest.as_bytes()),
            ("config.json", config),
        ]);
        let saved = SavedImage::open(&scratch.path).unwrap();

        let mut bytes = Vec::new();
        let mut stream = saved.upload_stream().unwrap();
        while let Some(chunk) = stream.try_next().await.unwrap() {
            bytes.extend_from_slice(&chunk);
        }

        assert!(
            bytes
                .windows(b"manifest.json".len())
                .any(|window| window == b"manifest.json")
        );
        assert_eq!(saved.id().as_str(), id_of(config));
    }
}
