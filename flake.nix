{
  description = "Reprise test workloads";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        reprise-test-tools = pkgs.callPackage ./nix/reprise-test-tools.nix { };
        # Export dependency metadata; Rust generates root and store-mount configuration on the host
        # where runsc will execute, before the benchmark starts measuring.
        reprise-test-closure = pkgs.closureInfo { rootPaths = [ reprise-test-tools ]; };
      in
      {
        packages = {
          inherit reprise-test-tools reprise-test-closure;
          default = reprise-test-tools;
        } // pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          runsc = pkgs.gvisor;
        };
      }
    );
}
