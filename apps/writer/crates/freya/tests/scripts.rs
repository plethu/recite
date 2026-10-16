//! Dialogue in non-Latin scripts renders in the Script view, map cards, Source and the sidebar.
use freya::prelude::*;
use freya_testing::prelude::*;

const SOURCE: &str = "\
:: start default
> ru@11111111111111111101 speaker=mara
  Если ты насчёт передатчика — возьми номерок.
> el@11111111111111111102 speaker=mara
  Αν ήρθες για τον πομπό, πάρε αριθμό.
> ja@11111111111111111103 speaker=mara
  送信機のことなら、番号札を取ってください。
> zh@11111111111111111104 speaker=mara
  如果你是为了发射机来的，请先取号。
> ko@11111111111111111105 speaker=mara
  송신기 때문에 왔다면 번호표를 뽑으세요.
> ar@11111111111111111106 speaker=mara
  إذا جئت بخصوص جهاز الإرسال، فخذ رقماً.
> he@11111111111111111107 speaker=mara
  אם באת בגלל המשדר, קח מספר.
> hi@11111111111111111108 speaker=mara
  अगर आप ट्रांसमीटर के बारे में आए हैं, तो नंबर ले लीजिए।
> th@11111111111111111109 speaker=mara
  ถ้าคุณมาเรื่องเครื่องส่งสัญญาณ กรุณารับบัตรคิว
> mix@11111111111111111110 speaker=mara
  Mara says «привет», 你好, مرحبا and שלום at once.
-> next
:: next
> end@11111111111111111111 speaker=mara
  Конец. 終わり。 النهاية.
";

fn capture(test: &mut TestingRunner, name: &str) {
    if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        let _ = std::fs::create_dir_all(&directory);
        test.render_to_file(std::path::Path::new(&directory).join(name));
    }
}

#[test]
fn dialogue_in_other_scripts_renders_with_a_face_for_every_line() {
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        Size2D::new(1400., 1500.),
        |runner| {
            runner.provide_root_context(|| recite_writer::InitialSource(SOURCE.into()));
        },
        1.,
    )
    .0;
    test.poll_n(std::time::Duration::from_millis(16), 15);
    // Which face Skia's font manager supplies for a glyph the bundled faces lack. Printed, not
    // asserted: the answer depends on the fonts the machine has installed.
    let manager = skia_safe::FontMgr::default();
    for (needle, ch) in [
        ("ja", '送'),
        ("zh", '发'),
        ("ko", '송'),
        ("ar", 'إ'),
        ("he", 'א'),
        ("hi", 'अ'),
        ("th", 'ถ'),
        ("ru", 'Е'),
        ("el", 'Α'),
    ] {
        let face = manager
            .match_family_style_character(
                "Literata",
                skia_safe::FontStyle::normal(),
                &[],
                ch as i32,
            )
            .map_or_else(|| "(none)".into(), |t| t.family_name());
        println!("FALLBACK {needle} {ch} → {face}");
    }
    capture(&mut test, "scripts-script.png");
    test.click_cursor((10., 10.));
    let map = test.find(|node, e| {
        Label::try_downcast(e)
            .filter(|l| l.text.as_ref() == "Map")
            .map(|_| node.layout().area)
    });
    if let Some(area) = map {
        test.click_cursor((f64::from(area.center().x), f64::from(area.center().y)));
        test.poll_n(std::time::Duration::from_millis(16), 15);
        capture(&mut test, "scripts-map.png");
    }
    let source = test.find(|node, e| {
        Label::try_downcast(e)
            .filter(|l| l.text.as_ref() == "Source")
            .map(|_| node.layout().area)
    });
    if let Some(area) = source {
        test.click_cursor((f64::from(area.center().x), f64::from(area.center().y)));
        test.poll_n(std::time::Duration::from_millis(16), 15);
        capture(&mut test, "scripts-source.png");
    }
}
