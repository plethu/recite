fn main() {
    let grammar = "../../../../editors/recite-tree-sitter/src";
    cc::Build::new()
        .include(grammar)
        .file(format!("{grammar}/parser.c"))
        .compile("recite_writer_grammar");
    println!("cargo:rerun-if-changed={grammar}/parser.c");
    println!("cargo:rerun-if-changed={grammar}/tree_sitter/parser.h");
}
