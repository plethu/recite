//! Host answers for a generated run: condition results, the interpolated
//! `$count` value and a deterministic choice policy.

use proptest::prelude::*;

use crate::program::{Actor, Scrutinee};

/// One fixed assignment of every generated condition, so conditions stay
/// pure: the same call returns the same answer for the whole run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Answers {
    pub flags: [bool; 2],
    pub ready: bool,
    pub mood: usize,
    pub stages: [usize; 2],
    pub count: i64,
    /// Seeds the choice policy.
    pub decisions: u64,
}

/// A condition result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Answer {
    Bool(bool),
    Variant(&'static str),
}

impl Answers {
    /// The answer to `function(args)`, or `None` for a call the schema does
    /// not declare.
    #[must_use]
    pub fn answer(&self, function: &str, args: &[&str]) -> Option<Answer> {
        let actor = |args: &[&str]| match args {
            ["player"] => Some(Actor::Player),
            ["rhea"] => Some(Actor::Rhea),
            _ => None,
        };
        match function {
            "flag" => actor(args).map(|actor| Answer::Bool(self.flags[actor.index()])),
            "ready" if args.is_empty() => Some(Answer::Bool(self.ready)),
            "mood" if args.is_empty() => Some(Answer::Variant(Scrutinee::MOODS[self.mood])),
            "stage" => actor(args)
                .map(|actor| Answer::Variant(Scrutinee::STAGES[self.stages[actor.index()]])),
            _ => None,
        }
    }

    /// The choice a player takes among `available` choice IDs. The policy
    /// depends only on the IDs, so the same prompt gets the same answer in a
    /// baseline and a patched program.
    #[must_use]
    pub fn pick<'a>(&self, available: &[&'a str]) -> Option<&'a str> {
        available
            .iter()
            .copied()
            .min_by_key(|id| fnv1a(self.decisions, id.as_bytes()))
    }
}

fn fnv1a(seed: u64, bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64 ^ seed;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

pub fn answers() -> impl Strategy<Value = Answers> {
    (
        any::<[bool; 2]>(),
        any::<bool>(),
        0_usize..3,
        [0_usize..2, 0_usize..2],
        0_i64..5,
        any::<u64>(),
    )
        .prop_map(|(flags, ready, mood, stages, count, decisions)| Answers {
            flags,
            ready,
            mood,
            stages,
            count,
            decisions,
        })
}
