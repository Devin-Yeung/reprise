//! Real Engine checks selected by the integration-tests feature.

use std::time::Duration;

use anyhow::Result;
use reprise_test_support::{ImagePreparation, PrepareError, TestFixture, ensure_image};

const DEADLINE: Duration = Duration::from_secs(120);

#[tokio::test]
async fn registry_reference_is_shared_by_parallel_consumers() -> Result<()> {
    let fixture = TestFixture::from_env().await?;
    fixture.docker.ping().await?;
    eprintln!(
        "prepared {} ({:?})",
        fixture.image.id, fixture.image.preparation
    );
    let (first, second) = tokio::join!(
        ensure_image(&fixture.docker, &fixture.reference, DEADLINE),
        ensure_image(&fixture.docker, &fixture.reference, DEADLINE),
    );
    for prepared in [first?, second?] {
        assert_eq!(
            prepared.id, fixture.image.id,
            "consumers resolved different image IDs"
        );
        assert_eq!(
            prepared.preparation,
            ImagePreparation::Cached,
            "reference was not cached"
        );
    }
    Ok(())
}

#[tokio::test]
async fn nonexistent_registry_digest_fails_preparation() -> Result<()> {
    let fixture = TestFixture::from_env().await?;
    let repository = fixture.reference.split_once('@').unwrap().0;
    // Keep the real registry/repository and request an absent digest. This
    // exercises the Engine's pull error stream without deleting shared images.
    let missing = format!("{repository}@sha256:{}", "0".repeat(64));
    let error = ensure_image(&fixture.docker, &missing, DEADLINE)
        .await
        .expect_err("an absent registry digest must not resolve to a fallback image");
    assert!(
        matches!(error, PrepareError::Engine(_)),
        "expected an Engine/registry rejection, got {error}"
    );
    Ok(())
}
