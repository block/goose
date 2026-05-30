# Goose Desktop — the Electron UI, bundling the `goosed` backend (./goosed.nix).
#
# Called from the flake as
#   pkgs.callPackage ./nix/goose-desktop { gooseSrc = self; }
# Linux x86_64 only; Goose Desktop is not packaged for other platforms here.
{
  lib,
  callPackage,
  fetchPnpmDeps,
  pnpmConfigHook,
  nodejs_24,
  pnpm_10,
  pkg-config,
  protobuf,
  makeWrapper,
  makeDesktopItem,
  copyDesktopItems,
  electron_41,
  zip,
  gitMinimal,
  python3,
  bash,
  coreutils,
  curl,
  gnused,
  gzip,
  which,
  wtype,
  wl-clipboard,
  stdenvNoCC,
  gooseSrc,
  extraWrapperArgs ? [],
}: let
  stdenv = stdenvNoCC;
  electron = electron_41;
  nodejs = nodejs_24;
  pnpm = pnpm_10.override {inherit nodejs;};

  goosed = callPackage ./goosed.nix {inherit gooseSrc;};

  workspaceCargo = builtins.fromTOML (builtins.readFile "${gooseSrc}/Cargo.toml");
  inherit (workspaceCargo.workspace.package) version;
in
  stdenv.mkDerivation (finalAttrs: {
    pname = "goose-desktop";
    inherit version;
    src = "${gooseSrc}/ui";

    pnpmDeps = fetchPnpmDeps {
      inherit (finalAttrs) pname version src;
      inherit pnpm;
      fetcherVersion = 3;
      hash = "sha256-xFW9NUN/oR7c7bBwydHksASfPywuIQFP++Y0u7hYTBs=";
    };

    strictDeps = true;

    nativeBuildInputs = [
      pnpmConfigHook
      makeWrapper
      copyDesktopItems
      nodejs
      pnpm
      pkg-config
      protobuf
      python3
      gitMinimal
      zip
    ];

    env = {
      ELECTRON_SKIP_BINARY_DOWNLOAD = "1";
    };

    # The packaged app lives in the Nix store and cannot update itself.
    postPatch = ''
      substituteInPlace desktop/src/updates.ts \
        --replace-fail "export const UPDATES_ENABLED = true;" "export const UPDATES_ENABLED = false;"
    '';

    buildPhase = ''
      runHook preBuild

      export HOME=$(mktemp -d)
      export npm_config_nodedir=${electron.headers}
      export ELECTRON_PLATFORM=linux
      export ELECTRON_ARCH=x64

      # The @aaif/goose-sdk workspace package is normally built by a pnpm
      # postinstall hook (skipped here for determinism). Its schema generator
      # reads crates/goose/acp-{schema,meta}.json, which live outside this ui/
      # source tree, so stage them where the generator resolves them
      # (repo root == the parent of this source root) and build the SDK.
      mkdir -p ../crates/goose
      install -Dm644 ${gooseSrc}/crates/goose/acp-schema.json ../crates/goose/acp-schema.json
      install -Dm644 ${gooseSrc}/crates/goose/acp-meta.json ../crates/goose/acp-meta.json
      pnpm --filter @aaif/goose-sdk run build

      # electron-forge resolves the Electron version from node_modules; pin it
      # to the nixpkgs Electron we package against.
      substituteInPlace node_modules/@electron-forge/core-utils/dist/electron-version.js \
        --replace-fail "return version" "return '${electron.version}'"

      # Provide Electron offline as a local zip so the packager does not fetch.
      cp -r ${electron.dist} electron-dist
      chmod -R u+w electron-dist
      pushd electron-dist
      zip -0Xqr ../electron.zip .
      popd
      rm -r electron-dist

      substituteInPlace node_modules/@electron/packager/dist/packager.js \
        --replace-fail "await this.getElectronZipPath(downloadOpts)" "'$(pwd)/electron.zip'"

      install -Dm755 ${lib.getExe goosed} desktop/src/bin/goosed
      patchShebangs desktop/node_modules desktop/node_modules/.bin
      patchShebangs desktop/src/bin

      node desktop/scripts/prepare-platform-binaries.js
      pnpm --dir desktop run generate-api
      pnpm --dir desktop run i18n:compile
      pnpm --dir desktop exec electron-forge package --platform=linux --arch=x64

      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall

      buildDir=$(find desktop/out -maxdepth 1 -mindepth 1 -type d -name '*-linux-x64' | head -n1)
      if [ -z "$buildDir" ]; then
        echo "Goose desktop output not found"
        exit 1
      fi

      mkdir -p $out/opt/goose-desktop
      cp -r "$buildDir/resources" $out/opt/goose-desktop/

      install -Dm644 desktop/src/images/icon.png \
        $out/share/icons/hicolor/256x256/apps/goose-desktop.png
      install -Dm644 desktop/src/images/icon.svg \
        $out/share/icons/hicolor/scalable/apps/goose-desktop.svg

      makeWrapper ${lib.getExe electron} $out/bin/goose-desktop \
        --run "cd $out/opt/goose-desktop/resources" \
        --add-flags $out/opt/goose-desktop/resources/app.asar \
        --set ELECTRON_FORCE_IS_PACKAGED 1 \
        --set GOOSED_BINARY $out/opt/goose-desktop/resources/bin/goosed \
        --prefix PATH : ${lib.makeBinPath [
        bash
        python3
        coreutils
        curl
        gnused
        gzip
        which
        wtype
        wl-clipboard
      ]} \
        ${lib.escapeShellArgs extraWrapperArgs} \
        --add-flags "\''${NIXOS_OZONE_WL:+\''${WAYLAND_DISPLAY:+--ozone-platform-hint=auto --enable-features=WaylandWindowDecorations --enable-wayland-ime=true}}" \
        --inherit-argv0

      runHook postInstall
    '';

    desktopItems = [
      (makeDesktopItem {
        name = "goose-desktop";
        desktopName = "Goose";
        comment = "Open source extensible AI agent";
        exec = "goose-desktop %U";
        icon = "goose-desktop";
        categories = [
          "Development"
        ];
        startupWMClass = "Goose";
        mimeTypes = [
          "x-scheme-handler/goose"
        ];
      })
    ];

    passthru = {
      inherit goosed;
    };

    meta = {
      description = "Goose Desktop";
      homepage = "https://github.com/aaif-goose/goose";
      license = lib.licenses.asl20;
      mainProgram = "goose-desktop";
      platforms = ["x86_64-linux"];
    };
  })
