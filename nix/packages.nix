{ pkgs }:
let
  inherit (pkgs) lib stdenv;
  rustVersion = (builtins.fromTOML (builtins.readFile ../.mise.toml)).tools.rust.version;
  cliVersion = (builtins.fromTOML (builtins.readFile ../Cargo.toml)).workspace.package.version;
  writerVersion = (builtins.fromTOML (builtins.readFile ../apps/writer/Cargo.toml)).workspace.package.version;
  rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default;
  rustPlatform = pkgs.makeRustPlatform {
    cargo = rustToolchain;
    rustc = rustToolchain;
  };
  source = lib.cleanSourceWith {
    src = ../.;
    filter = path: type:
      let
        root = toString ../.;
        relative = lib.removePrefix "${root}/" (toString path);
        top = builtins.head (lib.splitString "/" relative);
      in
      path == root || (
        builtins.elem top [
          ".cargo" "Cargo.toml" "Cargo.lock" "apps" "assets" "crates" "editors"
          "fixtures" "schemas" "README.md" "LICENSE" "LICENSE-MIT" "LICENSE-APACHE"
        ]
        && relative != "apps/writer/packaging/flatpak"
        && !(lib.hasPrefix "apps/writer/packaging/flatpak/" relative)
        && !(builtins.elem (baseNameOf path) [ "target" "node_modules" ".direnv" ])
      );
  };
  skiaRepo = pkgs.fetchFromGitHub {
    owner = "rust-skia";
    repo = "skia";
    rev = "41382841f36aa208e7433f3c46d14237a1b15054"; # m152-0.100.0
    hash = "sha256-h1N5drad9FPGsdI1lzFWa5q2JDyAPuZ6w3ieCV6NtWs=";
  };
  # Skia's DEPS pins these sources. The system-library switches below cover
  # the other active third-party dependencies, but these three remain in the
  # GN graph for Writer's Wuffs and Vulkan-enabled Skia build.
  skiaExternals = pkgs.linkFarm "recite-skia-externals" ([
    {
      name = "wuffs";
      path = pkgs.fetchFromGitHub {
        owner = "google";
        repo = "wuffs-mirror-release-c";
        rev = "e3f919ccfe3ef542cfc983a82146070258fb57f8";
        hash = "sha256-373d2F/STcgCHEq+PO+SCHrKVOo6uO1rqqwRN5eeBCw=";
      };
    }
    {
      name = "vulkanmemoryallocator";
      path = pkgs.fetchFromGitHub {
        owner = "GPUOpen-LibrariesAndSDKs";
        repo = "VulkanMemoryAllocator";
        rev = "eb744ea7a2b17040121b4bbb4d6f9e8a77e3cae7";
        hash = "sha256-LBZJcom7G7maF9wpUVeVEJQAJwGy6365INk3VD0/0PM=";
      };
    }
    {
      name = "spirv-cross";
      path = pkgs.fetchFromGitHub {
        owner = "KhronosGroup";
        repo = "SPIRV-Cross";
        rev = "b8fcf307f1f347089e3c46eb4451d27f32ebc8d3";
        hash = "sha256-H43M9DXfEuyKuvo6rjb5k0KEbYOSFodbPJh8ZKY4PQg=";
      };
    }
  ] ++ lib.optionals stdenv.hostPlatform.isDarwin [
    {
      # Skia's macOS build embeds FreeType even with system libraries enabled.
      name = "freetype";
      path = pkgs.fetchgit {
        url = "https://chromium.googlesource.com/chromium/src/third_party/freetype2.git";
        rev = "264b5fbf5b912b39f98d038bf75d39be0a73f21b";
        hash = "sha256-RRoTXV063rJBTL/JkVXuRZiNbPZLjtUP/kvt6XybG/k=";
      };
    }
  ]);
  skiaSource = pkgs.runCommand "recite-skia-source" { } ''
    cp -R ${skiaRepo} "$out"
    chmod -R u+w "$out"
    # Skia's system HarfBuzz rule assumes a FHS include tree.
    substituteInPlace "$out/third_party/harfbuzz/BUILD.gn" \
      --replace-fail /usr/include/harfbuzz ${pkgs.harfbuzz.dev}/include/harfbuzz
    ln -s ${skiaExternals} "$out/third_party/externals"
  '';
  writerEnv = {
    SKIA_SOURCE_DIR = "${skiaSource}";
    SKIA_GN_COMMAND = "${pkgs.gn}/bin/gn";
    SKIA_NINJA_COMMAND = "${pkgs.ninja}/bin/ninja";
    SKIA_USE_SYSTEM_LIBRARIES = "1";
  };
  # Skia's GN system WebP target names all three libraries. The bindings only
  # emit libwebp; macOS bindings also omit the other system-library links.
  writerExtraLinkFlags = "-lwebpdemux -lwebpmux"
    + lib.optionalString stdenv.hostPlatform.isDarwin
      " -lpng -lz -lharfbuzz -licuuc -licui18n -licuio -lexpat -ljpeg -lwebp";
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
  writerBuildInputs = with pkgs; [
    fontconfig
    freetype
    expat
    libpng
    zlib
    harfbuzz
    icu
    libjpeg_turbo
    libwebp
    rustPlatform.bindgenHook
  ] ++ lib.optionals stdenv.hostPlatform.isLinux linuxLibraries;
  writerNativeInputs = with pkgs; [
    pkg-config
    python3
    gn
    ninja
    makeWrapper
  ] ++ lib.optionals stdenv.hostPlatform.isLinux [ pkgs.patchelf ]
    ++ lib.optionals stdenv.hostPlatform.isDarwin [ pkgs.cctools.libtool ];
  recite = rustPlatform.buildRustPackage {
    pname = "recite";
    version = cliVersion;
    src = source;
    cargoLock.lockFile = ../Cargo.lock;
    cargoBuildFlags = [ "-p" "recite-cli" ];
    cargoTestFlags = [ "-p" "recite-cli" ];
    doCheck = false;
    meta = {
      description = "Deterministic dialogue compiler CLI";
      mainProgram = "recite";
      license = with lib.licenses; [ mit asl20 ];
      platforms = lib.platforms.linux ++ lib.platforms.darwin;
    };
  };
  recite-writer = rustPlatform.buildRustPackage.override { stdenv = pkgs.clangStdenv; } {
    pname = "recite-writer";
    version = writerVersion;
    src = source;
    cargoRoot = "apps/writer";
    buildAndTestSubdir = "apps/writer";
    cargoLock.lockFile = ../apps/writer/Cargo.lock;
    cargoBuildFlags = [ "-p" "recite-writer" ];
    doCheck = false;
    nativeBuildInputs = writerNativeInputs;
    buildInputs = writerBuildInputs;
    # Ninja is invoked by Skia's build script, not for the Cargo package itself.
    dontUseNinjaBuild = true;
    dontUseNinjaCheck = true;
    dontUseNinjaInstall = true;
    env = writerEnv;
    preBuild = ''
      export NIX_LDFLAGS="$NIX_LDFLAGS ${writerExtraLinkFlags}"
    '';
    postInstall = lib.optionalString stdenv.hostPlatform.isLinux ''
      install -Dm644 apps/writer/packaging/icons/recite-writer.png \
        "$out/share/icons/hicolor/512x512/apps/recite-writer.png"
      install -Dm644 ${./recite-writer.desktop} \
        "$out/share/applications/recite-writer.desktop"
    '';
    postFixup = lib.optionalString stdenv.hostPlatform.isLinux ''
      patchelf --add-rpath ${lib.makeLibraryPath linuxLibraries} \
        "$out/bin/recite-writer"
      wrapProgram "$out/bin/recite-writer" \
        --suffix PATH : ${lib.makeBinPath [ pkgs.gettext pkgs.xdg-utils ]}
    '' + lib.optionalString stdenv.hostPlatform.isDarwin ''
      wrapProgram "$out/bin/recite-writer" \
        --suffix PATH : ${lib.makeBinPath [ pkgs.gettext ]}
    '';
    meta = {
      description = "Recite dialogue writer";
      mainProgram = "recite-writer";
      license = with lib.licenses; [ mit asl20 ];
      platforms = lib.platforms.linux ++ lib.platforms.darwin;
    };
  };
in
{
  inherit pkgs rustToolchain writerEnv writerExtraLinkFlags writerNativeInputs writerBuildInputs recite recite-writer;
}
