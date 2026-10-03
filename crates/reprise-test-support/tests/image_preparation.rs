//! Upload a saved image and confirm it exists through the same Docker socket.
//!
//! Returns without contacting Docker when `REPRISE_DOCKER_SOCKET` or
//! `REPRISE_TEST_IMAGE_ARCHIVE` is unset.
use std::time::Duration;

use bollard::Docker;
use reprise_test_support::{DockerEndpoint, ImagePreparation};

#[tokio::test]
async fn uploaded_image_is_visible_through_docker_socket() {
    let Ok(socket) = std::env::var("REPRISE_DOCKER_SOCKET") else {
        return;
    };
    let Some(archive) = std::env::var_os("REPRISE_TEST_IMAGE_ARCHIVE") else {
        return;
    };

    let endpoint = DockerEndpoint::connect(&socket, Duration::from_secs(120))
        .await
        .expect("connect to test Docker socket");
    let prepared = endpoint
        .ensure_image(archive)
        .await
        .expect("prepare test image");
    assert_eq!(prepared.preparation, ImagePreparation::Loaded);
    let image_id = prepared.id.as_str();

    let docker = Docker::connect_with_unix(&socket, 120, bollard::API_DEFAULT_VERSION)
        .expect("connect to test Docker socket")
        .negotiate_version()
        .await
        .expect("negotiate Docker version");

    let image = docker
        .inspect_image(image_id)
        .await
        .expect("uploaded image must exist on the Docker endpoint");
    assert_eq!(image.id.as_deref(), Some(image_id));
}
