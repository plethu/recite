# Shared UI localisation contract

Recite-owned interface text has one Fluent boundary, implemented by the
`recite-ui` crate. The human-authored `en-US` resource and the typed `MsgId`
registry are the source of truth; the CLI/TUI, LSP, and Neovim are the shipped
clients. Neovim's small host-facing health and lifecycle
surface is a generated, read-only projection of the same English Fluent
resources. VS Code, VSCodium, Zed, and the native GUI are named conformance
clients, not claims that those clients ship in this milestone.

CLI, TUI, LSP, and Neovim adoption is covered by their catalog call sites and
the external contract tests. The Neovim projection is declared in the
canonical inventory, generated from `en-US.ftl`, and checked for stale output
and undeclared call-site IDs. The contract does not yet generate a
whole-repository call-site inventory; the bounded evidence is the typed
registry, explicit client ownership, and focused shipped-client tests.

`UiArgs` is a deterministic `BTreeMap<String, UiArg>`. `UiArg` is format
neutral and supports string, integer, float, and boolean values. The
independent `resources/arguments.toml` file is the checked-in argument
contract. A `UiContract` walks the Fluent AST (not regular expressions) and checks exact
ID and argument equality, duplicate and unknown IDs, malformed resources,
argument type mismatches, client duplication, and projections that are not
declared by their resource. Errors are sorted by their typed issue ordering.

The resolver tries the requested BCP-47 locale, its language-only form, and
`en-US`. A malformed, incomplete, or resolution-invalid non-default resource
is skipped atomically before insertion;
the default resource is fatal if malformed or incomplete. The checked-in
`en-GB` resource is intentionally a test fixture only. Publishing another UI
locale requires human authorship and review.

Host projections are read-only derivatives and must retain the source resource
ID. Host-required protocol metadata remains host-owned. Semantic diagnostics,
trace/JSON output, schema/protocol tokens, and dialogue localisation stay
locale-neutral or on their existing explicit boundary; semantic diagnostic
presentations belong to the producer and are rendered at the client boundary.

First-party semantic producers supply structured diagnostic presentations, and
the CLI and LSP convert and render those records through the shared catalog. The
public `Diagnostic.message`, `Diagnostic.related`, and `Diagnostic.help` fields
remain extension and migration surface, not semantic authority. `record()`
copies only `message` into `DiagnosticRecord` v1's `compatibility_message`
fallback; non-empty legacy `related` or `help` is rejected as `LegacyContext`.
`format_legacy_diagnostic_message` is the explicit catalog adapter for
extension producers. `DiagnosticRecord` v1 is a closed compatibility wire;
removing `compatibility_message` requires a versioned migration. Filesystem
payloads and OS-provided detail text remain host-owned data.

The active #182 inventory localises every migrated first-party primary and
auxiliary presentation, plus the meaning, common-cause, and remediation slots
for every known diagnostic explanation, through human-authored en-US Fluent IDs
in `diagnostics.ftl`. Unmigrated extension producers have exactly one explicit
compatibility adapter, `diagnostic-legacy-message`, for deterministic English
message text; it is not first-party diagnostic coverage. The CLI and LSP are
the structured/record-rendered clients in this boundary.

The CLI error wrapper inventory also gives dedicated Fluent IDs to its
first-party presentation wording: dialogue-catalog plural-header conflicts,
diagnostic rendering and code validation failures, compiled-asset metadata and
shape failures, UI-catalog loading, watcher failures, and the structured PO
malformed-reason details (header, stable ID, duplicate field/entry, and field
order). Their declared arguments are the typed call-site contract in
`resources/arguments.toml`; these IDs must be used instead of composing that
wrapper prose inside `CliError::to_user_message` or returning raw PO details.

PO markup validation retains the core diagnostic presentation ID and typed
arguments when the CLI wraps a malformed catalogue. The wrapper resolves that
presentation through the shared diagnostic Fluent resources, including each
unbalanced-tag shape; only `CliError`'s `Display` compatibility projection may
use the diagnostic's stored English fallback message.

The remaining generic CLI error path is an explicit boundary, not an inventory
gap: `Core`, `Compile`, `CompiledValue`, and `Runtime` carry semantic/domain
payloads whose diagnostic migration is out of scope here; `Io` and filesystem
source values retain host-provided detail text; and user/project content remains
data. Clap's invalid-command renderer remains the host-owned syntax exception.
Benchmark scale/report failures follow the same boundary: the `recite` bench
command wraps them through `cli-error-benchmark`; report JSON and Markdown
remain byte/schema stable, while standalone benchmark binaries retain their
small host-facing argument parser text.

The CLI's `i18n` module is a small compatibility facade for existing call
sites. LSP consumers can inject a `UiCatalog` or explicit `UiLocale`; the
zero-configuration server deterministically uses `en-US`. Locale preference
precedence remains with #167.

Clap owns invalid-command syntax errors. The CLI localises all Recite-owned
command metadata and help templates, then uses Clap's public renderer for its
remaining host-owned syntax labels; that renderer is the only intentional
English residual at this boundary.

See `crates/recite-ui/resources/inventory.toml` and
`docs/recite-production-spec.md` §§9, 13.7, 14–15, and 23 for the normative
boundary. Production locale loading and configuration, non-English publication,
and GUI/editor clients remain separate or out of scope for this contract.
