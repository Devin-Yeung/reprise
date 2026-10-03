use std::path::Path;
use std::time::Duration;

use bollard::Docker;
use bollard::errors::Error as DockerError;
use bollard::query_parameters::ImportImageOptions;
use futures_util::TryStreamExt;

use crate::archive::{ArchiveError, SavedImage};
use crate::error::PrepareError;
use crate::image::{ImageId, ImagePreparation, PreparedImage};

/// A negotiated connection to an explicit Docker Unix socket.
///
/// Share it with `&self` across parallel tests. Construction does not read
/// `DOCKER_HOST` or a Docker context. The stored timeout bounds [`Self::connect`]
/// and each [`Self::ensure_image`].
pub struct DockerEndpoint {
    docker: Docker,
    timeout: Duration,
}

impl DockerEndpoint {
    /// Connect to `socket` and negotiate the Engine API version.
    ///
    /// `socket` is a Unix socket path, not a `unix://` URI. `timeout` is the
    /// caller's deadline for the whole connection.
    pub async fn connect(socket: &str, timeout: Duration) -> Result<Self, PrepareError> {
        // The client timeout sits just past this deadline, so expiry surfaces as
        // ConnectTimeout rather than a bollard request timeout.
        let connect = async {
            Docker::connect_with_unix(
                socket,
                timeout.as_secs().saturating_add(1),
                bollard::API_DEFAULT_VERSION,
            )
            .map_err(PrepareError::Connect)?
            .negotiate_version()
            .await
            .map_err(PrepareError::Connect)
        };

        let docker = match tokio::time::timeout(timeout, connect).await {
            Ok(result) => result?,
            Err(_elapsed) => return Err(PrepareError::ConnectTimeout),
        };
        Ok(Self { docker, timeout })
    }

    /// Make the single image in an uncompressed Docker save present on this endpoint.
    ///
    /// The archive is read on every call, including a cache hit. Its config bytes
    /// determine the returned ID. Only a lookup 404 uploads; upload also imports
    /// archive tags. Concurrent misses may each upload. Nothing is deleted or retagged.
    ///
    /// The stored timeout bounds this call. A blocking archive read or an engine-side
    /// load may continue after [`PrepareError::PrepareTimeout`].
    pub async fn ensure_image(
        &self,
        archive: impl AsRef<Path>,
    ) -> Result<PreparedImage, PrepareError> {
        let prepare = self.prepare(archive.as_ref());
        match tokio::time::timeout(self.timeout, prepare).await {
            Ok(result) => result,
            Err(_elapsed) => Err(PrepareError::PrepareTimeout),
        }
    }

    async fn prepare(&self, path: &Path) -> Result<PreparedImage, PrepareError> {
        let path = path.to_owned();
        // Tar scans are blocking even on a cache hit; keep them off the executor.
        let saved = tokio::task::spawn_blocking(move || SavedImage::open(&path))
            .await
            .expect("archive reader panicked")?;
        let id = saved.id().clone();

        let preparation = if self.image_present(&id).await? {
            ImagePreparation::Cached
        } else {
            self.load_image(&saved).await?;
            ImagePreparation::Loaded
        };
        Ok(PreparedImage { id, preparation })
    }

    async fn image_present(&self, id: &ImageId) -> Result<bool, PrepareError> {
        // A 404 is a cache miss only before loading, not after it.
        match self.docker.inspect_image(id.as_str()).await {
            Ok(_) => Ok(true),
            Err(error) if is_not_found(&error) => Ok(false),
            Err(error) => Err(PrepareError::Engine(error)),
        }
    }

    async fn load_image(&self, archive: &SavedImage) -> Result<(), PrepareError> {
        let upload = archive
            .upload_stream()
            .map_err(|source| PrepareError::Archive(ArchiveError::Read(source)))?;
        let mut response =
            self.docker
                .import_image_stream(ImportImageOptions::default(), upload, None);

        // HTTP success is not load success. Bollard also folds an errorDetail
        // message into DockerStreamError, which classify_load_error reports as a rejection.
        while let Some(message) = response.try_next().await.map_err(classify_load_error)? {
            if let Some(detail) = message.error_detail {
                let message = match &detail.message {
                    Some(message) => message.clone(),
                    None => format!("{detail:?}"),
                };
                return Err(PrepareError::LoadRejected { message });
            }
        }

        self.confirm_loaded(archive.id()).await
    }

    async fn confirm_loaded(&self, id: &ImageId) -> Result<(), PrepareError> {
        match self.docker.inspect_image(id.as_str()).await {
            Ok(image) if image.id.as_deref() == Some(id.as_str()) => Ok(()),
            Ok(_) => Err(PrepareError::IdentityMismatch {
                expected: id.clone(),
            }),
            Err(error) if is_not_found(&error) => Err(PrepareError::IdentityMismatch {
                expected: id.clone(),
            }),
            Err(error) => Err(PrepareError::Engine(error)),
        }
    }
}

fn is_not_found(error: &DockerError) -> bool {
    matches!(
        error,
        DockerError::DockerResponseServerError {
            status_code: 404,
            ..
        }
    )
}

fn classify_load_error(error: DockerError) -> PrepareError {
    match error {
        DockerError::DockerStreamError { error } => PrepareError::LoadRejected { message: error },
        other => PrepareError::Engine(other),
    }
}
