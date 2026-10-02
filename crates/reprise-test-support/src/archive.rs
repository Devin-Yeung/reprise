use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Component, Path};

use bollard::models::{ImageInspect, Platform};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio_util::io::ReaderStream;

/// Keep the validated descriptor rather than reopening a potentially replaced path.
pub(crate) struct ValidatedArchive {
    file: File,
    pub(crate) image_id: String,
    pub(crate) platform: Platform,
}

impl ValidatedArchive {
    pub(crate) async fn read(path: &Path) -> Self {
        let path = path.to_owned();
        // Tar scans are blocking even on cache hits; keep them off the executor.
        tokio::task::spawn_blocking(move || validate_archive(&path))
            .await
            .expect("read test archive")
    }

    pub(crate) fn upload_stream(&self) -> ReaderStream<tokio::fs::File> {
        // The fixture is immutable by contract. Retain the original descriptor
        // rather than reopening its path; no concurrent-write detection is needed.
        // Its clone shares the offset, so upload once per preparation.
        let file = self
            .file
            .try_clone()
            .expect("clone test archive for upload");
        ReaderStream::new(tokio::fs::File::from_std(file))
    }

    pub(crate) fn verify_image(&self, image: &ImageInspect) {
        assert_eq!(
            image.id.as_deref(),
            Some(self.image_id.as_str()),
            "loaded image ID must match archive"
        );
        assert_eq!(
            image.os, self.platform.os,
            "loaded image OS must match archive"
        );
        assert_eq!(
            image.architecture, self.platform.architecture,
            "loaded image architecture must match archive"
        );
        assert!(
            image.variant.as_deref().is_none_or(str::is_empty),
            "image variants are unsupported"
        );
    }
}

fn validate_archive(path: &Path) -> ValidatedArchive {
    let mut file = File::open(path).expect("open test image archive");
    let metadata = file.metadata().expect("inspect test image archive");
    assert!(metadata.is_file(), "expected a regular Docker save tar");

    let bytes = read_config(&mut file);
    let config: ImageConfig = serde_json::from_slice(&bytes).expect("parse image config");
    assert_eq!(config.os, "linux", "test image must target Linux");
    assert!(
        matches!(config.architecture.as_str(), "amd64" | "arm64"),
        "unsupported test image architecture"
    );
    assert!(config.variant.is_empty(), "image variants are unsupported");

    file.rewind().expect("rewind test image archive for upload");
    ValidatedArchive {
        file,
        // Docker IDs hash exact config bytes, including whitespace.
        image_id: format!("sha256:{:x}", Sha256::digest(&bytes)),
        platform: Platform {
            os: Some(config.os),
            architecture: Some(config.architecture),
        },
    }
}

#[derive(Deserialize)]
struct ManifestImage {
    #[serde(rename = "Config")]
    config: String,
}

#[derive(Deserialize)]
struct ImageConfig {
    os: String,
    architecture: String,
    #[serde(default)]
    variant: String,
}

fn read_config(file: &mut File) -> Vec<u8> {
    // Two passes avoid retaining layer payloads or depending on manifest order.
    // Layers are validated by Docker during load, not duplicated here.
    let manifest = read_entry(file, "manifest.json", 1024 * 1024);
    let images: Vec<ManifestImage> =
        serde_json::from_slice(&manifest).expect("parse Docker save manifest");
    let [image] = images.as_slice() else {
        panic!("expected exactly one manifest image");
    };
    if image.config.is_empty()
        || image.config.contains('\\')
        || image
            .config
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || Path::new(&image.config)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        || image.config == "manifest.json"
    {
        panic!("manifest config must be an unambiguous relative path");
    }
    read_entry(file, &image.config, 16 * 1024 * 1024)
}

fn read_entry(file: &mut File, name: &str, limit: u64) -> Vec<u8> {
    file.rewind().expect("rewind test archive for reading");
    let mut archive = tar::Archive::new(file);
    let mut found = None;
    for entry in archive.entries().expect("read Docker save entries") {
        let mut entry = entry.expect("read Docker save entry");
        if entry.path_bytes().as_ref() != name.as_bytes() {
            continue;
        }
        if found.is_some() || !entry.header().entry_type().is_file() || entry.size() > limit {
            panic!("duplicate, non-file or oversized {name}");
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .expect("read archive entry contents");
        found = Some(bytes);
    }
    found.unwrap_or_else(|| panic!("missing {name}"))
}
