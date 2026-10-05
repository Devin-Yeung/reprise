{
  bash,
  buildEnv,
  closureInfo,
  reprise-test-tools,
  runCommand,
}:

let
  # This profile is the intentionally small command surface of the test
  # runtime. buildEnv links each selected package's bin directory and refuses
  # conflicting command names, so adding a tool cannot silently change what a
  # downstream caller executes.
  command-profile = buildEnv {
    name = "reprise-test-command-profile";
    paths = [
      bash
      reprise-test-tools
    ];
    pathsToLink = [ "/bin" ];
  };

  # The profile is the root, rather than merely metadata beside the closure:
  # this keeps every target of its symlinks alive and makes it available at the
  # same /nix/store path inside the sandbox.
  runtime-closure = closureInfo {
    rootPaths = [ command-profile ];
  };

  manifest = builtins.toJSON {
    format_version = 1;
    command_profile = toString command-profile;
  };
in
runCommand "reprise-test-runtime" { } ''
  mkdir -p "$out"
  cp ${runtime-closure}/store-paths "$out/store-paths"
  printf '%s\n' '${manifest}' > "$out/manifest.json"
''
