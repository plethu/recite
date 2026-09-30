set shell := ["bash", "-euo", "pipefail", "-c"]
set default-list := true

# Run the copyable CLI project and Bevy companion together.
workflow:
    cargo build --locked -p recite-cli
    python3 scripts/check-workflow.py --recite target/debug/recite
    cargo test --locked -p recite-bevy --test workflow_project

# Run Bevy App tests and a clean consumer of local packages.
bevy:
    cargo nextest run --locked -p recite-bevy
    scripts/check-bevy-package.sh

# Exercise the Godot addon in its pinned native host.
godot:
    mise -E godot install
    mise -E godot exec -- scripts/check-godot-host.sh

# Check the Unity package and managed/native bridge.
unity:
    scripts/check-unity-adapter.sh
