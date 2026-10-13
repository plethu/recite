use super::*;

#[test]
fn syntax_is_lazy_and_clones_preserve_diagnostics_and_lowering() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Parse>();
    for source in [
        ":: start default\n> hello@11111111111111111111\n  Café 🦀\n-> END\n",
        "# preamble\r\n:: start\r\n> broken@\r\n  prose\r\n \t mixed\r\nstray\r\n",
    ] {
        let parsed = parse_region("scene.recite".into(), source.into(), 17);
        assert!(parsed.green.get().is_none());
        let diagnostics = parsed.diagnostics().to_vec();
        let lowered = parsed.lower_source_file();
        assert!(parsed.green.get().is_none());
        let before = parsed.clone();
        assert!(before.green.get().is_none());
        assert_eq!(parsed.syntax().text().to_string(), source);
        assert!(parsed.green.get().is_some());
        assert_eq!(parsed.syntax().green(), before.syntax().green());
        let after = parsed.clone();
        assert!(after.green.get().is_some());
        for copy in [&parsed, &before, &after] {
            assert_eq!(copy.syntax().text().to_string(), source);
            assert_eq!(copy.diagnostics(), diagnostics);
            assert_eq!(copy.lower_source_file(), lowered);
        }
    }
}

#[test]
fn concurrent_syntax_access_preserves_lossless_source() {
    let source = ":: start default\r\n> line\r\n  Hello.\r\n";
    let parsed = parse("shared.recite", source);
    std::thread::scope(|scope| {
        let first = scope.spawn(|| parsed.syntax().text().to_string());
        let second = scope.spawn(|| parsed.syntax().text().to_string());
        assert_eq!(first.join().unwrap(), source);
        assert_eq!(second.join().unwrap(), source);
    });
    assert!(parsed.green.get().is_some());
}
