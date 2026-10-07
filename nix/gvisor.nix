{
  lib,
  makeWrapper,
  stdenvNoCC,
  stdenv,
  fetchurl,
  iproute2,
  iptables,
  procps,
}:

let
  version = "20260928.0";

  arch = if stdenv.hostPlatform.isAarch64 then "aarch64" else "x86_64";

  hashes = {
    x86_64 = "sha256-8+2RMbwlIlnfFQ4nAVQYAYj531a3OiMSlAMl4fUiotY=";
    aarch64 = "sha256-t+EdJ8vWk3Dtet22wNHDPnDlehU87le1/cU0S/MDx+s=";
  };
in
stdenvNoCC.mkDerivation {
  pname = "gvisor";
  inherit version;

  src = fetchurl {
    url = "https://storage.googleapis.com/gvisor/releases/release/${version}/${arch}/gvisor.tar.bz2";
    hash = hashes.${arch};
  };

  sourceRoot = ".";

  nativeBuildInputs = [ makeWrapper ];

  installPhase = ''
    runHook preInstall
    install -Dm755 runsc $out/bin/runsc
    install -Dm755 containerd-shim-runsc-v1 $out/bin/containerd-shim-runsc-v1
    cp -r gvisor-bin $out/bin/gvisor-bin
    # Needed for the 'runsc do' subcommand
    wrapProgram $out/bin/runsc \
      --prefix PATH : ${
        lib.makeBinPath [
          iproute2
          iptables
          procps
        ]
      }
    runHook postInstall
  '';

  meta = {
    description = "Application Kernel for Containers";
    homepage = "https://github.com/google/gvisor";
    license = lib.licenses.asl20;
    sourceProvenance = with lib.sourceTypes; [ binaryNativeCode ];
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "runsc";
  };
}
