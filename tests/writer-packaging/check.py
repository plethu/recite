#!/usr/bin/env python3
"""Validate the three package contracts before an expensive native build."""

import importlib.util
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("package_writer", ROOT / "scripts/package-writer.py")
package_writer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package_writer)
check_spec = importlib.util.spec_from_file_location(
    "check_writer_package", ROOT / "scripts/check-writer-package.py"
)
package_check = importlib.util.module_from_spec(check_spec)
check_spec.loader.exec_module(package_check)


class PackageConfigTests(unittest.TestCase):
    def test_platform_formats_and_registration(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            for target, formats in (
                ("linux", ["deb"]),
                ("macos", ["app", "dmg"]),
                ("windows", ["nsis"]),
            ):
                with self.subTest(target=target):
                    config = package_writer.load_config(target, base, base)
                    self.assertEqual(config["formats"], formats)
                    self.assertEqual(config["version"], "0.0.0")
                    self.assertEqual(config["identifier"], "io.github.plethu.recite")
                    self.assertEqual(config["binaries"], [{"path": "recite-writer", "main": True}])
                    self.assertEqual(
                        bool(config.get("deepLinkProtocols")), target == "linux"
                    )
                    for icon in config["icons"]:
                        self.assertTrue(Path(icon).is_file(), icon)
                    for resource in config["resources"]:
                        self.assertTrue(Path(resource["src"]).is_file())

    def test_linux_receives_one_url(self):
        desktop = (ROOT / "apps/writer/packaging/recite-writer.desktop.hbs").read_text()
        self.assertIn("Exec={{exec}} %u", desktop)
        self.assertIn("MimeType=x-scheme-handler/recite;", desktop)

    def test_icons_have_native_file_headers(self):
        icons = ROOT / "apps/writer/packaging/icons"
        self.assertTrue((icons / "recite-writer.png").read_bytes().startswith(b"\x89PNG\r\n\x1a\n"))
        ico = (icons / "recite-writer.ico").read_bytes()
        self.assertEqual(ico[:4], b"\x00\x00\x01\x00")
        icns = (icons / "recite-writer.icns").read_bytes()
        self.assertEqual(icns[:4], b"icns")
        self.assertEqual(struct.unpack(">I", icns[4:8])[0], len(icns))

    def test_smoke_staging_preserves_cargo_release_binary(self):
        with tempfile.TemporaryDirectory() as temporary:
            target_dir = Path(temporary) / "target"
            release = target_dir / "release" / "recite-writer"
            release.parent.mkdir(parents=True)
            release.write_bytes(b"cargo release sentinel")
            source = Path(temporary) / "debug-writer"
            source.write_bytes(b"smoke binary")
            config = package_writer.load_config("linux", target_dir, Path(temporary))
            staged = package_writer.stage_smoke_binary(config, target_dir, "linux", source)
            self.assertEqual(staged.read_bytes(), b"smoke binary")
            self.assertEqual(release.read_bytes(), b"cargo release sentinel")
            self.assertEqual(Path(config["binariesDir"]), staged.parent)
            self.assertNotEqual(Path(config["binariesDir"]), release.parent)

    @unittest.skipUnless(
        sys.platform.startswith("linux")
        and shutil.which("cc")
        and shutil.which("readelf")
        and (shutil.which("dpkg-deb") or (shutil.which("ar") and shutil.which("bsdtar"))),
        "Debian fixture needs cc, readelf, and archive tools",
    )
    def test_debian_inspection_fixture(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            payload = base / "payload"
            (payload / "DEBIAN").mkdir(parents=True)
            control = payload / "DEBIAN/control"
            binary = payload / "usr/bin/recite-writer"
            binary.parent.mkdir(parents=True)
            source = base / "writer.c"
            source.write_text(
                '#include <stdio.h>\n#include <string.h>\n'
                'int main(int argc, char **argv) { if (argc > 1 && !strcmp(argv[1], "--help")) '
                '{ puts("--project"); return 0; } if (argc > 1 && !strcmp(argv[1], "--version")) '
                '{ puts("0.0.0"); return 0; } return 1; }\n'
            )
            subprocess.run(["cc", str(source), "-o", str(binary)], check=True, capture_output=True)
            abi = package_writer.linux_abi(binary)
            (base / "runtime-abi.json").write_text(json.dumps(abi))
            control.write_text(
                "Package: recite-writer\nVersion: 0.0.0\nArchitecture: amd64\n"
                "Maintainer: Recite contributors <noreply@example.invalid>\n"
                f"Depends: libc6 (>= {abi['minimumGlibc']}), libxkbcommon-x11-0\nDescription: test fixture\n"
            )
            desktop = payload / "usr/share/applications/recite-writer.desktop"
            desktop.parent.mkdir(parents=True)
            desktop.write_text("[Desktop Entry]\nExec=recite-writer %u\nMimeType=x-scheme-handler/recite;\n")
            licenses = payload / "usr/lib/recite-writer/licenses"
            licenses.mkdir(parents=True)
            for name in ("LICENSE", "LICENSE-MIT", "LICENSE-APACHE"):
                (licenses / name).write_text("fixture\n")
            artifact = base / "recite-writer.deb"
            self.build_deb_fixture(payload, artifact)
            package_check.check_linux(base)
            complete_control = control.read_text()
            control.write_text(complete_control.replace(", libxkbcommon-x11-0", ""))
            self.build_deb_fixture(payload, artifact)
            with self.assertRaisesRegex(ValueError, "libxkbcommon-x11-0"):
                package_check.check_linux(base)
            control.write_text(complete_control)
            desktop.write_text("[Desktop Entry]\nExec=recite-writer\n")
            self.build_deb_fixture(payload, artifact)
            with self.assertRaisesRegex(ValueError, "one URL"):
                package_check.check_linux(base)
            desktop.write_text("[Desktop Entry]\nExec=recite-writer %u\nMimeType=x-scheme-handler/recite;\n")
            control.write_text(
                "Package: recite-writer\nVersion: 0.0.0\nArchitecture: amd64\n"
                "Maintainer: Recite contributors <noreply@example.invalid>\n"
                "Depends: libc6 (>= 2.1), libxkbcommon-x11-0\nDescription: test fixture\n"
            )
            self.build_deb_fixture(payload, artifact)
            with self.assertRaisesRegex(ValueError, "below the packaged ELF requirement"):
                package_check.check_linux(base)
            control.write_text(
                "Package: recite-writer\nVersion: 0.0.0\nArchitecture: amd64\n"
                "Maintainer: Recite contributors <noreply@example.invalid>\n"
                "Description: test fixture\n"
            )
            self.build_deb_fixture(payload, artifact)
            with self.assertRaisesRegex(ValueError, "runtime dependencies"):
                package_check.check_linux(base)

    def test_foreign_platform_inspection_is_rejected(self):
        foreign = "windows" if sys.platform.startswith("linux") else "linux"
        result = subprocess.run(
            [sys.executable, str(ROOT / "scripts/check-writer-package.py"), "--platform", foreign, "."],
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("matching host OS", result.stderr)

    @staticmethod
    def build_deb_fixture(payload, artifact):
        if shutil.which("dpkg-deb"):
            subprocess.run(["dpkg-deb", "--build", str(payload), str(artifact)], check=True, capture_output=True)
            return
        control = payload.parent / "control.tar.gz"
        data = payload.parent / "data.tar.gz"
        with tarfile.open(control, "w:gz") as archive:
            archive.add(payload / "DEBIAN/control", arcname="control")
        with tarfile.open(data, "w:gz") as archive:
            archive.add(payload / "usr", arcname="usr")
        (payload.parent / "debian-binary").write_text("2.0\n")
        artifact.unlink(missing_ok=True)
        subprocess.run(
            ["ar", "rcs", str(artifact), "debian-binary", control.name, data.name],
            cwd=payload.parent,
            check=True,
            capture_output=True,
        )


if __name__ == "__main__":
    unittest.main()
