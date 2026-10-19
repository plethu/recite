"""Owned native accessibility host processes and PID-scoped X11 focus."""

import subprocess
from contextlib import contextmanager


@contextmanager
def child_process(arguments, **kwargs):
    process = subprocess.Popen(arguments, **kwargs)
    try:
        yield process
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def focus_x11_window(pid):
    # Bare Xvfb has no window manager. Focus only the specimen's visible window.
    subprocess.run(
        [
            "xdotool",
            "search",
            "--sync",
            "--onlyvisible",
            "--limit",
            "1",
            "--pid",
            str(pid),
            ".",
            "windowfocus",
            "--sync",
        ],
        check=True,
        capture_output=True,
        timeout=20,
    )
