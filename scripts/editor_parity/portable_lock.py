import os
import time
from pathlib import Path

if os.name == "nt":
    import errno
    import msvcrt
else:
    import fcntl


class PortableLock:
    """A fixed one-byte lock that serializes parity Cargo discovery."""

    def __init__(self, path: Path):
        self.path = path
        self.handle = None

    def __enter__(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.handle = self.path.open("a+b")
        self.handle.seek(0, os.SEEK_END)
        if self.handle.tell() == 0:
            self.handle.write(b"0")
            self.handle.flush()
        self.handle.seek(0)
        self._acquire()
        return self

    def _acquire(self):
        if os.name == "nt":
            while True:
                try:
                    msvcrt.locking(self.handle.fileno(), msvcrt.LK_NBLCK, 1)
                    return
                except OSError as error:
                    if error.errno not in {errno.EACCES, errno.EAGAIN}:
                        raise
                    time.sleep(0.1)
        else:
            fcntl.flock(self.handle.fileno(), fcntl.LOCK_EX)

    def __exit__(self, *_):
        if self.handle is None:
            return
        self.handle.seek(0)
        if os.name == "nt":
            msvcrt.locking(self.handle.fileno(), msvcrt.LK_UNLCK, 1)
        else:
            fcntl.flock(self.handle.fileno(), fcntl.LOCK_UN)
        self.handle.close()
