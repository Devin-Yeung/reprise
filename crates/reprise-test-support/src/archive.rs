use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio_util::io::ReaderStream;

/// A caller-provided Docker save fixture and its immutable image ID.
pub(crate) struct SavedImageArchive {
    file: File,
    pub(crate) image_id: String,
}

impl SavedImageArchive {
    pub(crate) async fn read(path: &Path) -> Self {
        let path = path.to_owned();
        // Tar scans are blocking even on cache hits; keep them off the executor.
        tokio::task::spawn_blocking(move || read_archive(&path))
            .await
            .expect("read test archive")
    }

    pub(crate) fn upload_stream(&self) -> ReaderStream<tokio::fs::File> {
        // The fixture stays immutable by contract. The clone shares the offset,
        // so upload once per preparation, using the descriptor we already read.
        let file = self
            .file
            .try_clone()
            .expect("clone test archive for upload");
        ReaderStream::new(tokio::fs::File::from_std(file))
    }
}

fn read_archive(path: &Path) -> SavedImageArchive {
    let mut file = File::open(path).expect("open test image archive");
    let bytes = read_config(&mut file);
    file.rewind().expect("rewind test image archive for upload");
    SavedImageArchive {
        file,
        // Docker IDs hash exact config bytes, including whitespace.
        image_id: format!("sha256:{:x}", Sha256::digest(&bytes)),
    }
}

#[derive(Deserialize)]
struct ManifestImage {
    #[serde(rename = "Config")]
    config: String,
}

fn read_config(file: &mut File) -> Vec<u8> {
    // Two passes avoid retaining layer payloads or depending on manifest order.
    // The caller supplies a single-image Docker save tar; Docker handles loading.
    let manifest = read_entry(file, "manifest.json");
    let images: Vec<ManifestImage> =
        serde_json::from_slice(&manifest).expect("parse Docker save manifest");
    read_entry(file, &images[0].config)
}

fn read_entry(file: &mut File, name: &str) -> Vec<u8> {
    file.rewind().expect("rewind test archive for reading");
    let mut archive = tar::Archive::new(file);
    for entry in archive.entries().expect("read Docker save entries") {
        let mut entry = entry.expect("read Docker save entry");
        if entry.path_bytes().as_ref() == name.as_bytes() {
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .expect("read archive entry contents");
            return bytes;
        }
    }
    panic!("missing {name}")
}
