#![cfg(test)]
//! Save-impact analysis classifies every save point of a shipped asset.

use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::compiled::{
    CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
};
use recite_runtime::resume::{
    PromptResume, ResumeFallbackReason, ResumeLevel, SavePointKind, SavePointOutcome,
    analyze_save_impact,
};

const BASE: &str = concat!(
    ":: start default\n",
    "> @l_one\n",
    "  One.\n",
    "! blocking wait(door)\n",
    "> @l_two\n",
    "  Two.\n",
    "  ? @c_go\n",
    "    Go.\n",
    "    -> next\n",
    "\n",
    ":: next\n",
    "> @l_three\n",
    "  Three.\n",
    "-> END\n",
);

fn compile(source: &str) -> CompiledDialogue {
    compile_inputs(
        [CompileInput::new("dialogue/a.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("compiler"),
            CompiledAssetId::new("dialogue/main.recitec").expect("asset id"),
            SourceMapId::new("dialogue/main.recitec.map").expect("map id"),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("compiles")
    .asset
    .expect("asset")
    .dialogue
}

#[test]
fn identical_assets_resume_every_point_exactly() {
    let base = compile(BASE);
    let report = analyze_save_impact(&base, &compile(BASE)).expect("analyses");
    assert!(report.identical);
    let summary = report.summary();
    // Two block starts, two lines, one blocking effect, one prompt.
    assert_eq!(summary.total, 6);
    assert_eq!(summary.exact, 6);
    assert_eq!(report.fallbacks().count(), 0);
}

#[test]
fn edits_are_classified_per_save_point() {
    let base = compile(BASE);
    let patched = compile(
        &BASE
            .replacen("! blocking wait(door)\n", "! blocking wait(gate)\n", 1)
            .replacen("  Three.\n", "  Three, edited.\n", 1)
            .replacen(
                "    -> next\n",
                "    -> next\n  ? @c_stay\n    Stay.\n    -> END\n",
                1,
            ),
    );
    let report = analyze_save_impact(&base, &patched).expect("analyses");
    assert!(!report.identical);
    let outcome = |kind: &SavePointKind| {
        report
            .points
            .iter()
            .find(|point| &point.kind == kind)
            .map(|point| point.outcome.clone())
            .expect("save point is reported")
    };

    assert!(matches!(
        outcome(&SavePointKind::BlockingEffect {
            function: "wait".to_owned()
        }),
        SavePointOutcome::Fallback {
            reason: ResumeFallbackReason::StatementsChanged { .. },
            block_available: true,
        }
    ));
    // The line before the changed effect relocates through its own ID.
    assert_eq!(
        outcome(&SavePointKind::AfterLine {
            line: recite_core::LineId::new("l_one").expect("id")
        }),
        SavePointOutcome::Resumes {
            level: ResumeLevel::Relocated,
            prompt: PromptResume::None,
        }
    );
    let prompt = report
        .points
        .iter()
        .find(|point| matches!(point.kind, SavePointKind::Prompt { .. }))
        .expect("prompt");
    assert_eq!(
        prompt.outcome,
        SavePointOutcome::Resumes {
            level: ResumeLevel::Relocated,
            prompt: PromptResume::Reevaluated,
        }
    );
    assert_eq!(prompt.baseline_span.start.line(), 5);
    assert_eq!(
        outcome(&SavePointKind::AfterLine {
            line: recite_core::LineId::new("l_three").expect("id")
        }),
        SavePointOutcome::Resumes {
            level: ResumeLevel::TextEdited,
            prompt: PromptResume::None,
        }
    );
    let summary = report.summary();
    assert_eq!(summary.fallback, 1);
    assert_eq!(summary.reevaluated_prompts, 1);
    let fallback = report.fallbacks().next().expect("one fallback");
    assert_eq!(fallback.block_id.as_str(), "start");
    assert!(fallback.candidate_block_span.is_some());
}

#[test]
fn removed_block_reports_no_candidate_span() {
    let base = compile(BASE);
    let patched = compile(&BASE.replacen(":: next\n", ":: later\n", 1).replacen(
        "-> next\n",
        "-> later\n",
        1,
    ));
    let report = analyze_save_impact(&base, &patched).expect("analyses");
    let removed = report
        .fallbacks()
        .filter(|point| point.block_id.as_str() == "next")
        .collect::<Vec<_>>();
    assert_eq!(removed.len(), 2);
    for point in removed {
        assert_eq!(point.candidate_block_span, None);
        assert!(matches!(
            point.outcome,
            SavePointOutcome::Fallback {
                reason: ResumeFallbackReason::BlockRemoved,
                block_available: false,
            }
        ));
    }
}

#[test]
fn statements_no_player_can_reach_are_not_save_points() {
    let reachable = concat!(
        ":: start default\n",
        "> @l_one\n",
        "  One.\n",
        ":if ready()\n",
        "  -> END\n",
        "> @l_ask\n",
        "  Where?\n",
        "  ? @c_go\n",
        "    Go.\n",
        "    -> END\n",
    );
    let with_dead_code = reachable.replace(
        "  -> END\n> @l_ask",
        "  -> END\n  > @l_dead_branch\n    Dead.\n> @l_ask",
    ) + "> @l_dead\n  Dead.\n";
    let base = compile(&with_dead_code);
    let report = analyze_save_impact(&base, &compile(reachable)).expect("analyses");
    // A block start, a line, a prompt: nothing after the divert or the prompt.
    assert_eq!(report.summary().total, 3);
    assert_eq!(report.fallbacks().count(), 0);
}

#[test]
fn statements_after_a_branch_that_always_leaves_are_not_save_points() {
    for branch in [
        ":if ready()\n  -> work\n:else\n  -> END\n",
        ":match mood()\n  :case tired\n    -> work\n  :case _\n    -> END\n",
    ] {
        let reachable =
            format!(":: start default\n> @l_a\n  A.\n{branch}\n:: work\n> @l_work\n  W.\n-> END\n");
        let with_dead_code = reachable.replace(
            &format!("{branch}\n"),
            &format!("{branch}> @l_dead\n  Dead.\n! immediate dead_fx()\n\n"),
        );
        let report =
            analyze_save_impact(&compile(&with_dead_code), &compile(&reachable)).expect("analyses");
        // Two block starts and two lines: nothing after the branch.
        assert_eq!(report.summary().total, 4, "{branch}");
        assert_eq!(report.fallbacks().count(), 0, "{branch}");
    }
    // A branch that can fall through keeps the statements after it.
    let falls_through = concat!(
        ":: start default\n",
        ":if ready()\n",
        "  -> END\n",
        "> @l_after\n",
        "  After.\n",
        "-> END\n",
    );
    let report =
        analyze_save_impact(&compile(falls_through), &compile(falls_through)).expect("analyses");
    assert_eq!(report.summary().total, 2);
}

#[test]
fn match_arms_traversal_can_never_select_are_not_save_points() {
    let selectable = concat!(
        ":: start default\n",
        ":match mood()\n",
        "  :case tired\n",
        "    > @l_tired\n",
        "      Tired.\n",
        "  :case _\n",
        "    > @l_other\n",
        "      Other.\n",
        "> @l_after\n",
        "  After.\n",
        "-> END\n",
    );
    let with_repeats = selectable.replace(
        "  :case _\n",
        "  :case tired\n    > @l_again\n      Again.\n  :case _\n",
    );
    let with_repeats = with_repeats.replace(
        "> @l_after\n",
        "  :case _\n    > @l_second\n      Second.\n> @l_after\n",
    );
    let report =
        analyze_save_impact(&compile(&with_repeats), &compile(selectable)).expect("analyses");
    // A block start and three lines: the repeated arms are never entered.
    assert_eq!(report.summary().total, 4);
    assert_eq!(report.fallbacks().count(), 0);
}
