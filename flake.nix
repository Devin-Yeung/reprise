{
  description = "Reprise test workloads";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";
    flake-utils.url = "github:numtide/flake-utils";
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      devshell,
      ...
    }:
    # x86_64-darwin is excluded: Nixpkgs 26.11 dropped support for it and fails to evaluate.
    flake-utils.lib.eachSystem
      [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ]
      (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ devshell.overlays.default ];
          };
          reprise-test-tools = pkgs.callPackage ./nix/reprise-test-tools.nix { };
          reprise-test-runtime = pkgs.callPackage ./nix/reprise-test-runtime.nix {
            inherit reprise-test-tools;
          };
          # Linux-only: gVisor publishes binaries only for Linux.
          runsc = pkgs.callPackage ./nix/gvisor.nix { };
        in
        {
          packages = {
            inherit reprise-test-tools reprise-test-runtime;
            default = reprise-test-tools;
          }
          // pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
            inherit runsc;
          };

          # runsc only works on Linux, so the shell is Linux-only as well.
          devShells = pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
            default = pkgs.devshell.mkShell {
              name = "reprise";
              packages = [
                reprise-test-runtime
                runsc
              ];
              # Integration tests resolve these explicitly instead of relying
              # on the PATH / `./result` fallbacks in the test support code.
              env = [
                {
                  name = "REPRISE_RUNSC_BIN";
                  value = "${runsc}/bin/runsc";
                }
                {
                  name = "REPRISE_TEST_RUNTIME";
                  value = reprise-test-runtime;
                }
              ];
            };
          };
        }
      );
}
