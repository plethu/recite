# Recite production specification

The specification defines product and compatibility requirements. It is split by subsystem so
maintainers can read the affected contract without loading the whole project. Read the chapter and
subsection needed for the change. Code and tests establish implemented behaviour; GitHub owns
current task and milestone state. Historical experiment reports are evidence, not additional
specification.

| Contract                                | Sections | Chapter                                              |
| --------------------------------------- | -------- | ---------------------------------------------------- |
| Product and invariants                  | §1–4     | [product](spec/product.md)                           |
| Source format                           | §5       | [source](spec/source.md)                             |
| Conditions and effects                  | §6–7     | [conditions-effects](spec/conditions-effects.md)     |
| Runtime and localisation                | §8–9     | [runtime-localisation](spec/runtime-localisation.md) |
| Schema                                  | §10      | [schema](spec/schema.md)                             |
| Scene manifests, compiler and CLI       | §11–13   | [build-cli](spec/build-cli.md)                       |
| LSP, editors and engine adapters        | §14–16   | [tooling](spec/tooling.md)                           |
| Testing, diagnostics and performance    | §17–19   | [quality](spec/quality.md)                           |
| Migration, scope and release acceptance | §20–24   | [release](spec/release.md)                           |
