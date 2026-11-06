use super::StructuredCommand;
use super::errors::{ErrorCategory, ErrorCode, ErrorOperation};
use crate::args::Cli;
use clap::CommandFactory;
use serde::Serialize;
use std::{fs, path::Path};
use strum::IntoEnumIterator;

fn names<T: Serialize>(variants: impl Iterator<Item = T>) -> Vec<String> {
    variants
        .map(|variant| {
            let serde_json::Value::String(name) = serde_json::to_value(variant).expect("wire enum")
            else {
                panic!("wire vocabulary must be strings");
            };
            name
        })
        .collect()
}

#[test]
fn error_vocabulary_projections_are_current() {
    // The editors expose finite dialogue commands and watch, not import or
    // schema export. Dispatch is an unreachable CLI routing fallback.
    let vocabularies = [
        ("errorCategories", names(ErrorCategory::iter())),
        (
            "errorCodes",
            names(
                ErrorCode::iter()
                    .filter(|code| !matches!(code, ErrorCode::Import | ErrorCode::ImportJson)),
            ),
        ),
        (
            "operations",
            names(ErrorOperation::iter().filter(|operation| {
                !matches!(
                    operation,
                    ErrorOperation::ExportSchema | ErrorOperation::Dispatch
                )
            })),
        ),
    ];
    let mut js = "// Generated from CLI error enums; update with `just editor protocol-update`.\n"
        .to_owned();
    let mut lua =
        "-- Generated from CLI error enums; update with `just editor protocol-update`.\nreturn {\n"
            .to_owned();
    for (name, values) in vocabularies {
        js.push_str(&format!("export const {name} = new Set([\n"));
        lua.push_str(&format!("  {name} = {{\n"));
        for value in values {
            let quoted = serde_json::to_string(&value).expect("wire name");
            js.push_str(&format!("  {quoted},\n"));
            lua.push_str(&format!("    [{quoted}] = true,\n"));
        }
        js.push_str("]);\n");
        lua.push_str("  },\n");
    }
    lua.push_str("}\n");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (relative, expected) in [
        ("editors/vscode/src/error-vocabulary.generated.js", js),
        ("editors/recite-neovim/lua/recite_error_vocabulary.lua", lua),
    ] {
        let path = root.join(relative);
        if std::env::var_os("RECITE_UPDATE_PROTOCOL").is_some_and(|value| value == "1") {
            fs::write(&path, &expected).expect("write generated vocabulary");
        }
        assert_eq!(
            fs::read_to_string(&path).expect("read generated vocabulary"),
            expected,
            "{relative} is stale; run `just editor protocol-update`"
        );
    }
}

/// The clap leaf command paths, space-joined (`schema export`).
fn command_paths(command: &clap::Command, prefix: &str, paths: &mut Vec<String>) {
    for sub in command.get_subcommands() {
        let path = if prefix.is_empty() {
            sub.get_name().to_owned()
        } else {
            format!("{prefix} {}", sub.get_name())
        };
        if sub.has_subcommands() {
            command_paths(sub, &path, paths);
        } else {
            paths.push(path);
        }
    }
}

#[test]
fn structured_command_wire_names_are_real_command_paths() {
    // The wire `command` is what a caller typed, so it must resolve in the clap tree. This catches
    // a rename of a command that forgets its structured name, and a typo in a new one.
    let mut paths = Vec::new();
    command_paths(&Cli::command(), "", &mut paths);
    for command in StructuredCommand::iter() {
        assert!(
            paths.iter().any(|path| path == command.wire_name()),
            "{command:?} wire name {:?} is not a CLI command path; commands: {paths:?}",
            command.wire_name()
        );
    }
}

#[test]
fn structured_commands_have_distinct_wire_names_and_operations() {
    let wire_names = StructuredCommand::iter()
        .map(StructuredCommand::wire_name)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(wire_names.len(), StructuredCommand::iter().count());
    let operations = names(StructuredCommand::iter().map(StructuredCommand::operation));
    let distinct = operations.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(distinct.len(), operations.len());
}
