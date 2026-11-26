//! Breaking one rule in a valid generated program is reported with exactly
//! that rule's diagnostic, wherever the broken statement is nested.

use proptest::prelude::*;
use recite_testkit::{ArmSpec, Program, Stmt, Target, program};

use super::{compile_sources, schema};

#[derive(Clone, Copy, Debug)]
enum Breakage {
    /// §5.9.1: a `:case` names a variant the enum does not declare.
    UndeclaredVariant,
    /// §5.9.1: no `:case _`, and a declared variant has no arm.
    UncoveredVariant,
    /// §5.9.1: a variant has two arms.
    DuplicateArm,
    /// §5.9.1: an arm follows `:case _`.
    ArmAfterWildcard,
    /// §5.4.2: two lines share an ID.
    DuplicateLineId,
}

impl Breakage {
    const ALL: [Self; 5] = [
        Self::UndeclaredVariant,
        Self::UncoveredVariant,
        Self::DuplicateArm,
        Self::ArmAfterWildcard,
        Self::DuplicateLineId,
    ];

    fn code(self) -> &'static str {
        match self {
            Self::UndeclaredVariant => "RECITE_VALIDATE054",
            Self::UncoveredVariant => "RECITE_VALIDATE055",
            Self::DuplicateArm => "RECITE_VALIDATE052",
            Self::ArmAfterWildcard => "RECITE_VALIDATE053",
            Self::DuplicateLineId => "RECITE_ID003",
        }
    }

    /// Breaks the rule at the `site`-th applicable statement, or returns
    /// `None` when the program has none.
    fn apply(self, program: &Program, site: usize) -> Option<Program> {
        let mut broken = program.clone();
        if let Self::DuplicateLineId = self {
            let mut first = None;
            let mut lines = 0;
            broken.visit(&mut |statement| {
                if let Stmt::Line(line) = statement {
                    first.get_or_insert_with(|| line.id.clone());
                    lines += 1;
                }
            });
            let first = first.filter(|_| lines >= 2)?;
            let chosen = 1 + site % (lines - 1);
            let mut seen = 0;
            broken.visit_mut(&mut |statement| {
                if let Stmt::Line(line) = statement {
                    if seen == chosen {
                        line.id.clone_from(&first);
                    }
                    seen += 1;
                }
            });
            return Some(broken);
        }
        let mut matches = 0;
        broken.visit(&mut |statement| {
            matches += usize::from(matches!(statement, Stmt::Match { .. }))
        });
        if matches == 0 {
            return None;
        }
        let chosen = site % matches;
        let mut seen = 0;
        broken.visit_mut(&mut |statement| {
            if let Stmt::Match { scrutinee, arms } = statement {
                if seen == chosen {
                    self.break_arms(scrutinee.variants(), arms);
                }
                seen += 1;
            }
        });
        Some(broken)
    }

    fn break_arms(self, variants: &[&'static str], arms: &mut Vec<ArmSpec>) {
        let arm = |pattern| ArmSpec {
            pattern,
            body: vec![Stmt::Divert(Target::End)],
        };
        let wildcard = arms.iter().position(|arm| arm.pattern.is_none());
        match self {
            Self::UndeclaredVariant => {
                if wildcard.is_none() {
                    arms.push(arm(None));
                }
                arms.insert(0, arm(Some("sleepy")));
            }
            Self::UncoveredVariant => {
                arms.retain(|arm| arm.pattern.is_some());
                if arms.len() == variants.len() {
                    arms.remove(0);
                }
                if arms.is_empty() {
                    arms.push(arm(Some(variants[0])));
                }
            }
            Self::DuplicateArm => {
                let pattern = arms[0].pattern.or(Some(variants[0]));
                arms.insert(0, arm(pattern));
                if pattern != arms[1].pattern {
                    // The match was only `:case _`; repeat the new arm.
                    arms.insert(0, arm(pattern));
                }
            }
            Self::ArmAfterWildcard => {
                let moved = wildcard.map_or_else(|| arm(None), |index| arms.remove(index));
                arms.insert(0, moved);
                if arms.len() == 1 {
                    arms.push(arm(Some(variants[0])));
                }
            }
            Self::DuplicateLineId => {}
        }
    }
}

proptest! {
    #[test]
    fn breaking_one_rule_reports_exactly_its_diagnostic(
        program in program(),
        breakage in prop::sample::select(Breakage::ALL.to_vec()),
        site in any::<usize>(),
    ) {
        let Some(broken) = breakage.apply(&program, site) else {
            return Ok(());
        };
        let report = compile_sources(&broken.sources(), &schema());
        let codes: Vec<_> = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();
        // A rule broken in several places, such as two arms after `:case _`,
        // is reported at each of them.
        prop_assert!(
            !codes.is_empty() && codes.iter().all(|code| *code == breakage.code()),
            "{breakage:?} reported {codes:?}"
        );
        prop_assert!(report.asset.is_none(), "a rejected program must not compile");
    }
}
