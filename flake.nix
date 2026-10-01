{
  description = "Reprise test tools and reproducible test images";

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
        reprise-test-image = pkgs.callPackage ./nix/reprise-test-image.nix {
          inherit reprise-test-tools;
        };
      in
      {
        packages = {
          inherit reprise-test-tools;
          default = reprise-test-tools;
        }
        // pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          inherit reprise-test-image;
          default = reprise-test-image;
        };
      }
    );
}
