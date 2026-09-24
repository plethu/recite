mod support;
use freya::{elements::paragraph::ParagraphCursorExt, prelude::*};
use freya_testing::prelude::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn assert_caret_visible(test: &TestingRunner) -> TestResult {
    let viewport = test
        .find(|node, element| {
            Rect::try_downcast(element)
                .filter(|rect| {
                    rect.accessibility.builder.role() == AccessibilityRole::TextInput
                        && node.layout().area.height() > 100.
                })
                .map(|_| node.layout().area)
        })
        .ok_or("scroll viewport")?;
    let caret = test
        .find(|node, element| {
            let paragraph = Paragraph::try_downcast(element)?;
            let index = paragraph.cursor_index?;
            let text: String = paragraph
                .spans
                .iter()
                .map(|span| span.text.as_ref())
                .collect();
            let holder = paragraph.sk_paragraph.0.borrow();
            let measured = holder
                .as_ref()?
                .paragraph
                .cursor_rect(&text, index, TextAlign::Left);
            let area = node.layout().inner_area;
            Some((
                area.min_x() + measured.left,
                area.min_x() + measured.right.max(measured.left + 6.),
                area.min_y(),
                area.max_y(),
            ))
        })
        .ok_or("mounted caret paragraph")?;
    assert!(
        caret.0 >= viewport.min_x() && caret.1 <= viewport.max_x(),
        "horizontal {caret:?} in {viewport:?}"
    );
    assert!(
        caret.2 >= viewport.min_y() && caret.3 <= viewport.max_y(),
        "vertical {caret:?} in {viewport:?}"
    );
    Ok(())
}

#[test]
fn clicking_an_offscreen_diagnostic_reveals_its_source() -> TestResult {
    let source = format!(
        ":: start default\n> line@11111111111111111111\n  Hello.\n{}-> {}missing\n",
        "\n".repeat(160),
        " ".repeat(200)
    );
    let model = recite_writer_model::Workbench::new(&source)?;
    let diagnostics = model.document().diagnostics();
    let diagnostic = diagnostics
        .iter()
        .find(|d| d.span.start.line() > 100)
        .ok_or("distant diagnostic")?;
    let span = &diagnostic.span;
    let caption = format!(
        "Open diagnostic {} at {}:{}:{}",
        diagnostic.code,
        span.file,
        span.start.line(),
        span.start.column()
    );
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        (1200., 900.).into(),
        |runner| {
            runner.provide_root_context(|| recite_writer::InitialSource(source));
        },
        1.,
    )
    .0;
    support::click(&mut test, "Source")?;
    support::click(&mut test, &caption)?;
    assert_caret_visible(&test)
}
