use super::*;

#[test]
fn full_replacements_and_deleted_keys_preserve_dependency_diagnostics() {
    let a = ":: a default\n> a@11111111111111111111\n  A.\n-> END\n";
    let b = ":: b\n> b@22222222222222222222\n  B.\n-> END\n";
    let c = ":: c\n> c@11111111111111111111\n  C.\n-> END\n";
    let d = ":: d\n> d@22222222222222222222\n  D.\n-> END\n";
    let changed_a = a.replace("11111111111111111111", "55555555555555555555");
    let changed_b = b.replace("22222222222222222222", "44444444444444444444");
    let changed_d = d.replace("22222222222222222222", "44444444444444444444");
    let mut kernel = AuthoringKernel::new();
    for documents in [
        vec![("a.recite", a), ("b.recite", b), ("c.recite", c)],
        // A deleted key keeps affected.len() equal to the current document
        // count, but b still needs discovery as a new stable-ID collider.
        vec![("a.recite", a), ("b.recite", b), ("d.recite", d)],
        vec![
            ("a.recite", changed_a.as_str()),
            ("b.recite", changed_b.as_str()),
            ("d.recite", changed_d.as_str()),
        ],
        vec![("a.recite", a), ("b.recite", b), ("c.recite", c)],
    ] {
        compare(&mut kernel, &documents, None, true);
        let mut fresh = AuthoringKernel::new();
        compare(&mut fresh, &documents, None, true);
        for (warm, cold) in kernel
            .snapshot()
            .documents()
            .iter()
            .zip(fresh.snapshot().documents())
        {
            assert_eq!(warm.key(), cold.key());
            assert_eq!(warm.diagnostics(), cold.diagnostics());
        }
    }
}
