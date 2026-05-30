# `goosed` — the Goose Desktop backend server (the `goose-server` crate).
{
  lib,
  rustPlatform,
  fetchurl,
  pkg-config,
  protobuf,
  clang,
  cmake,
  openssl,
  cacert,
  libxcb,
  dbus,
  libsecret,
  libclang,
  gooseSrc,
}: let
  workspaceCargo = builtins.fromTOML (builtins.readFile "${gooseSrc}/Cargo.toml");
  inherit (workspaceCargo.workspace.package) version;
  cargoLock = builtins.fromTOML (builtins.readFile "${gooseSrc}/Cargo.lock");
  rustyV8Version = (builtins.head (builtins.filter (pkg: pkg.name == "v8") cargoLock.package)).version;
  rustyV8Archive = fetchurl {
    url = "https://github.com/denoland/rusty_v8/releases/download/v${rustyV8Version}/librusty_v8_release_x86_64-unknown-linux-gnu.a.gz";
    sha256 = "sha256-chV1PAx40UH3Ute5k3lLrgfhih39Rm3KqE+mTna6ysE=";
  };
in
  rustPlatform.buildRustPackage {
    pname = "goosed";
    inherit version;
    src = gooseSrc;
    cargoLock = {
      lockFile = "${gooseSrc}/Cargo.lock";
      outputHashes = {
        "cudaforge-0.1.6" = "sha256-w0e/mfx08BkphDEFEWxuyxyZu/gHiG0m6RHx+3BLzDY=";
      };
    };
    LIBCLANG_PATH = "${libclang.lib}/lib";
    RUSTY_V8_ARCHIVE = rustyV8Archive;

    cargoBuildFlags = [
      "--package"
      "goose-server"
      "--bin"
      "goosed"
    ];

    nativeBuildInputs = [
      pkg-config
      protobuf
      clang
      cmake
    ];

    buildInputs = [
      openssl
      cacert
      libxcb
      dbus
      libsecret
    ];

    doCheck = false;

    meta = {
      description = "Goose Desktop backend server";
      homepage = "https://github.com/aaif-goose/goose";
      license = lib.licenses.asl20;
      mainProgram = "goosed";
      platforms = ["x86_64-linux"];
    };
  }
