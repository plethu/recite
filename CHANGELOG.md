# Changelog

Notable changes to Recite are documented here, following
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Recite is pre-release: everything sits
under Unreleased. The
[pre-release compiled-format policy](docs/spec/build-cli.md#122-compiled-format) applies until the
first tagged release.

## [Unreleased]

Initial pre-release development of the dialogue compiler/runtime, authoring tools and engine
companions. [The README](README.md) introduces the current toolchain.

- `recite-core` source AST and `recite-parser` lowering remain pre-1.0 APIs: authoring span and
  recovery fields must be constructed through the published constructors/builders and read through
  their accessors; direct struct literals are not a compatibility promise.
