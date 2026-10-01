{
  buildEnv,
  busybox,
  tini,
  reprise-test-tools,
}:

# Shared filesystem layout for the Nix image and the Dockerfile.
buildEnv {
  name = "reprise-test-runtime";
  paths = [
    reprise-test-tools
    busybox
    tini
  ];
  pathsToLink = [ "/bin" ];
}
