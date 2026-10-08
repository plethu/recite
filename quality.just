set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments
set default-list

# Provision language quality tools and the exact PowerShell analyzer module.
setup:
    mise -E quality install
    mise -E quality run quality-powershell-setup

# Deliberately format non-Rust sources with the shared configuration.
fmt:
    mise -E quality exec -- tombi format --offline
    mise -E quality exec -- dprint fmt
    for directory in Packages/com.recite.dialogue tests/unity-project; do dotnet format whitespace "$directory" --folder --verbosity quiet; done
    for file in $(git ls-files '*justfile' '*.just'); do just --justfile "$file" --unstable --fmt; done
    mise -E quality exec -- editorconfig-checker -fix

# Check formatting without rewriting source files.
fmt-check:
    mise -E quality exec -- tombi format --offline --check
    mise -E quality exec -- dprint check
    for directory in Packages/com.recite.dialogue tests/unity-project; do dotnet format whitespace "$directory" --folder --verify-no-changes --verbosity quiet; done
    for file in $(git ls-files '*justfile' '*.just'); do just --justfile "$file" --unstable --fmt --check; done
    mise -E quality exec -- editorconfig-checker

# Run language linters; engine/parser contract checks remain in their own lanes.
lint:
    just web check
    mise -E quality exec -- tombi lint --offline --error-on-warnings
    mise -E quality exec -- ruff check
    mise -E quality exec -- rumdl check .
    mise -E quality exec -- oxlint --deny-warnings .
    mise -E quality exec -- scripts/quality/check-shell.sh
    mise -E quality exec -- gdlint $(git ls-files '*.gd')
    mise -E quality exec -- lua-language-server --check=editors/recite-neovim --configpath="$PWD/.luarc.json" --checklevel=Warning --logpath=target/lua-language-server
    mise -E quality exec -- pwsh -NoProfile -File scripts/quality/powershell.ps1 lint

# Install browsers matching the committed Playwright dependency.
setup-browsers:
    pnpm -C docs-site exec playwright install
