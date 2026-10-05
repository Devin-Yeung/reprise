//! Nix runtime closures represented as a read-only bind-mount layer.
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{Bind, Error, Layer};

/// Store objects listed by a complete local Nix runtime closure artifact.
///
/// The artifact producer supplies complete absolute store paths and the target
/// architecture. The caller keeps these objects available while containers or
/// snapshots use them; loading only reads metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NixClosure {
    store_paths: Vec<PathBuf>,
}

impl NixClosure {
    /// Reads a `store-paths` file and describes its objects as read-only mounts.
    pub fn load(manifest_path: impl AsRef<Path>) -> Result<Self, Error> {
        let manifest = fs::read_to_string(manifest_path)?;
        let store_paths = manifest.lines().map(PathBuf::from).collect();
        Ok(Self { store_paths })
    }

    /// Store objects in manifest order.
    pub fn store_paths(&self) -> &[PathBuf] {
        &self.store_paths
    }
}

/// Exposes the closure as read-only binds of each store object at its own path.
///
/// Objects are bound individually, so the container sees only the closure and
/// not the host's whole store. Stack the layer on a [`Rootfs`](crate::Rootfs)
/// and call `prepare` to create the mount targets.
impl From<&NixClosure> for Layer {
    fn from(closure: &NixClosure) -> Self {
        Layer::binds(
            closure
                .store_paths()
                .iter()
                .map(|path| Bind::read_only(path, path)),
        )
    }
}

/// A Nix output that describes both a workload's complete closure and its
/// intentionally public command namespace.
///
/// The artifact keeps discovery out of callers: they supply its directory,
/// then receive the complete set of rootfs mounts. Its command profile must be
/// a member of the closure, because profile symlinks only work when their
/// targets are available at the original store paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifact {
    closure: NixClosure,
    command_profile: PathBuf,
    command_directory: PathBuf,
}

impl RuntimeArtifact {
    /// Reads and validates a version-one runtime artifact directory.
    ///
    /// The directory contains `manifest.json` and `store-paths`. Validation
    /// checks the filesystem only enough to reject an artifact that cannot
    /// form its advertised command mount; it neither invokes Nix nor copies
    /// store objects.
    pub fn load(artifact_directory: impl AsRef<Path>) -> Result<Self, Error> {
        let artifact_directory = artifact_directory.as_ref();
        let manifest: RuntimeArtifactManifest =
            serde_json::from_reader(fs::File::open(artifact_directory.join("manifest.json"))?)?;
        if manifest.format_version != 1 {
            return Err(Error::UnsupportedRuntimeArtifactFormat {
                version: manifest.format_version,
            });
        }

        let closure = NixClosure::load(artifact_directory.join("store-paths"))?;
        if !closure.store_paths().contains(&manifest.command_profile) {
            return Err(Error::CommandProfileOutsideClosure {
                profile: manifest.command_profile,
            });
        }
        if !manifest.command_profile.join("bin").is_dir() {
            return Err(Error::MissingCommandProfileBin {
                profile: manifest.command_profile,
            });
        }

        Ok(Self {
            closure,
            command_profile: manifest.command_profile,
            command_directory: manifest.command_directory,
        })
    }

    /// Store object containing the profile whose `bin` directory appears at
    /// `/bin` inside a rootfs prepared from this artifact.
    pub fn command_profile(&self) -> &Path {
        &self.command_profile
    }

    /// The PATH entry that resolves commands exposed by this artifact.
    pub fn path_environment(&self) -> String {
        format!("PATH={}", self.command_directory.display())
    }
}

/// The producer-owned part of a runtime artifact's stable file format.
#[derive(Deserialize)]
struct RuntimeArtifactManifest {
    format_version: u32,
    command_profile: PathBuf,
    command_directory: PathBuf,
}

/// Exposes the closure and its profile-backed command namespace read-only.
impl From<&RuntimeArtifact> for Layer {
    fn from(artifact: &RuntimeArtifact) -> Self {
        Layer::binds(
            artifact
                .closure
                .store_paths()
                .iter()
                .map(|path| Bind::read_only(path, path))
                .chain(std::iter::once(Bind::read_only(
                    artifact.command_profile.join("bin"),
                    &artifact.command_directory,
                ))),
        )
    }
}
