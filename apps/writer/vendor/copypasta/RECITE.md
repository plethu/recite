# Owned display construction

Based on Copypasta 0.10.2, upstream commit `47f5501e3af68ba93e80cda02d94631aec795e63`.
`copypasta-owned-display.patch` in `apps/writer/vendor/patches/` records the API addition:
consume a display owner, retain it across both clipboard handles, and stop the worker before
releasing the owner. The original raw constructor remains available. `Cargo.toml` uses the upstream
manifest and adds an empty workspace boundary for the retained dependency.

MIT OR Apache-2.0 notices are retained. Remove this override when equivalent upstream behavior is
available and verified. Tests cover constructor errors, borrowed-owner rejection and native display
lifetime; native tests require an owned Wayland compositor.
