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
      in
      {
        packages = {
          inherit reprise-test-tools;
          default = reprise-test-tools;
        };
      }
    );
}
