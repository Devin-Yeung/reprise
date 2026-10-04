{ lib, buildGoModule }:

buildGoModule {
  pname = "reprise-test-tools";
  version = "0.1.0";

  # Keep workload builds independent of the Rust workspace.
  src = lib.cleanSource ../test-tools;
  vendorHash = "sha256-YRAUYGyv9Nfy+1nmQKzG8CDVpi3u7C2tZC61dogtYx0=";
  subPackages = [ "cmd/memory-state" ];

  env.CGO_ENABLED = "0";
  ldflags = [ "-s" "-w" ];
}
