{
  pkgs,
  crane,
}: let
  inherit (pkgs) lib stdenv;
  rustVersion = (builtins.fromTOML (builtins.readFile ../.config/mise/config.toml)).tools.rust.version;
  cliVersion = (builtins.fromTOML (builtins.readFile ../Cargo.toml)).workspace.package.version;
  writerManifest = builtins.fromTOML (builtins.readFile ../apps/writer/Cargo.toml);
  writerVersion = writerManifest.workspace.package.version;
  rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default;
  rustPlatform = pkgs.makeRustPlatform {
    cargo = rustToolchain;
    rustc = rustToolchain;
  };
  source = lib.cleanSourceWith {
    src = ../.;
    filter = path: type: let
      root = toString ../.;
      relative = lib.removePrefix "${root}/" (toString path);
      top = builtins.head (lib.splitString "/" relative);
    in
      path
      == root
      || (
        builtins.elem top [
          ".cargo"
          "Cargo.toml"
          "Cargo.lock"
          "apps"
          "assets"
          "crates"
          "editors"
          "fixtures"
          "schemas"
          "tools"
          "README.md"
          "LICENSE"
          "LICENSE-MIT"
          "LICENSE-APACHE"
        ]
        && relative != "apps/writer/packaging/flatpak"
        && !(lib.hasPrefix "apps/writer/packaging/flatpak/" relative)
        && !(builtins.elem (baseNameOf path) ["target" "node_modules" ".direnv"])
      );
  };
  linuxLibraries = with pkgs; [
    libGL
    vulkan-loader
    libxkbcommon
    wayland
    libx11
    libxcursor
    libxi
    libxrandr
    libxcb
  ];
  writerBuildInputs = lib.optionals stdenv.hostPlatform.isLinux ([pkgs.fontconfig] ++ linuxLibraries);
  writerNativeInputs =
    [pkgs.pkg-config pkgs.makeWrapper]
    ++ lib.optionals stdenv.hostPlatform.isLinux [pkgs.patchelf];
  recite = rustPlatform.buildRustPackage {
    pname = "recite";
    version = cliVersion;
    src = source;
    cargoLock.lockFile = ../Cargo.lock;
    cargoBuildFlags = ["-p" "recite-cli"];
    cargoTestFlags = ["-p" "recite-cli"];
    doCheck = false;
    meta = {
      description = "Deterministic dialogue compiler CLI";
      mainProgram = "recite";
      license = with lib.licenses; [mit asl20];
      platforms = lib.platforms.linux ++ lib.platforms.darwin;
    };
  };
  writerCraneLib = ((crane.mkLib pkgs).overrideToolchain rustToolchain).overrideScope (_: _: {
    stdenvSelector = p: p.clangStdenv;
  });
  recite-writer = let
    writerArgs = {
      pname = "recite-writer";
      version = writerVersion;
      src = source;
      cargoLock = ../apps/writer/Cargo.lock;
      # Resolve vendoring without building the generated dummy source during evaluation.
      cargoVendorDir = writerCraneLib.vendorCargoDeps {
        src = source;
        cargoLock = ../apps/writer/Cargo.lock;
      };
      cargoExtraArgs = "--locked -p recite-writer";
      postUnpack = ''sourceRoot="$sourceRoot/apps/writer"'';
      doCheck = false;
      nativeBuildInputs = writerNativeInputs;
      buildInputs = writerBuildInputs;
      env.CARGO_TARGET_DIR = "target";
    };
    writerCargoArtifacts = writerCraneLib.buildDepsOnly ((builtins.removeAttrs writerArgs ["src"])
      // {
        dummySrc = writerCraneLib.mkDummySrc {
          src = source;
          cargoLock = ../apps/writer/Cargo.lock;
          extraDummyScript = ''
            cp ${../apps/writer/Cargo.lock} "$out/apps/writer/Cargo.lock"
            # Upstream toolkit crates consume these patches' public APIs.
            # Stub application crates, but compile the actual dependency patches.
            cp -Rf ${../apps/writer/vendor}/. "$out/apps/writer/vendor/"
          '';
        };
        buildPhaseCargoCommand = "cargo build --profile release ${writerArgs.cargoExtraArgs}";
      });
  in
    writerCraneLib.buildPackage (writerArgs
      // {
        cargoArtifacts = writerCargoArtifacts;
        postInstall = lib.optionalString stdenv.hostPlatform.isLinux ''
          install -Dm644 packaging/icons/recite-writer.png \
            "$out/share/icons/hicolor/512x512/apps/recite-writer.png"
          install -Dm644 ${./recite-writer.desktop} \
            "$out/share/applications/recite-writer.desktop"
        '';
        postFixup =
          lib.optionalString stdenv.hostPlatform.isLinux ''
            patchelf --add-rpath ${lib.makeLibraryPath linuxLibraries} \
              "$out/bin/recite-writer"
            wrapProgram "$out/bin/recite-writer" \
              --suffix PATH : ${lib.makeBinPath [pkgs.gettext pkgs.xdg-utils]}
          ''
          + lib.optionalString stdenv.hostPlatform.isDarwin ''
            wrapProgram "$out/bin/recite-writer" \
              --suffix PATH : ${lib.makeBinPath [pkgs.gettext]}
          '';
        meta = {
          description = "Recite dialogue writer";
          mainProgram = "recite-writer";
          license = with lib.licenses; [mit asl20];
          platforms = lib.platforms.linux ++ lib.platforms.darwin;
        };
      });
in {
  inherit pkgs rustToolchain writerNativeInputs writerBuildInputs recite recite-writer;
}
