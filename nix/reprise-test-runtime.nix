{
  buildEnv,
  busybox,
  tini,
  reprise-test-tools,
}:

# Filesystem layout assembled into the Nix test image.
buildEnv {
  name = "reprise-test-runtime";
  paths = [
    reprise-test-tools
    busybox
    tini
  ];
  pathsToLink = [ "/bin" ];
}
