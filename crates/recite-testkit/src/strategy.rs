//! Proptest strategies for generated programs.
//!
//! Strategies produce raw shapes with unassigned IDs and target selectors;
//! [`Program::normalize`] turns a shape into a valid program, so shrinking
//! acts on the shape and every shrunk case stays valid.

use proptest::collection::vec;
use proptest::option;
use proptest::prelude::*;

use crate::program::{
    Actor, ArmSpec, ChoiceSpec, Cond, EffectMode, EffectSpec, FileSpec, LineSpec, Program,
    Scrutinee, Stmt, Target,
};

/// Marks a target whose block is chosen by normalization from a selector.
pub(crate) const RAW_FILE: usize = usize::MAX;

const WORDS: [&str; 8] = [
    "Hello", "Maybe", "Onward", "Quiet", "Rain", "Sure", "Tomorrow", "Wait",
];

/// Programs of one to four blocks in a main file and up to two in a second
/// file, with `:if`/`:match` nested up to two levels.
pub fn program() -> impl Strategy<Value = Program> {
    (vec(body(2), 1..=4), vec(body(2), 0..=2)).prop_map(|(main, side)| {
        let mut files = vec![file("dialogue/main.recite", 'm', main)];
        if !side.is_empty() {
            files.push(file("dialogue/side.recite", 's', side));
        }
        let mut program = Program { files, next_id: 0 };
        program.normalize();
        program
    })
}

fn file(path: &str, prefix: char, bodies: Vec<Vec<Stmt>>) -> FileSpec {
    FileSpec {
        path: path.to_owned(),
        blocks: bodies
            .into_iter()
            .enumerate()
            .map(|(index, body)| crate::program::BlockSpec {
                id: format!("{prefix}{index}"),
                body,
            })
            .collect(),
    }
}

pub(crate) fn text() -> impl Strategy<Value = String> {
    (0..WORDS.len(), 0..WORDS.len())
        .prop_map(|(first, second)| format!("{} {}.", WORDS[first], WORDS[second].to_lowercase()))
}

fn actor() -> impl Strategy<Value = Actor> {
    prop_oneof![Just(Actor::Player), Just(Actor::Rhea)]
}

pub(crate) fn cond() -> BoxedStrategy<Cond> {
    prop_oneof![actor().prop_map(Cond::Flag), Just(Cond::Ready)]
        .prop_recursive(2, 6, 3, |inner| {
            prop_oneof![
                inner
                    .clone()
                    .prop_map(|operand| Cond::Not(Box::new(operand))),
                vec(inner.clone(), 2..=3).prop_map(Cond::And),
                vec(inner, 2..=3).prop_map(Cond::Or),
            ]
        })
        .boxed()
}

fn raw_target() -> impl Strategy<Value = Target> {
    prop_oneof![
        1 => Just(Target::End),
        4 => any::<u8>().prop_map(|selector| Target::Block {
            file: RAW_FILE,
            block: usize::from(selector),
        }),
    ]
}

fn effect() -> impl Strategy<Value = EffectSpec> {
    let mode = prop_oneof![
        Just(EffectMode::Immediate),
        Just(EffectMode::Deferred),
        Just(EffectMode::Blocking),
    ];
    prop_oneof![
        3 => (mode, actor()).prop_map(|(mode, actor)| EffectSpec::Note(mode, actor)),
        1 => Just(EffectSpec::Ping),
        1 => actor().prop_map(EffectSpec::Grant),
    ]
}

fn choice() -> impl Strategy<Value = ChoiceSpec> {
    (
        text(),
        option::weighted(0.4, cond()),
        raw_target(),
        any::<bool>(),
    )
        .prop_map(|(text, requires, target, comment_before)| ChoiceSpec {
            id: String::new(),
            text,
            requires,
            target,
            comment_before,
        })
}

fn line(prompt: bool) -> BoxedStrategy<Stmt> {
    let choices = if prompt {
        vec(choice(), 1..=3).boxed()
    } else {
        Just(Vec::new()).boxed()
    };
    (
        option::of(0_u8..2),
        prop::bool::weighted(0.25),
        text(),
        choices,
    )
        .prop_map(|(speaker, count, text, choices)| {
            Stmt::Line(LineSpec {
                id: String::new(),
                speaker,
                count,
                text,
                choices,
            })
        })
        .boxed()
}

fn leaf() -> BoxedStrategy<Stmt> {
    prop_oneof![
        4 => line(false),
        3 => effect().prop_map(Stmt::Effect),
        1 => Just(Stmt::Comment),
    ]
    .boxed()
}

fn terminal() -> BoxedStrategy<Stmt> {
    prop_oneof![
        line(true),
        vec(choice(), 1..=3).prop_map(Stmt::Choices),
        raw_target().prop_map(Stmt::Divert),
    ]
    .boxed()
}

/// A statement list: simple statements, then optionally one prompt or
/// divert, which must come last.
pub(crate) fn body(depth: u32) -> BoxedStrategy<Vec<Stmt>> {
    let simple = if depth == 0 {
        leaf()
    } else {
        prop_oneof![
            6 => leaf(),
            1 => if_statement(depth - 1),
            1 => match_statement(depth - 1),
        ]
        .boxed()
    };
    (vec(simple, 0..4), option::weighted(0.6, terminal()))
        .prop_map(|(mut statements, terminal)| {
            statements.extend(terminal);
            statements
        })
        .boxed()
}

fn if_statement(depth: u32) -> BoxedStrategy<Stmt> {
    let otherwise = prop_oneof![Just(Vec::new()), body(depth)];
    (cond(), body(depth), otherwise)
        .prop_map(|(condition, then, otherwise)| Stmt::If {
            condition,
            then,
            otherwise,
        })
        .boxed()
}

fn match_statement(depth: u32) -> BoxedStrategy<Stmt> {
    let scrutinee = prop_oneof![Just(Scrutinee::Mood), actor().prop_map(Scrutinee::Stage),];
    let order = Just(vec![0_usize, 1, 2]).prop_shuffle();
    (
        scrutinee,
        order,
        1_usize..=3,
        any::<bool>(),
        vec(body(depth), 4),
    )
        .prop_map(|(scrutinee, order, count, wildcard, bodies)| {
            let variants = scrutinee.variants();
            let patterns: Vec<_> = order
                .into_iter()
                .filter(|index| *index < variants.len())
                .take(count)
                .map(|index| Some(variants[index]))
                .collect();
            // An arm for every variant makes `:case _` optional; otherwise
            // the match needs it to be exhaustive.
            let wildcard = wildcard || patterns.len() < variants.len();
            let arms = patterns
                .into_iter()
                .chain(wildcard.then_some(None))
                .zip(bodies)
                .map(|(pattern, body)| ArmSpec { pattern, body })
                .collect();
            Stmt::Match { scrutinee, arms }
        })
        .boxed()
}
