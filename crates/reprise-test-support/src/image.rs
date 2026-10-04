//! Digest-pinned image preparation on the fixture's Docker endpoint.

use std::error::Error;
use std::fmt;
use std::time::Duration;

use bollard::Docker;
use bollard::errors::Error as DockerError;
use bollard::query_parameters::CreateImageOptionsBuilder;
use futures_util::TryStreamExt;

/// Whether this call found the reference locally or completed a registry pull.
/// Concurrent cache misses may each pull; this is not a cross-call lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImagePreparation {
    /// Inspect found the digest-pinned reference before pulling.
    Cached,
    /// This call consumed the pull response and inspected the result.
    Pulled,
}

/// An image present on the Docker endpoint used for preparation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedImage {
    /// Image ID reported by Engine inspect, for use on this same endpoint.
    /// Multi-platform image stores may report an ID equal to the index digest.
    pub id: String,
    pub preparation: ImagePreparation,
}

/// Resolve `name@sha256:<64 lowercase hex digits>` to a local Docker image ID.
///
/// Only an initial inspect 404 triggers an anonymous registry pull. Other
/// inspect failures, pull failures (including streamed errors), and a failed
/// final inspect are returned without a retry or fallback. Docker selects the
/// server's platform; no client-side architecture or platform override is used.
/// Repository-name validation is left to Docker.
///
/// `timeout` bounds inspect, pull, and final inspect together; the supplied
/// client's request timeout may fail sooner. After cancellation, the Engine
/// may still finish a pull. Parallel callers may share the client and image;
/// concurrent misses may pull independently. No runtime/checkpoint capability
/// is established by successful image preparation.
pub async fn ensure_image(
    docker: &Docker,
    reference: &str,
    timeout: Duration,
) -> Result<PreparedImage, PrepareError> {
    let valid = reference.split_once('@').is_some_and(|(name, digest)| {
        !name.is_empty() && !name.chars().any(char::is_whitespace) && valid_id(digest)
    });
    if !valid {
        return Err(PrepareError::InvalidReference);
    }
    match tokio::time::timeout(timeout, prepare(docker, reference)).await {
        Ok(result) => result,
        Err(_) => Err(PrepareError::Timeout),
    }
}

fn valid_id(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

async fn prepare(docker: &Docker, reference: &str) -> Result<PreparedImage, PrepareError> {
    let (image, preparation) = match docker.inspect_image(reference).await {
        Ok(image) => (image, ImagePreparation::Cached),
        Err(DockerError::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            let options = CreateImageOptionsBuilder::default()
                .from_image(reference)
                .build();
            let mut pull = docker.create_image(Some(options), None, None);
            while let Some(message) = pull.try_next().await? {
                // Bollard converts details with a message into DockerStreamError;
                // details without one must still reject the pull.
                if let Some(detail) = message.error_detail {
                    return Err(DockerError::DockerStreamError {
                        error: format!("{detail:?}"),
                    }
                    .into());
                }
            }
            (
                docker.inspect_image(reference).await?,
                ImagePreparation::Pulled,
            )
        }
        Err(error) => return Err(PrepareError::Engine(error)),
    };
    let id = image
        .id
        .filter(|id| valid_id(id))
        .ok_or(PrepareError::InvalidImage {
            reason: "inspect did not return a valid image ID".into(),
        })?;
    Ok(PreparedImage { id, preparation })
}

/// Failure preparing a digest-pinned registry image.
#[derive(Debug)]
pub enum PrepareError {
    /// The reference lacks a repository name or a well-formed SHA-256 pin.
    InvalidReference,
    /// The whole preparation deadline elapsed; an Engine-side pull may continue.
    Timeout,
    /// Inspect, pull transport, or a streamed registry error from Bollard.
    Engine(DockerError),
    /// Inspect succeeded but did not yield a usable Docker image ID.
    InvalidImage { reason: String },
}

impl From<DockerError> for PrepareError {
    fn from(error: DockerError) -> Self {
        Self::Engine(error)
    }
}

impl fmt::Display for PrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReference => {
                f.write_str("image reference must be name@sha256:<64 lowercase hex digits>")
            }
            Self::Timeout => f.write_str("timed out preparing image"),
            Self::Engine(error) => write!(f, "Docker engine: {error}"),
            Self::InvalidImage { reason } => write!(f, "invalid prepared image: {reason}"),
        }
    }
}

impl Error for PrepareError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Engine(error) => Some(error),
            _ => None,
        }
    }
}
