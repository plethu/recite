set working-directory := '../..'
set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments
set default-list

# Provision language quality tools and the exact PowerShell analyzer module.
setup:
    mise install python
    "$(mise which python)" scripts/setup-quality.py

# Deliberately format non-Rust sources with the shared configuration.
fmt:
    mise -E quality exec -- tombi format --offline
    bun run js:fmt
    mise -E quality exec -- dprint fmt
    for directory in Packages/com.recite.dialogue tests/unity-project; do dotnet format whitespace "$directory" --folder --verbosity quiet; done
    for file in $(git ls-files '*justfile' '*.just'); do just --justfile "$file" --unstable --fmt; done
    mise -E quality exec -- editorconfig-checker -fix

# Check formatting without rewriting source files.
fmt-check:
    mise -E quality exec -- python scripts/run-checks.py fmt-other

[private]
_fmt-check:
    tombi format --offline --check
    bun run --no-env-file js:fmt-check
    dprint check
    for directory in Packages/com.recite.dialogue tests/unity-project; do dotnet format whitespace "$directory" --folder --verify-no-changes --verbosity quiet; done
    for file in $(git ls-files '*justfile' '*.just'); do just --justfile "$file" --unstable --fmt --check; done
    editorconfig-checker

# Run language linters; engine/parser contract checks remain in their own lanes.
lint:
    mise -E quality exec -- python scripts/run-checks.py lint-other

[private]
_lint:
    bun run --no-env-file --cwd docs-site lint:css
    tombi lint --offline --error-on-warnings
    ruff check
    rumdl check .
    bun run --no-env-file js:check
    scripts/quality/check-shell.sh
    gdlint $(git ls-files '*.gd')
    lua-language-server --check=editors/recite-neovim --configpath="$PWD/.luarc.json" --checklevel=Warning --logpath=target/lua-language-server
    pwsh -NoProfile -File scripts/quality/powershell.ps1 lint

# Install browsers matching the committed Playwright dependency.
setup-browsers:
    bun run --cwd docs-site playwright install
