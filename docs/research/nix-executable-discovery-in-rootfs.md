# Making Nix executables discoverable in an OCI rootfs

## Decision-oriented finding

For a fixed Reprise workload, keep `process.args[0]` as the absolute
`/nix/store/.../bin/<program>` path and mount its complete closure at those
same paths. This is already the contract of `Workload`. It is not a workaround:
the OCI runtime specification gives the first process argument `execvp`-style
file semantics, and Nix's own `dockerTools` examples also use an absolute
store path when the command is assembled from a package.

Add a conventional rootfs-facing command path only when a caller needs to name
tools independently of their store derivation, such as an interactive shell,
scripts with bare commands, or a stable execution API. Build that interface at
image/rootfs construction time as a small symlink tree (normally
`/bin/<program>`), rather than discovering an executable's location at runtime.
If a program is addressed by a bare name, supply an explicit `PATH` containing
that directory. Do not treat `PATH` or `/bin` as something supplied by OCI.

This is a design recommendation derived from the cited specifications and Nix
implementation/documentation. It has not been exercised against `runsc` in
this repository.

## What the standards require

The OCI runtime spec defines `process.args[0]` with the same file-resolution
semantics as `execvp`, and defines `process.env` as an ordinary POSIX
environment. In particular, the specification does not require the first
argument to be an absolute path, and it does not supply a default `PATH`.
Therefore a bare command is a policy choice that needs a `PATH` and a matching
filesystem interface; an absolute store path needs neither.

The OCI image spec separates this from image defaults: `config.Env`,
`Entrypoint`, and `Cmd` are defaults, with `Cmd[0]` interpreted as the
executable when there is no entrypoint. Image construction can consequently
set a conventional command path or store path, while direct bundle generation
must write the equivalent runtime `process` fields itself.

Sources: [OCI Runtime Specification, process `args` and `env`](https://github.com/opencontainers/runtime-spec/blob/main/config.md#process), [OCI Image Specification, config](https://github.com/opencontainers/image-spec/blob/main/config.md#properties).

## Nix practices

`dockerTools` supports both relevant layouts:

- Its simple image example combines `buildEnv { paths = [ redis ]; pathsToLink = [ "/bin" ]; }` with `config.Cmd = [ "/bin/redis-server" ]`. That is an explicit, conventional command namespace in the rootfs.
- Its layered-image example supplies a package as `contents` and runs
  `/bin/hello`. `dockerTools` documents that the final layer contains links
  into the actual store paths, while the store objects themselves appear in
  preceding layers. The generated `/bin` path is therefore a stable facade,
  not a copied binary.
- When the command can be fixed during evaluation, Nix's documented compact
  form is `config.Cmd = [ "${lib.getExe hello}" ]`; that is the exact,
  absolute store executable. This mirrors Reprise's current model.

The documented `includeStorePaths = false` escape hatch is particularly
relevant: Nix warns that image links alone will not run unless other tooling
inserts the store paths, giving host-store bind mounts as an example. Reprise's
read-only mounts of the complete closure at original `/nix/store` destinations
are precisely that missing half. A symlink farm works only while its store
targets remain mounted and retained from garbage collection.

Sources: [Nixpkgs `dockerTools.buildImage` example](https://github.com/NixOS/nixpkgs/blob/master/doc/build-helpers/images/dockertools.section.md#building-a-docker-image), [Nixpkgs layered-image links and `includeStorePaths`](https://github.com/NixOS/nixpkgs/blob/master/doc/build-helpers/images/dockertools.section.md#streamlayeredimage), [Nixpkgs `symlinkJoin` implementation documentation](https://github.com/NixOS/nixpkgs/blob/master/pkgs/build-support/trivial-builders/default.nix#L3029-L3126).

## Implications for Reprise

| Need | Rootfs interface | Process configuration |
| --- | --- | --- |
| The known initial workload | Mount the complete closure at `/nix/store/...` | Absolute `args[0]` in the store, as today. |
| Stable commands for `runsc exec` callers | Add a generated `/bin` symlink tree for the chosen command set. | Use absolute `/bin/<program>` paths, or document `PATH=/bin` for bare names. |
| Interactive/debug shell | Add `/bin/sh` explicitly and the tools it needs, plus a deliberate `PATH`. | Start `/bin/sh`, not an assumed shell. |

The second and third rows are product-interface decisions, not prerequisites
for cold boot. A generated facade needs collision policy: two packages may
both provide the same `bin` name. Limit the facade to explicitly selected
programs, or make collisions a build failure. The Nix closure remains the
source of runtime dependencies; the facade should not become a second,
hand-maintained dependency list.

`symlinkJoin` or `buildEnv` are good Nix-side tools for producing the facade:
both yield a store object with normal top-level directories whose members link
to input derivations. `buildEnv` is the profile-style choice and can limit the
published subtree with `pathsToLink = [ "/bin" ]`; `symlinkJoin` is the
lighter-weight link-tree builder. A package wrapper is different: Nix's
`makeWrapper` can give that package's process a private dependency `PATH`, but
it does not publish an image-wide command namespace.

Sources: [Nixpkgs `buildEnv` manual](https://nixos.org/manual/nixpkgs/unstable/#sec-buildenv), [Nixpkgs `symlinkJoin` manual](https://nixos.org/manual/nixpkgs/unstable/#ssec-symlinkJoin), [Nixpkgs `makeWrapper` manual](https://nixos.org/manual/nixpkgs/unstable/#fun-makeWrapper).

Reprise can then either materialize that tree in the shared rootfs, or add it
as another read-only rootfs layer. That is a future implementation choice. It
should be validated with a Linux `runsc` cold boot, `runsc exec`, and
checkpoint/restore before replacing the current absolute-path contract.
