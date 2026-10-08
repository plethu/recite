use super::*;

#[test]
fn assembling_shifted_regions_preserves_previously_published_snapshots() {
    let source = ":: a default\r\n> a@11111111111111111111\r\n  Café 🦀\r\n:: b\r\n> b@22222222222222222222\r\n  Middle\r\n:: c\r\n-> missing\r\n";
    let mut kernel = AuthoringKernel::new();
    compare(&mut kernel, &[("a.recite", source)], None, true);
    let published = kernel.snapshot().clone();
    let original = published.documents().to_vec();
    for padding in ["\r\n  More", "\r\n  More\r\n  Again", ""] {
        let edited = source.replace("Café 🦀", &format!("Café 🦀{padding}"));
        compare(&mut kernel, &[("a.recite", &edited)], None, true);
        let mut cold = AuthoringKernel::new();
        compare(&mut cold, &[("a.recite", &edited)], None, true);
        let warm = &kernel.snapshot().documents()[0];
        let fresh = &cold.snapshot().documents()[0];
        assert_eq!(warm.summary(), fresh.summary());
        assert_eq!(warm.diagnostics(), fresh.diagnostics());
        assert_eq!(published.documents(), original);
    }
}

#[test]
fn region_edits_and_global_recovery_match_batch_validation() {
    let a =
        "# preamble\n:: a default\n> a@11111111111111111111\n  Café 🦀\n-> other.recite::target\n";
    let b = ":: b\n> b@22222222222222222222\n  Hello {unknown}.\n? choice@33333333333333333333 echo=a@11111111111111111111\n  Yes\n  -> a\n";
    let c = ":: c\n:if missing()\n  > c@44444444444444444444\n    Nested\n-> END\n";
    let source = format!("{a}{b}{c}");
    let other = ":: target\n-> a.recite::b\n";
    let schema = ProjectSchema::empty_v1();
    let mut kernel = AuthoringKernel::with_schema(schema.clone());
    for newline in ["\n", "\r\n"] {
        for edited in [
            source.clone(),
            source.replace("Café 🦀", "A new sentence 🦀"),
            source.replace("Café 🦀", "A new\n  paragraph"),
            source.replace("22222222222222222222", "11111111111111111111"),
            source.replace(":: b", ":: renamed"),
            source.replace(":: b", "  :: b"),
            source.replace(":: b", "::"),
            source.replace(":: b", "broken\n:: b"),
            source.replace(":: a default", ":: a default bind=(name:string=$)"),
            source.replace("missing()", "missing("),
            format!("{a}{c}"),
            format!("{c}{a}{b}"),
            format!("{a}{b}{b}{c}"),
            format!("\n{source}"),
            source.clone(),
        ] {
            let edited = edited.replace('\n', newline);
            compare(
                &mut kernel,
                &[("a.recite", &edited), ("other.recite", other)],
                Some(&schema),
                true,
            );
            let mut cold = AuthoringKernel::with_schema(schema.clone());
            compare(
                &mut cold,
                &[("a.recite", &edited), ("other.recite", other)],
                Some(&schema),
                true,
            );
            for (warm, cold) in kernel
                .snapshot()
                .documents()
                .iter()
                .zip(cold.snapshot().documents())
            {
                assert_eq!(warm.summary(), cold.summary());
                assert_eq!(warm.participation(), cold.participation());
                assert_eq!(warm.diagnostics(), cold.diagnostics());
            }
        }
    }
}

#[test]
fn relocating_multiple_files_updates_primary_and_related_project_diagnostics() {
    let a = ":: shared default\n> duplicate@11111111111111111111\n  First 🦀.\n? reply@33333333333333333333 echo=line(22222222222222222222)\n  Reply\n  -> missing\n";
    let b = ":: shared default\n> duplicate@11111111111111111111\n  Second.\n-> a.recite::absent\n";
    let mut kernel = AuthoringKernel::new();
    compare(&mut kernel, &[("a.recite", a), ("b.recite", b)], None, true);
    assert!(kernel.snapshot().documents().iter().any(|document| {
        document
            .diagnostics()
            .iter()
            .any(|diagnostic| !diagnostic.related_presentations.is_empty())
    }));
    for padding in ["\n", "# preamble\n\n", ""] {
        let shifted_a = format!("{padding}{a}").replace("First 🦀.", "First\n  🦀 paragraph.");
        let shifted_b = format!("{padding}{b}");
        compare(
            &mut kernel,
            &[("a.recite", &shifted_a), ("b.recite", &shifted_b)],
            None,
            true,
        );
    }
}

#[test]
fn unrelated_recovery_invalidates_unknown_echo_diagnostics_in_both_directions() {
    let a = ":: a default\n> a@11111111111111111111\n  Hello\n";
    let b = ":: b\n? b@22222222222222222222 echo=line(33333333333333333333)\n  Reply\n  -> END\n";
    let c = ":: c\n-> END\n";
    let mut kernel = AuthoringKernel::new();
    for edited in [a.to_owned(), format!("{a}stray text\n"), a.to_owned()] {
        compare(
            &mut kernel,
            &[("a.recite", &edited), ("b.recite", b), ("c.recite", c)],
            None,
            true,
        );
        let diagnostics = kernel
            .snapshot()
            .document(&DocumentKey::new("b.recite").unwrap())
            .unwrap()
            .diagnostics();
        assert_eq!(diagnostics.is_empty(), edited.contains("stray text"));
    }
}
