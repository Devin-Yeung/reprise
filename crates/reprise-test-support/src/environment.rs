use std::path::Path;
use std::time::Duration;

use bollard::Docker;
use bollard::errors::Error;
use bollard::models::ImageInspect;
use bollard::query_parameters::ImportImageOptions;
use futures_util::TryStreamExt;

use crate::archive::SavedImageArchive;
use crate::{ImagePreparation, PreparedImage};

/// A Docker connection for preparing test images. Safe to share among parallel tests.
pub struct DockerTestEnvironment {
    docker: Docker,
    timeout: Duration,
}

impl DockerTestEnvironment {
    /// Connect to an explicit UTF-8 Docker Unix socket path and negotiate the Engine version.
    /// `timeout` bounds connection and each subsequent preparation as a whole.
    ///
    /// # Panics
    /// Panics if the path is not UTF-8, Docker cannot connect, or connection times out.
    pub async fn connect(socket: impl AsRef<Path>, timeout: Duration) -> Self {
        let socket = socket.as_ref().to_str().expect("socket path must be UTF-8");

        let docker = tokio::time::timeout(timeout, async {
            Docker::connect_with_unix(
                socket,
                timeout.as_secs().saturating_add(1),
                bollard::API_DEFAULT_VERSION,
            )
            .expect("connect to test Docker socket")
            .negotiate_version()
            .await
        })
        .await
        .expect("Docker connection timed out")
        .expect("negotiate Docker version");

        Self { docker, timeout }
    }

    /// Prepare a single-image, uncompressed Docker save tar.
    ///
    /// The caller supplies an immutable fixture suitable for the test host;
    /// the archive is read even on cache hits. Only a lookup 404 triggers upload,
    /// which also imports archive tags. Concurrent misses may each upload.
    /// Timeout bounds the caller's wait; blocking reads or Docker-side loading
    /// may continue after cancellation. No shared images are removed.
    ///
    /// # Panics
    /// Panics if reading the archive or preparing the image fails, or preparation times out.
    pub async fn ensure_image(&self, archive: impl AsRef<Path>) -> PreparedImage {
        tokio::time::timeout(self.timeout, async {
            let archive = SavedImageArchive::read(archive.as_ref()).await;

            let (image, preparation) = match self.find_image(&archive.image_id).await {
                Some(image) => (image, ImagePreparation::Cached),
                None => {
                    let image = self.load_image(&archive).await;
                    (image, ImagePreparation::Loaded)
                }
            };

            PreparedImage { image, preparation }
        })
        .await
        .expect("image preparation timed out")
    }

    async fn find_image(&self, image_id: &str) -> Option<ImageInspect> {
        // Absence is a cache miss only before loading, not after it.
        match self.docker.inspect_image(image_id).await {
            Ok(image) => Some(image),
            Err(Error::DockerResponseServerError {
                status_code: 404, ..
            }) => None,
            Err(error) => panic!("look up test image: {error}"),
        }
    }

    async fn load_image(&self, archive: &SavedImageArchive) -> ImageInspect {
        let mut response = self.docker.import_image_stream(
            ImportImageOptions::default(),
            archive.upload_stream(),
            None,
        );

        // HTTP success is not load success: consume late stream failures too.
        while let Some(message) = response.try_next().await.expect("load test image") {
            if let Some(detail) = message.error_detail {
                panic!("Docker rejected test image: {detail:?}");
            }
        }

        self.docker
            .inspect_image(&archive.image_id)
            .await
            .expect("inspect loaded test image")
    }
}
