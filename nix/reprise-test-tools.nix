{ lib, buildGoModule }:

buildGoModule {
  pname = "reprise-test-tools";
  version = "0.1.0";

  # Keep workload builds independent of the Rust workspace.
  src = lib.cleanSource ../test-tools;
  vendorHash = "sha256-m5mBubfbXXqXKsygF5j7cHEY+bXhAMcXUts5KBKoLzM=";
  subPackages = [ "cmd/memory-state" ];

  env.CGO_ENABLED = "0";
  ldflags = [
    "-s"
    "-w"
  ];
}
