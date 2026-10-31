use super::compare;
use recite_compiler::authoring::AuthoringKernel;

#[test]
fn reference_edits_preserve_structural_errors_and_provider_dependencies() {
    let source = ":: a default\n> @ffee6p8\n  First.\n? echo=line(pm96kqq) @ffee6p8\n  Reply.\n  -> missing\n:: a default\n-> b.recite::b\n";
    let b = ":: b default\n> @ffee6p8\n  Other.\n-> END\n";
    let c = ":: c\n-> END\n";
    for complete in [true, false] {
        let mut kernel = AuthoringKernel::new();
        for target in [
            "b.recite::missing",
            "b.recite::b",
            "c.recite::c",
            "absent.recite::c",
            "a",
            "END",
        ] {
            let next = source.replace("b.recite::b", target);
            for provider in [c.to_owned(), c.replace(":: c", ":: renamed"), c.to_owned()] {
                compare(
                    &mut kernel,
                    &[
                        ("a.recite", &next),
                        ("b.recite", b),
                        ("c.recite", &provider),
                    ],
                    None,
                    complete,
                );
            }
            // Relocation must move both report families and related spans.
            compare(
                &mut kernel,
                &[
                    ("a.recite", &format!("\n{next}")),
                    ("b.recite", b),
                    ("c.recite", c),
                ],
                None,
                complete,
            );
            compare(
                &mut kernel,
                &[("a.recite", &next), ("b.recite", b), ("c.recite", c)],
                None,
                complete,
            );
        }
    }
}

#[test]
fn reference_recovery_completeness_and_simultaneous_changes_match_batch() {
    let source = ":: a default\n-> b.recite::b\n";
    let provider = ":: b\n-> END\n";
    let mut kernel = AuthoringKernel::new();
    for complete in [true, false, true] {
        for b in [
            provider,
            ":: b\n:if broken(\n",
            ":: b\n> @Bad\n  Broken.\n",
            "",
        ] {
            for a in [
                source.to_owned(),
                source.replace("::b", "::missing"),
                source.replace("-> b.recite::b", "->"),
                source.to_owned(),
            ] {
                compare(
                    &mut kernel,
                    &[("a.recite", &a), ("b.recite", b)],
                    None,
                    complete,
                );
            }
        }
    }
    for (a, b) in [
        (source, ":: b\n-> missing\n"),
        (":: a default\n-> missing\n", ":: b\n-> a.recite::a\n"),
        (source, provider),
    ] {
        compare(&mut kernel, &[("a.recite", a), ("b.recite", b)], None, true);
    }
}
