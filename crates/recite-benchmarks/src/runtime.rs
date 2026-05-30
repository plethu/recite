use recite_core::CompiledDialogue;
use recite_runtime::{
    DialogueEffectMode, DialogueEvent, DialogueSession, DialogueSessionOptions, EffectAck,
    LocaleResolution, acknowledge_effect, choose, decode_session_messagepack,
    encode_session_messagepack, next, next_with, start_scene, start_scene_with_options,
};

use crate::catalog::CatalogProvider;
use crate::compiler::CompiledProject;
use crate::fixture_context::RuntimeFixture;
use crate::project::BenchmarkProject;
use crate::{BenchmarkResult, error};

#[derive(Clone, Debug)]
pub struct RuntimeProject {
    dialogue: CompiledDialogue,
    fixture: RuntimeFixture,
    catalog: CatalogProvider,
}

impl RuntimeProject {
    pub fn load(project: &BenchmarkProject, compiled: &CompiledProject) -> BenchmarkResult<Self> {
        let fixture = RuntimeFixture::load(&project.runtime_fixture_source()?)?;
        let catalog = CatalogProvider::load(project, &fixture)?;
        Ok(Self {
            dialogue: compiled.asset().dialogue.clone(),
            fixture,
            catalog,
        })
    }

    #[must_use]
    pub fn driver(&self) -> TraversalDriver<'_> {
        TraversalDriver {
            asset: &self.dialogue,
            fixture: &self.fixture,
            catalog: &self.catalog,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TraversalDriver<'a> {
    asset: &'a CompiledDialogue,
    fixture: &'a RuntimeFixture,
    catalog: &'a CatalogProvider,
}

impl<'a> TraversalDriver<'a> {
    pub fn start_scene(&self) -> BenchmarkResult<DialogueSession> {
        start_scene(self.asset, None).map_err(Into::into)
    }

    pub fn start_localised_scene(&self) -> BenchmarkResult<DialogueSession> {
        start_scene_with_options(
            self.asset,
            None,
            DialogueSessionOptions::new().with_locale(self.fixture.locale()),
        )
        .map_err(Into::into)
    }

    pub fn session_before_first_line(&self) -> BenchmarkResult<DialogueSession> {
        start_scene(self.asset, Some("block_00001")).map_err(Into::into)
    }

    pub fn localised_session_before_first_line(&self) -> BenchmarkResult<DialogueSession> {
        start_scene_with_options(
            self.asset,
            Some("block_00001"),
            DialogueSessionOptions::new().with_locale(self.fixture.locale()),
        )
        .map_err(Into::into)
    }

    pub fn session_before_first_prompt(&self) -> BenchmarkResult<DialogueSession> {
        start_scene(self.asset, Some("block_00002")).map_err(Into::into)
    }

    pub fn session_before_condition_prompt(&self) -> BenchmarkResult<DialogueSession> {
        let mut session = self.start_scene()?;
        self.consume_next_effect(&mut session, DialogueEffectMode::Immediate)?;
        self.consume_next_effect(&mut session, DialogueEffectMode::Blocking)?;
        Ok(session)
    }

    pub fn session_with_prompt(&self) -> BenchmarkResult<DialogueSession> {
        let mut session = self.session_before_condition_prompt()?;
        expect_prompt(next(self.asset, &mut session, self.fixture)?)?;
        Ok(session)
    }

    pub fn session_before_blocking_effect(&self) -> BenchmarkResult<DialogueSession> {
        let mut session = self.start_scene()?;
        self.consume_next_effect(&mut session, DialogueEffectMode::Immediate)?;
        Ok(session)
    }

    pub fn session_before_deferred_effect(&self) -> BenchmarkResult<DialogueSession> {
        start_scene(self.asset, Some("block_00007")).map_err(Into::into)
    }

    pub fn next_line(&self, session: &mut DialogueSession) -> BenchmarkResult<DialogueEvent> {
        let event = next(self.asset, session, self.fixture)?;
        expect_line(event)
    }

    pub fn next_prompt(&self, session: &mut DialogueSession) -> BenchmarkResult<DialogueEvent> {
        let event = next(self.asset, session, self.fixture)?;
        expect_prompt(event)
    }

    pub fn choose_first(&self, session: &mut DialogueSession) -> BenchmarkResult<DialogueEvent> {
        let choice = self.fixture.choice_for_line("line_00000_000")?;
        choose(self.asset, session, choice, self.fixture).map_err(Into::into)
    }

    pub fn condition_dispatch(
        &self,
        session: &mut DialogueSession,
    ) -> BenchmarkResult<DialogueEvent> {
        self.next_prompt(session)
    }

    pub fn immediate_effect(
        &self,
        session: &mut DialogueSession,
    ) -> BenchmarkResult<DialogueEvent> {
        let event = next(self.asset, session, self.fixture)?;
        expect_effect(event, DialogueEffectMode::Immediate)
    }

    pub fn deferred_effect(&self, session: &mut DialogueSession) -> BenchmarkResult<DialogueEvent> {
        let before = session.deferred_effects().len();
        let event = next(self.asset, session, self.fixture)?;
        let deferred = session.deferred_effects();
        match deferred.get(before) {
            Some(effect)
                if deferred.len() == before + 1 && effect.mode == DialogueEffectMode::Deferred =>
            {
                Ok(event)
            }
            _ => Err(error("expected one deferred effect to be collected")),
        }
    }

    pub fn blocking_effect(&self, session: &mut DialogueSession) -> BenchmarkResult<DialogueEvent> {
        let event = next(self.asset, session, self.fixture)?;
        expect_effect(event, DialogueEffectMode::Blocking)
    }

    pub fn acknowledge_blocking(&self, session: &mut DialogueSession) -> BenchmarkResult<()> {
        let Some(effect) = session.pending_effect() else {
            return Err(error(
                "session has no pending blocking effect to acknowledge",
            ));
        };
        acknowledge_effect(session, effect.id.clone(), EffectAck::Completed).map_err(Into::into)
    }

    pub fn localised_next(&self, session: &mut DialogueSession) -> BenchmarkResult<DialogueEvent> {
        let event = next_with(
            self.asset,
            session,
            self.fixture,
            LocaleResolution::new().with_provider(self.catalog),
        )?;
        expect_line(event)
    }

    pub fn encoded_prompt_session(&self) -> BenchmarkResult<Vec<u8>> {
        let session = self.session_with_prompt()?;
        encode_session_messagepack(&session).map_err(Into::into)
    }

    pub fn decode_session(&self, bytes: &[u8]) -> BenchmarkResult<DialogueSession> {
        decode_session_messagepack(self.asset, bytes).map_err(Into::into)
    }

    pub fn encode_session(&self, session: &DialogueSession) -> BenchmarkResult<Vec<u8>> {
        encode_session_messagepack(session).map_err(Into::into)
    }

    pub fn full_traversal(&self) -> BenchmarkResult<usize> {
        let mut events = 0;
        let mut session = self.start_scene()?;
        let mut current_event = None;
        loop {
            let event = match current_event.take() {
                Some(event) => event,
                None => next(self.asset, &mut session, self.fixture)?,
            };
            events += 1;
            match event {
                DialogueEvent::Prompt { line, .. } => {
                    let line_id = line
                        .as_ref()
                        .map(|line| line.id.as_str())
                        .ok_or_else(|| error("benchmark prompt did not include a line"))?;
                    let choice = self.fixture.choice_for_line(line_id)?;
                    let selected = choose(self.asset, &mut session, choice, self.fixture)?;
                    current_event = Some(selected);
                }
                DialogueEvent::Effect(effect) if effect.mode == DialogueEffectMode::Blocking => {
                    if self.fixture.auto_ack_blocking() {
                        acknowledge_effect(&mut session, effect.id, EffectAck::Completed)?;
                    }
                }
                DialogueEvent::End { .. } => return Ok(events),
                DialogueEvent::Line(_) | DialogueEvent::Effect(_) => {}
            }
        }
    }

    fn consume_next_effect(
        &self,
        session: &mut DialogueSession,
        mode: DialogueEffectMode,
    ) -> BenchmarkResult<()> {
        let event = next(self.asset, session, self.fixture)?;
        let DialogueEvent::Effect(effect) = event else {
            return Err(error(format!("expected {mode} effect event")));
        };
        if effect.mode != mode {
            return Err(error(format!(
                "expected {mode} effect event, got {}",
                effect.mode
            )));
        }
        if mode == DialogueEffectMode::Blocking {
            acknowledge_effect(session, effect.id, EffectAck::Completed)?;
        }
        Ok(())
    }
}

fn expect_line(event: DialogueEvent) -> BenchmarkResult<DialogueEvent> {
    match event {
        DialogueEvent::Line(_) => Ok(event),
        other => Err(error(format!("expected line event, got {other:?}"))),
    }
}

fn expect_prompt(event: DialogueEvent) -> BenchmarkResult<DialogueEvent> {
    match event {
        DialogueEvent::Prompt { .. } => Ok(event),
        other => Err(error(format!("expected prompt event, got {other:?}"))),
    }
}

fn expect_effect(event: DialogueEvent, mode: DialogueEffectMode) -> BenchmarkResult<DialogueEvent> {
    match &event {
        DialogueEvent::Effect(effect) if effect.mode == mode => Ok(event),
        other => Err(error(format!(
            "expected {mode} effect event, got {other:?}"
        ))),
    }
}
