{
  description = "Recite CLI and Writer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f system);
      packagesFor = system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
        in
        import ./nix/packages.nix { inherit pkgs; };
    in
    {
      packages = eachSystem (system:
        let packages = packagesFor system;
        in {
          inherit (packages) recite recite-writer;
          default = packages.recite-writer;
        });

      apps = eachSystem (system:
        let packages = packagesFor system;
        in {
          recite = {
            type = "app";
            program = "${packages.recite}/bin/recite";
          };
          recite-writer = {
            type = "app";
            program = "${packages.recite-writer}/bin/recite-writer";
          };
          default = self.apps.${system}.recite-writer;
        });

      checks = eachSystem (system:
        let packages = packagesFor system;
        in {
          inherit (packages) recite recite-writer;
          cli-smoke = packages.pkgs.runCommand "recite-cli-smoke" { } ''
            ${packages.recite}/bin/recite --help > /dev/null
            ${packages.recite}/bin/recite --version > /dev/null
            touch "$out"
          '';
          writer-smoke = packages.pkgs.runCommand "recite-writer-smoke" { } ''
            ${packages.recite-writer}/bin/recite-writer --help > /dev/null
            ${packages.recite-writer}/bin/recite-writer --version > /dev/null
            touch "$out"
          '';
        });

      devShells = eachSystem (system:
        let
          packages = packagesFor system;
          pkgs = packages.pkgs;
        in {
          default = pkgs.mkShell.override { stdenv = pkgs.clangStdenv; } {
            packages = [
              packages.rustToolchain
              pkgs.just
              pkgs.gettext
            ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.xdg-utils ]
              ++ packages.writerNativeInputs ++ packages.writerBuildInputs;
            inherit (packages.writerEnv) SKIA_SOURCE_DIR SKIA_GN_COMMAND SKIA_NINJA_COMMAND SKIA_USE_SYSTEM_LIBRARIES;
            shellHook = ''
              export NIX_LDFLAGS="$NIX_LDFLAGS ${packages.writerExtraLinkFlags}"
            '';
          };
        });
    };
}
