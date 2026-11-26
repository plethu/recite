//! Shared setup for Recite's fuzz targets: the generated-program schema, a
//! fixed asset saves are restored against, and bounded play.

use std::sync::OnceLock;

use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs_with_schema};
use recite_core::{
    compiled::{CompiledAssetId, CompiledDialogue, CompilerVersion, SourceMapId},
    schema::{ProjectSchema, load_schema_manifest_str},
};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueEvent, DialogueSession,
    EffectAck, LocaleResolution, acknowledge_effect, choose, localisation::InterpolationValues,
    next_with,
};

/// The asset every session-snapshot input is restored and resumed against.
/// Its ID matches generated programs, so saves from any generated program
/// reach relocation rather than failing the asset check.
pub const FIXED_SOURCE: &str = concat!(
    ":: m0 default\n",
    "> speaker=s0 @l0\n",
    "  Hello.\n",
    "! immediate note(player)\n",
    "! deferred note(rhea)\n",
    ":if flag(player) and not ready()\n",
    "  > bind=(count:int=$count) @l1\n",
    "    Count {count}.\n",
    "  ! blocking grant(rhea)\n",
    ":else\n",
    "  :match stage(rhea)\n",
    "    :case early\n",
    "      > @l2\n",
    "        Early.\n",
    "    :case _\n",
    "      -> m1\n",
    "> @l3\n",
    "  Where now?\n",
    "  ? requires=(ready()) @c4\n",
    "    Onward.\n",
    "    -> m1\n",
    "  ? @c5\n",
    "    Stop.\n",
    "    -> END\n",
    "\n",
    ":: m1\n",
    ":match mood()\n",
    "  :case calm\n",
    "    > @l6\n",
    "      Calm.\n",
    "  :case _\n",
    "    ! immediate ping()\n",
    "? @c7\n",
    "  Again.\n",
    "  -> m0\n",
    "# between choices\n",
    "? @c8\n",
    "  Leave.\n",
    "  -> END\n",
);

/// The schema generated programs and the fixed asset validate against.
pub fn schema() -> &'static ProjectSchema {
    static SCHEMA: OnceLock<ProjectSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        load_schema_manifest_str(recite_testkit::SCHEMA_PATH, recite_testkit::SCHEMA_JSON)
            .schema
            .unwrap_or_else(|| panic!("the generated schema is valid"))
    })
}

/// Compiles `sources` against the generated schema. Diagnostics are allowed;
/// the asset is returned when one is produced.
pub fn compile(sources: &[(String, String)]) -> Option<CompiledDialogue> {
    let schema = schema();
    let options = CompileOptions::new(
        CompilerVersion::new("0.0.1").ok()?,
        CompiledAssetId::new(recite_testkit::ASSET_ID).ok()?,
        SourceMapId::new("generated.recitec.map").ok()?,
        schema.canonical_fingerprint(),
    );
    compile_inputs_with_schema(
        sources
            .iter()
            .map(|(path, source)| CompileInput::new(path.as_str(), source.as_str())),
        options,
        schema,
    )
    .ok()?
    .asset
    .map(|asset| asset.dialogue)
}

pub fn fixed_asset() -> &'static CompiledDialogue {
    static ASSET: OnceLock<CompiledDialogue> = OnceLock::new();
    ASSET.get_or_init(|| {
        compile(&[("dialogue/main.recite".to_owned(), FIXED_SOURCE.to_owned())])
            .unwrap_or_else(|| panic!("the fixed fuzz asset compiles"))
    })
}

fn answer(query: ConditionQuery<'_>) -> Result<ConditionValue, ConditionEvaluationError> {
    Ok(match query.function() {
        "mood" => ConditionValue::EnumVariant("calm".to_owned()),
        "stage" => ConditionValue::EnumVariant("late".to_owned()),
        _ => ConditionValue::Bool(true),
    })
}

/// Advances at most `steps` times, taking the first choice of each prompt
/// and acknowledging blocking effects. Errors end play; they must never
/// panic.
pub fn play(asset: &CompiledDialogue, session: &mut DialogueSession, steps: usize) {
    let mut values = InterpolationValues::new();
    values.insert("count".to_owned(), recite_core::ScalarValue::Integer(2));
    let mut event = next_with(
        asset,
        session,
        &answer,
        LocaleResolution::new().with_values(&values),
    );
    for _ in 0..steps {
        event = match event {
            Ok(DialogueEvent::Prompt { choices, .. }) => match choices.first() {
                Some(choice) => choose(asset, session, choice.id.clone(), &answer),
                None => return,
            },
            Ok(DialogueEvent::Effect(request)) => {
                let _ = acknowledge_effect(session, request.id, EffectAck::Completed);
                next_with(
                    asset,
                    session,
                    &answer,
                    LocaleResolution::new().with_values(&values),
                )
            }
            Ok(DialogueEvent::Line(_)) => next_with(
                asset,
                session,
                &answer,
                LocaleResolution::new().with_values(&values),
            ),
            Ok(DialogueEvent::End { .. }) | Err(_) => return,
        };
    }
}
