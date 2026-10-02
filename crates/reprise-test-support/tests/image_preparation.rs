//! Upload a saved image and confirm it exists through the same Docker socket.
use std::time::Duration;

use bollard::Docker;
use reprise_test_support::{DockerTestEnvironment, ImagePreparation};

#[tokio::test]
#[ignore = "requires a saved test image and a Docker endpoint without that image"]
async fn uploaded_image_is_visible_through_docker_socket() {
    let socket = std::env::var("REPRISE_DOCKER_SOCKET").unwrap();
    let archive = std::env::var_os("REPRISE_TEST_IMAGE_ARCHIVE").unwrap();

    let environment = DockerTestEnvironment::connect(&socket, Duration::from_secs(120)).await;
    let prepared = environment.ensure_image(archive).await;
    assert_eq!(prepared.preparation, ImagePreparation::Loaded);
    let image_id = prepared.image.id.unwrap();

    let docker = Docker::connect_with_unix(&socket, 120, bollard::API_DEFAULT_VERSION)
        .expect("connect to test Docker socket")
        .negotiate_version()
        .await
        .expect("negotiate Docker version");

    let image = docker
        .inspect_image(&image_id)
        .await
        .expect("uploaded image must exist on the Docker endpoint");
    assert_eq!(image.id.as_deref(), Some(image_id.as_str()));
}
