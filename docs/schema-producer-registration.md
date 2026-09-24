# Connecting a schema producer to the writer

A project may supply `recite.producer.toml` beside `recite.project.toml`.
The writer reads this registration when Declarations opens. Loading a project or
registration never executes commands. The author explicitly chooses Regenerate
or Open declaration source; generated schema remains read-only.

```toml
version = 1

[producer]
kind = "bevy"
id = "my-game-dialogue"

[generate]
program = "cargo"
args = ["run", "--bin", "export-dialogue-schema", "--", "--output", "{output}"]
directory = "."

# Optional: an editor that accepts file and line arguments.
[editor]
program = "code"
args = ["--goto", "{file}:{line}:{column}"]
directory = "."

[[sources]]
kind = "condition"
name = "has_item"
file = "src/dialogue/schema.rs"
line = 42
column = 1
```

The producer identity must exactly match the generated schema configured by
`project.schema`. This registration is a tooling contract, not a second schema.
An adapter can generate source locations alongside its schema export. Source
keys are the declaration kind and name, independent of the UI language.

Supported source kinds are `condition`, `effect`, `speaker`, `registry`, `type`,
`availability_reason`, `metadata_domain`, `metadata`, `projection_query`,
`presentation_projector`, and `markup`. Locations use one-based lines and columns.
Duplicate keys, unknown kinds, unsupported versions and unknown fields are errors.
Files and working directories are project-relative and cannot traverse above the
root. The host also checks their resolved paths before use.

Commands contain an executable and separate arguments. Recite does not split a
shell command string or expand environment variables. A bare executable is found
on PATH; a relative executable containing a path separator is resolved against
the configured working directory. Absolute executables are allowed for local
configuration, though they reduce portability.

Generation must accept `{output}` in an argument. Recite substitutes an absolute
path in a temporary directory, waits for the command, validates the resulting
schema and producer identity, then publishes it using the existing checked atomic
file writer. The command must write **only to that supplied output**, wait for its
own workers, and exit after exporting. It must not write the project's live schema
or detach a background exporter. Commands are ordinary local processes, not a
sandbox: use a producer you trust with the project.

A failed command, invalid output or concurrent change to the live schema prevents
publication. Failure feedback includes up to 64 KiB of command output. Cancel or
the two-minute deadline stops the direct producer process and prevents publication;
a producer is responsible for its own subprocess cleanup. The last valid schema
remains available when the command follows the output contract. Build and preview
consume no intermediate output; restart a preview to use new declarations.

The optional editor command accepts `{file}`, `{line}` and `{column}` within its
arguments. Without it, Recite still shows the source location. This command is
also run only on an explicit click. Configure it for the author's preferred editor.

Use Reload producer registration after changing the registration. Recite refuses
to run a generation command if its registration changed after being displayed.
Use Reload generated declarations to adopt an externally updated valid schema;
this retains standalone source drafts. Unregistered producers remain inspectable
and explain how to connect their source.

Standalone projects can continue to use the TOML source editor and its built-in
exporter without registering any command.
