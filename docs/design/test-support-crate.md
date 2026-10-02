# reprise-test-support

Status: API skeleton; methods are not implemented yet.

## Fixture integration

Prepare the image before starting the daemon or measuring restore time. Configure
its `FixedTemplate.image` with the returned immutable ID and use the same Docker
endpoint. The acceptance scenario still runs through `SandboxService`; image
preparation is fixture setup, not evidence of checkpoint capability.

Intended use after implementation:

```rust
let environment = DockerTestEnvironment::connect(
    socket_path,
    Duration::from_secs(120),
).await?;

let prepared = environment.ensure_image(archive_path).await?;
let image_id = prepared.image.id.as_deref()
    .expect("preparation returns an image ID");

// Configure FixedTemplate.image with image_id on the same endpoint.
```

## Implementation notes

- A Docker save tar may have multiple tags for its single manifest image entry.
  Bound manifest/config reads, reject duplicate config entries and unsafe or
  ambiguous config references, and parse without extracting files to disk.
- Hash the original config bytes, not reserialized JSON. Layer hashes are not
  needed to look up the image by ID.
- Retain the same opened file for validation and upload, checking for detectable
  changes. Schedule blocking archive work off the async executor and include it
  in the preparation budget.

When implemented, exercise both paths against an explicit Docker socket: the
first call uploads, and the second uses the cached image.
