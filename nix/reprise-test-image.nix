{
  dockerTools,
  reprise-test-runtime,
}:

dockerTools.buildLayeredImage {
  name = "reprise-test-image";
  # Omit tag: dockerTools derives it from the Nix output hash, not "latest".
  created = "1970-01-01T00:00:01Z";
  contents = [ reprise-test-runtime ];

  config = {
    Env = [ "PATH=/bin" ];
    WorkingDir = "/";
    # tini reaps detached test processes; it is not the sandbox daemon.
    Entrypoint = [
      "/bin/tini"
      "--"
    ];
    Cmd = [
      "/bin/sleep"
      "infinity"
    ];
  };
}
