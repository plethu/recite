#![cfg(test)]

use recite_core::{
    Argument, Block, Choice, ChoiceEcho, ChoiceTarget, ConditionExpression, DivertTarget,
    EffectMode, IfBranch, Line, MatchBranch, MatchPattern, ScalarValue, SourceMetadataScalar,
    SourceMetadataValue, SpeakerId, Statement, StatementKind,
};
use recite_parser::{LoweredSourceFile, ReciteSyntaxKind, parse};

const TEST_PATH: &str = "dialogue/tavern.recite";

#[path = "parser/branches_and_recovery.rs"]
mod branches_and_recovery;
#[path = "parser/diagnostic_contract.rs"]
mod diagnostic_contract;
#[path = "../../../tests/support/fixtures.rs"]
mod fixture_support;
#[path = "parser/lowering.rs"]
mod lowering;
#[path = "parser/metadata.rs"]
mod metadata;
#[path = "parser/spans.rs"]
mod spans;
#[path = "parser/statements.rs"]
mod statements;
#[path = "parser/support.rs"]
mod support;
#[path = "parser/syntax_and_fixtures.rs"]
mod syntax_and_fixtures;

use fixture_support::{assert_diagnostic_snapshot, fixture_source};
use support::*;
