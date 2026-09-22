use recite_core::Statement;
use recite_parser::parse;

#[test]
fn singleton_bodies_reserve_one_large_statement_slot_at_boundaries_and_eof() {
    let parsed = parse(
        "allocation.recite",
        ":: first default\n:if ready()\n  # then\n\n:else\n  # else\n\n:: second\n# last\n\n",
    );
    let lowered = parsed.lower_source_file();
    assert!(lowered.diagnostics.is_empty());
    let blocks = &lowered.source_file.blocks;
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].statements.capacity(), 1);
    let Statement::If(branch) = &blocks[0].statements[0] else {
        panic!("expected branch");
    };
    assert_eq!(branch.then_statements.len(), 1);
    assert_eq!(branch.then_statements.capacity(), 1);
    assert_eq!(branch.else_statements.len(), 1);
    assert_eq!(branch.else_statements.capacity(), 1);
    assert_eq!(blocks[1].statements.len(), 1);
    assert_eq!(blocks[1].statements.capacity(), 1);
}

#[test]
fn wider_bodies_keep_geometric_growth() {
    let lowered = parse(
        "allocation.recite",
        ":: wide default\n# one\n# two\n# three\n# four\n# five\n",
    )
    .lower_source_file();
    assert!(lowered.diagnostics.is_empty());
    let statements = &lowered.source_file.blocks[0].statements;
    assert_eq!(statements.len(), 5);
    // Compare with the standard growth policy rather than an allocator-specific number.
    let mut normal = Vec::new();
    for statement in statements {
        normal.push(statement.clone());
    }
    assert_eq!(statements.capacity(), normal.capacity());
}
