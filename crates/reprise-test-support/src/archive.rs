use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Component, Path};

use bollard::models::{ImageInspect, Platform};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio_util::io::ReaderStream;

use anyhow::{Context, Result, ensure};

/// Keep the validated descriptor rather than reopening a potentially replaced path.
pub(crate) struct ValidatedArchive {
    file: File,
    pub(crate) image_id: String,
    pub(crate) platform: Platform,
}

impl ValidatedArchive {
    pub(crate) async fn read(path: &Path) -> Result<Self> {
        let path = path.to_owned();
        // Tar scans are blocking even on cache hits; keep them off the executor.
        tokio::task::spawn_blocking(move || {
            validate_archive(&path).with_context(|| format!("read test archive {}", path.display()))
        })
        .await
        .context("archive reader failed")?
    }

    pub(crate) fn upload_stream(&self) -> Result<ReaderStream<tokio::fs::File>> {
        // The fixture is immutable by contract. Retain the original descriptor
        // rather than reopening its path; no concurrent-write detection is needed.
        // Its clone shares the offset, so upload once per preparation.
        let file = self
            .file
            .try_clone()
            .context("clone test archive for upload")?;
        Ok(ReaderStream::new(tokio::fs::File::from_std(file)))
    }

    pub(crate) fn verify_image(&self, image: &ImageInspect) -> Result<()> {
        ensure!(
            image.id.as_deref() == Some(&self.image_id)
                && image.os == self.platform.os
                && image.architecture == self.platform.architecture
                && image.variant.as_deref().is_none_or(str::is_empty),
            "Docker image ID or platform differs from archive config {}",
            self.image_id
        );
        Ok(())
    }
}

fn validate_archive(path: &Path) -> Result<ValidatedArchive> {
    let mut file = File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "expected a regular Docker save tar"
    );
    let bytes = read_config(&mut file)?;
    let config: ImageConfig = serde_json::from_slice(&bytes)?;
    ensure!(
        config.os == "linux"
            && matches!(config.architecture.as_str(), "amd64" | "arm64")
            && config.variant.is_empty(),
        "only Linux amd64/arm64 configs without variants are supported"
    );
    file.rewind()?;
    Ok(ValidatedArchive {
        file,
        // Docker IDs hash exact config bytes, including whitespace.
        image_id: format!("sha256:{:x}", Sha256::digest(&bytes)),
        platform: Platform {
            os: Some(config.os),
            architecture: Some(config.architecture),
        },
    })
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

fn read_config(file: &mut File) -> Result<Vec<u8>> {
    // Two passes avoid retaining layer payloads or depending on manifest order.
    // Layers are validated by Docker during load, not duplicated here.
    let manifest = read_entry(file, "manifest.json", 1024 * 1024)?;
    let images: Vec<ManifestImage> = serde_json::from_slice(&manifest)?;
    let [image] = images.as_slice() else {
        anyhow::bail!("expected exactly one manifest image");
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
        anyhow::bail!("manifest config must be an unambiguous relative path");
    }
    read_entry(file, &image.config, 16 * 1024 * 1024)
}

fn read_entry(file: &mut File, name: &str, limit: u64) -> Result<Vec<u8>> {
    file.rewind()?;
    let mut archive = tar::Archive::new(file);
    let mut found = None;
    for entry in archive.entries()? {
        let mut entry = entry?;
        if entry.path_bytes().as_ref() != name.as_bytes() {
            continue;
        }
        if found.is_some() || !entry.header().entry_type().is_file() || entry.size() > limit {
            anyhow::bail!("duplicate, non-file or oversized {name}");
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        found = Some(bytes);
    }
    found.with_context(|| format!("missing {name}"))
}
