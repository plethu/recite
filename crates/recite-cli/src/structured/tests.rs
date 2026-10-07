use super::errors::{ErrorCategory, ErrorCode, ErrorOperation};
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
