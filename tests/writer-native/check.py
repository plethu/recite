#!/usr/bin/python3
"""Exercise the Xilem Writer on owned Wayland, D-Bus, project and editor fixtures."""

import argparse
import hashlib
import json
import os
import signal
import subprocess
import sys
import tempfile
from pathlib import Path

import atspi
from atspi import ACCESSIBLE, Gio, GLib, call, child_process, wait_for


def atspi_binary(name):
    for directory in ("/usr/lib", "/usr/libexec"):
        path = Path(directory) / name
        if path.is_file():
            return str(path)
    raise RuntimeError(f"Install at-spi2-core: could not find {name}")


def run_probe(binary, output, scenario):
    session = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    status = ("org.a11y.Bus", "/org/a11y/bus")
    call(
        session,
        status,
        "org.freedesktop.DBus.Properties",
        "Set",
        GLib.Variant("(ssv)", ("org.a11y.Status", "IsEnabled", GLib.Variant("b", True))),
    )
    call(
        session,
        status,
        "org.freedesktop.DBus.Properties",
        "Set",
        GLib.Variant("(ssv)", ("org.a11y.Status", "ScreenReaderEnabled", GLib.Variant("b", True))),
    )
    address = call(session, status, "org.a11y.Bus", "GetAddress")[0]
    bus = Gio.DBusConnection.new_for_address_sync(
        address,
        Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT
        | Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION,
        None,
        None,
    )
    registry = ("org.a11y.atspi.Registry", atspi.ROOT)
    registry_binary = atspi_binary("at-spi2-registryd")
    with (
        child_process(
            [registry_binary],
            env=dict(os.environ, AT_SPI_BUS_ADDRESS=address),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ) as registry_process,
        child_process(["wtype", "-s", "60000"]),
    ):
        wait_for(
            lambda: (
                call(
                    bus, registry, ACCESSIBLE, "GetChildren", flags=Gio.DBusCallFlags.NO_AUTO_START
                )
                or True
            ),
            "private registry",
            registry_process,
        )
        if scenario == "reading-continuity":
            from reading_continuity import run
        elif scenario == "writing-journey":
            from writing_journey import run
        elif scenario == "workbench":
            from workbench_interactions import run
        elif scenario == "specimen":
            from specimen_interactions import run
        elif scenario == "localisation-final":
            from localisation_final import run
        elif scenario == "clipboard":
            from clipboard_interactions import run
        elif scenario == "linked-destinations":
            from linked_destinations import run
        elif scenario == "disk-notices":
            from disk_notices import run
        elif scenario == "preview-trace-scale":
            from preview_trace import run_scale as run
        elif scenario == "preview-trace":
            from preview_trace import run
        elif scenario == "localisation-navigation-scale":
            from localisation_navigation import run_scale as run
        elif scenario == "localisation-navigation":
            from localisation_navigation import run
        elif scenario == "localisation-details-scale":
            from localisation_details import run_scale as run
        elif scenario == "localisation-details":
            from localisation_details import run
        elif scenario == "preview-inputs-scale":
            from preview_inputs import run_scale as run
        elif scenario == "preview-inputs":
            from preview_inputs import run
        elif scenario == "localisation-sources-scale":
            from localisation_sources import run_scale as run
        elif scenario == "localisation-sources":
            from localisation_sources import run
        elif scenario == "localisation-scale":
            from localisation_scale import run
        elif scenario == "localisation-beat":
            from localisation_beat import run
        elif scenario == "localisation":
            from localisation_interactions import run
        elif scenario == "external":
            from external_interactions import run
        elif scenario == "rename":
            from rename_interactions import run
        elif scenario == "project-settings":
            from project_settings_interactions import run
        elif scenario == "declaration-source":
            from declaration_source_interactions import run
        elif scenario == "generation":
            from generation_interactions import run
        elif scenario == "project-tools":
            from project_tools_interactions import run
        elif scenario == "build":
            from build_interactions import run
        elif scenario == "focus":
            from focus_interactions import run
        elif scenario == "reload":
            from reload_interactions import run
        elif scenario == "map":
            from map_interactions import run
        elif scenario == "routes":
            from routes_interactions import run
        elif scenario == "rules":
            from rules_interactions import run
        elif scenario == "repair":
            from repair_interactions import run
        elif scenario == "picker":
            from picker_interactions import run
        elif scenario == "passage-lookup":
            from passage_lookup import run
        elif scenario == "preview":
            from preview_interactions import run
        elif scenario == "keyboard":
            from keyboard_interactions import run
        elif scenario == "editor":
            from editor_interactions import run
        elif scenario == "preferences":
            from preferences_interactions import run
        elif scenario == "project-replacement":
            from project_replacement import run
        elif scenario == "startup":
            from startup_interactions import run
        elif scenario == "structure":
            from structure_interactions import run
        elif scenario == "frame-timing":
            from frame_timing import run
        else:
            from writer_interactions import run

        return run(atspi, bus, registry, binary, output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--child", action="store_true")
    parser.add_argument(
        "--renderer",
        choices=("pixman", "gles2", "vulkan"),
        default="pixman",
        help="Renderer for the owned headless compositor; GPU modes require a usable render device.",
    )
    parser.add_argument(
        "--scenario",
        choices=(
            "all",
            "specimen",
            "workbench",
            "writing-journey",
            "reading-continuity",
            "localisation-final",
            "localisation-beat",
            "clipboard",
            "linked-destinations",
            "disk-notices",
            "structure",
            "startup",
            "project-replacement",
            "preferences",
            "editor",
            "keyboard",
            "preview",
            "preview-inputs",
            "preview-inputs-scale",
            "preview-trace",
            "preview-trace-scale",
            "picker",
            "passage-lookup",
            "repair",
            "rules",
            "routes",
            "map",
            "reload",
            "focus",
            "project-tools",
            "build",
            "generation",
            "declaration-source",
            "project-settings",
            "rename",
            "external",
            "localisation",
            "localisation-navigation",
            "localisation-navigation-scale",
            "localisation-details",
            "localisation-details-scale",
            "localisation-scale",
            "localisation-sources",
            "localisation-sources-scale",
            "frame-timing",
        ),
        default="all",
    )
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    output = args.output.resolve()
    if output.exists():
        parser.error("Refusing to overwrite an existing result")
    output.parent.mkdir(parents=True, exist_ok=True)
    if args.child:
        os.environ.pop("DISPLAY", None)
        if os.environ.get("RECITE_PRIVATE_PROBE") != "1":
            parser.error("Child mode requires the owned compositor and bus")
        with child_process(
            [atspi_binary("at-spi-bus-launcher"), "--launch-immediately"]
        ) as launcher:
            session = Gio.bus_get_sync(Gio.BusType.SESSION, None)
            wait_for(
                lambda: call(
                    session,
                    ("org.freedesktop.DBus", "/org/freedesktop/DBus"),
                    "org.freedesktop.DBus",
                    "NameHasOwner",
                    GLib.Variant("(s)", ("org.a11y.Bus",)),
                )[0],
                "owned accessibility bus launcher",
                launcher,
            )
            with binary.open("rb") as executable:
                digest = hashlib.file_digest(executable, "sha256").hexdigest()
            passed = run_probe(binary, output, args.scenario)
            with binary.open("rb") as executable:
                unchanged = hashlib.file_digest(executable, "sha256").hexdigest() == digest
            receipt = json.loads(output.read_text())
            receipt["binary"] = {"path": str(binary), "sha256": digest, "unchanged": unchanged}
            receipt["compositor_renderer"] = args.renderer
            from logging_policy import metrics

            receipt["logging"] = metrics(output)
            if receipt["logging"]["automatic_dense_trace_files"]:
                receipt["passed"] = passed = False
                receipt["logging_error"] = "Writer created an implicit unfiltered trace file."
            if not unchanged:
                receipt["passed"] = passed = False
                receipt["binary_error"] = (
                    "Executable changed during the probe; rerun after building."
                )
            output.write_text(json.dumps(receipt, indent=2) + "\n")
        if not passed:
            sys.exit(2)
        return
    # Wayland's Unix socket pathname must fit within 108 bytes, including NUL.
    with tempfile.TemporaryDirectory(prefix="recite-ui-", dir="/tmp") as temporary:
        runtime = Path(temporary) / "runtime"
        runtime.mkdir(mode=0o700)
        config = Path(temporary) / "session.conf"
        config.write_text("""<busconfig><type>session</type><listen>unix:tmpdir=/tmp</listen>
<policy context="default"><allow send_destination="*"/><allow receive_sender="*"/>
<allow own="*"/></policy></busconfig>""")
        environment = dict(os.environ)
        for key in (
            "DISPLAY",
            "RECITE_CONFIG",
            "WAYLAND_DISPLAY",
            "WINIT_UNIX_BACKEND",
            "DBUS_SESSION_BUS_ADDRESS",
            "AT_SPI_BUS_ADDRESS",
            "DBUS_STARTER_ADDRESS",
            "DBUS_STARTER_BUS_TYPE",
        ):
            environment.pop(key, None)
        environment.update(
            XDG_RUNTIME_DIR=str(runtime),
            XDG_CONFIG_HOME=str(Path(temporary) / "config"),
            XDG_STATE_HOME=str(Path(temporary) / "state"),
            GSETTINGS_BACKEND="memory",
            WLR_BACKENDS="headless",
            WLR_HEADLESS_OUTPUTS="1",
            WLR_LIBINPUT_NO_DEVICES="1",
            WLR_RENDERER=args.renderer,
            RECITE_PRIVATE_PROBE="1",
        )
        process = subprocess.Popen(
            [
                "cage",
                "--",
                "dbus-run-session",
                "--config-file=" + str(config),
                "--",
                sys.executable,
                __file__,
                str(binary),
                str(output),
                "--child",
                "--scenario",
                args.scenario,
                "--renderer",
                args.renderer,
            ],
            env=environment,
            start_new_session=True,
        )
        try:
            code = process.wait(
                timeout=120
                if args.scenario
                in (
                    "all",
                    "preferences",
                    "editor",
                    "localisation-beat",
                    "workbench",
                    "writing-journey",
                    "reading-continuity",
                )
                or args.scenario.startswith(
                    (
                        "preview-inputs",
                        "preview-trace",
                        "localisation-details",
                        "localisation-navigation",
                    )
                )
                else 60
            )
            if code:
                sys.exit(code)
        finally:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)


if __name__ == "__main__":
    main()
