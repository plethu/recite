use super::*;

#[test]
fn unchanged_exports_still_replace_echo_and_reference_dependencies() {
    let a = ":: a default\n> @ffee6p8\n  A.\n-> END\n";
    let b = ":: b\n? echo=line(ffee6p8) @rthh7ab\n  Reply.\n  -> a.recite::a\n";
    let c = ":: c\n> @bqyvpqq\n  C.\n-> END\n";
    let retargeted = b
        .replace("ffee6p8", "bqyvpqq")
        .replace("a.recite::a", "c.recite::c");
    let changed_provider = c
        .replace("bqyvpqq", "z0g6c80")
        .replace(":: c", ":: renamed");
    let mut kernel = AuthoringKernel::new();
    for (consumer, provider, has_error) in [
        (b, c, false),
        (retargeted.as_str(), c, false),
        (retargeted.as_str(), changed_provider.as_str(), true),
        (retargeted.as_str(), c, false),
        (b, changed_provider.as_str(), false),
    ] {
        compare(
            &mut kernel,
            &[
                ("a.recite", a),
                ("b.recite", consumer),
                ("c.recite", provider),
            ],
            None,
            true,
        );
        let diagnostics = kernel
            .snapshot()
            .document(&DocumentKey::new("b.recite").unwrap())
            .unwrap()
            .diagnostics();
        assert_eq!(!diagnostics.is_empty(), has_error);
    }
}

#[test]
fn moving_default_between_existing_blocks_matches_cold_diagnostics() {
    let source = ":: first default\n-> missing\n:: second\n-> absent\n";
    let moved = source
        .replace(":: first default", ":: first")
        .replace(":: second", ":: second default");
    let other = ":: other default\n-> a.recite::first\n";
    let mut kernel = AuthoringKernel::new();
    for next in [source, moved.as_str(), source] {
        let documents = [("a.recite", next), ("other.recite", other)];
        compare(&mut kernel, &documents, None, true);
        let mut cold = AuthoringKernel::new();
        compare(&mut cold, &documents, None, true);
        for (warm, fresh) in kernel
            .snapshot()
            .documents()
            .iter()
            .zip(cold.snapshot().documents())
        {
            assert_eq!(warm.key(), fresh.key());
            assert_eq!(warm.diagnostics(), fresh.diagnostics());
        }
    }
}
