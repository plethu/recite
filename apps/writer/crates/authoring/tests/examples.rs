use recite_compiler::{CompileInput, CompileOptions, compile_inputs_with_schema};
use recite_core::{CompiledAssetId, CompilerVersion, SourceMapId, load_schema_manifest_str};
use recite_runtime::{PreviewEvent, PreviewInputs, PreviewOptions, PreviewSession};
use recite_writer_model::WRITER_EXAMPLES;

struct Trace {
    lines: Vec<String>,
    effects: Vec<String>,
    prompts: usize,
}

fn trace(index: usize, choices: &[usize]) -> Result<Trace, Box<dyn std::error::Error>> {
    let example = &WRITER_EXAMPLES[index];
    let workbench = example.open()?;
    assert!(
        workbench.document().diagnostics().is_empty(),
        "{:?}",
        workbench.document().diagnostics()
    );
    let report = load_schema_manifest_str(
        "writer_examples.json",
        include_str!("../../../../../fixtures/schema/valid/writer_examples.json"),
    );
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let schema = report.schema.ok_or("schema missing")?;
    let compiled = compile_inputs_with_schema(
        [CompileInput::new(example.name, example.source)],
        CompileOptions::new(
            CompilerVersion::new("0.1.0")?,
            CompiledAssetId::new("examples")?,
            SourceMapId::new("examples.map")?,
            schema.canonical_fingerprint(),
        ),
        &schema,
    )?;
    let asset = compiled.asset.ok_or("example did not compile")?.dialogue;
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new())?;
    let mut trace = Trace {
        lines: vec![],
        effects: vec![],
        prompts: 0,
    };
    let mut choices = choices.iter();
    let mut pending = None;
    for _ in 0..80 {
        let output = if let Some(choice) = pending.take() {
            preview.choose(choice, PreviewInputs::new())
        } else {
            preview.step(PreviewInputs::new())
        };
        for event in output.events() {
            match event {
                PreviewEvent::Line(line) => trace.lines.push(line.text.clone()),
                PreviewEvent::Prompt(prompt) => {
                    trace.prompts += 1;
                    if let Some(line) = prompt.line() {
                        trace.lines.push(line.text.clone());
                    }
                    let index = *choices.next().ok_or("unexpected prompt")?;
                    pending = Some(
                        prompt
                            .choices()
                            .get(index)
                            .ok_or("choice missing")?
                            .id
                            .clone(),
                    );
                }
                PreviewEvent::EffectRequested(effect) => {
                    trace.effects.push(effect.function.clone())
                }
                PreviewEvent::ChoiceSelected { .. } => {}
                PreviewEvent::End { .. } => {
                    assert!(
                        choices.next().is_none(),
                        "scene ended before the requested route"
                    );
                    return Ok(trace);
                }
                other => panic!("unexpected event: {other:?}"),
            }
        }
    }
    Err("route did not terminate".into())
}

#[test]
fn hub_returns_from_nested_topics_and_emits_consequences() -> Result<(), Box<dyn std::error::Error>>
{
    // Work -> route -> shelter -> route -> work -> accept -> hub -> insult -> hub -> leave.
    let result = trace(0, &[0, 0, 0, 1, 1, 2, 3])?;
    assert_eq!(
        result.effects,
        ["set_flag", "change_disposition", "change_disposition"]
    );
    assert_eq!(
        result
            .lines
            .iter()
            .filter(|line| line.starts_with("If you're here"))
            .count(),
        3
    );
    assert!(
        result
            .lines
            .iter()
            .any(|line| line.starts_with("A pump house"))
    );
    // History -> answer -> history -> hub -> leave.
    let history = trace(0, &[1, 0, 1, 3])?;
    assert!(
        history
            .lines
            .iter()
            .any(|line| line.starts_with("Last week."))
    );
    assert!(history.effects.is_empty());
    Ok(())
}

#[test]
fn every_waterfall_route_converges_and_rebranches() -> Result<(), Box<dyn std::error::Error>> {
    for first in 0..2 {
        for second in 0..2 {
            for ending in 0..2 {
                let result = trace(1, &[first, second, ending])?;
                assert_eq!(result.prompts, 3);
                assert!(result.effects.is_empty());
                assert!(
                    result
                        .lines
                        .iter()
                        .any(|line| line.starts_with("Everyone's out."))
                );
                assert!(
                    result
                        .lines
                        .iter()
                        .any(|line| line.starts_with("Everyone's accounted for."))
                );
                assert_eq!(
                    result.lines.last().ok_or("ending")?,
                    if ending == 0 {
                        "Give us a moment, everyone. We owe you the whole story."
                    } else {
                        "I made a choice down there. You deserve to know why."
                    }
                );
            }
        }
    }
    Ok(())
}

#[test]
fn linear_scene_is_exactly_three_lines_without_choices() -> Result<(), Box<dyn std::error::Error>> {
    let result = trace(2, &[])?;
    assert_eq!(
        result.lines,
        [
            "Is this the last tram?",
            "Only if we get on it.",
            "Hold the door, then."
        ]
    );
    assert_eq!(result.prompts, 0);
    assert!(result.effects.is_empty());
    Ok(())
}
