//! Standard binding to the repository's generated Recite highlighting grammar.
use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_recite() -> *const ();
}

// SAFETY: build.rs compiles Recite's generated tree-sitter parser, whose
// exported function has this ABI and returns its immutable static language.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_recite) };
pub const HIGHLIGHTS: &str =
    include_str!("../../../../../editors/recite-tree-sitter/queries/highlights.scm");
