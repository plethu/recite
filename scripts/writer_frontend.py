#!/usr/bin/env python3
"""Resolve a Writer frontend to its Cargo package and executable name."""

import argparse
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def package_for(frontend=None):
    with (ROOT / "apps/writer/Cargo.toml").open("rb") as source:
        writer = tomllib.load(source)["workspace"]["metadata"]["writer"]
    selected = frontend or writer["default-frontend"]
    try:
        return writer["frontends"][selected]
    except KeyError as error:
        raise ValueError(f"unknown Writer frontend: {selected}") from error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--frontend", choices=("xilem", "freya"))
    print(package_for(parser.parse_args().frontend))


if __name__ == "__main__":
    main()
