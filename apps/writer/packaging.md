# Writer packaging

| Platform | Preview artifact                                                | `recite://` links                                    |
| -------- | --------------------------------------------------------------- | ---------------------------------------------------- |
| Linux    | Flatpak; `.deb` for the Ubuntu CI baseline                      | Desktop entry; later launches reach the open window  |
| macOS    | Apple Silicon and Intel `.app`/`.dmg`                           | Not registered until the app handles URL-open events |
| Windows  | Current-user NSIS installer                                     | Not registered until uninstall can check ownership   |
| Nix      | Flake packages for Linux and macOS ([nix](../../nix/README.md)) | Linux desktop entry                                  |

The application ID is `io.github.plethu.recite`. The icon source is
`packaging/icons/recite-writer.svg`.

## Native packages

Packages are built with cargo-packager 0.11.8:

```sh
mise -E packaging install cargo:cargo-packager
mise -E packaging exec -- python tests/writer-packaging/check.py
mise -E packaging exec -- python scripts/package-writer.py --check-config
mise -E packaging exec -- python scripts/package-writer.py            # writes target/writer-packages/
mise -E packaging exec -- python scripts/check-writer-package.py target/writer-packages/linux
mise -E packaging exec -- python scripts/check-writer-desktop-links.py /path/to/extracted-deb
```

`--binary /path/to/recite-writer` stages an existing build for a quick packaging smoke. Build Linux
packages on the declared Ubuntu baseline; newer local libraries don't establish older-system
support.

## Flatpak

Needs `flatpak-builder`, `dbus-run-session`, `org.gnome.Platform//50`, `org.gnome.Sdk//50` and
`org.freedesktop.Sdk.Extension.rust-stable//25.08`. Install them into a throwaway profile
(`XDG_DATA_HOME=$PWD/target/writer-flatpak-profile`) rather than your user profile.

```sh
python3 scripts/check-writer-flatpak.py              # metadata and Cargo sources, no SDK needed
python3 scripts/check-writer-flatpak.py --builder
scripts/package-writer-flatpak.sh /path/to/build-output
```

When `apps/writer/Cargo.lock` changes, regenerate the offline sources with
[flatpak-cargo-generator](https://github.com/flatpak/flatpak-builder-tools/tree/41c20aa10819cdb2a4f3ca171758a96d1955c018/cargo):

```sh
python3 flatpak-cargo-generator.py apps/writer/Cargo.lock -o apps/writer/packaging/flatpak/cargo-sources.json
```

The manifest grants host filesystem access so projects can live anywhere, and
`org.freedesktop.Flatpak` so configured editors and producers run on the host through `flatpak-spawn
--host`.

## Desktop links

An installed package must register `recite://`, open the linked project before resolving its
location, deliver later links to the running window through the same draft guards as Back/Forward,
treat the URL as data (never shell input), and survive upgrade while uninstall removes only its own
registration.
