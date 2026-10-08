{
  description = "Reprise test workloads";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    # x86_64-darwin is excluded: Nixpkgs 26.11 dropped support for it and fails to evaluate.
    flake-utils.lib.eachSystem [
      "aarch64-darwin"
      "aarch64-linux"
      "x86_64-linux"
    ] (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        reprise-test-tools = pkgs.callPackage ./nix/reprise-test-tools.nix { };
        reprise-test-runtime = pkgs.callPackage ./nix/reprise-test-runtime.nix {
          inherit reprise-test-tools;
        };
      in
      {
        packages = {
          inherit reprise-test-tools reprise-test-runtime;
          default = reprise-test-tools;
        } // pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          runsc = pkgs.callPackage ./nix/gvisor.nix { };
        };
      }
    );
}
