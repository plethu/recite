use std::cell::RefCell;
use std::io::{self, IsTerminal, Read, Write};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use recite_core::{ChoiceId, CompiledDialogue};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, DialogueChoice, DialogueContext, DialogueEffectMode,
    DialogueEffectRequest, DialogueEvent, DialogueLine, EffectAck, acknowledge_effect,
    choose as runtime_choose, next as runtime_next, start_scene,
};

use crate::args::{PlayArgs, PlayUi};
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};
use crate::runtime_fixture::load_compiled_asset;
use crate::runtime_format::{
    RuntimeDisplayArgument, format_condition_query, format_effect_arguments,
};
use crate::tui::{
    KeyHints, Keymap, PromptMode, TextBuffer, TuiIntent, TuiSettings, command_quits,
    enter_terminal, map_key, restore_terminal,
};

pub(crate) fn run_play_command(
    args: PlayArgs,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), CliError> {
    let asset = load_compiled_asset(&args.asset)?;
    let settings = TuiSettings::load(args.keymap)?;
    let messages = Messages::load(&settings.locale)?;
    match resolve_ui(args.ui)? {
        ResolvedUi::Plain => {
            let mut stdin = io::stdin().lock();
            let mut ui = PlainPlayUi::new(&mut stdin, stdout, &messages);
            PlayDriver::new(&asset, &args.block).run(&mut ui)
        }
        ResolvedUi::Tui => {
            if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
                return Err(CliError::PlayTuiRequiresTerminal);
            }
            writeln!(stderr, "{}", messages.text(MsgId::PlayTuiStarting))?;
            run_tui_stdio(&asset, &args.block, settings, messages)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResolvedUi {
    Tui,
    Plain,
}

fn resolve_ui(ui: PlayUi) -> Result<ResolvedUi, CliError> {
    match ui {
        PlayUi::Plain => Ok(ResolvedUi::Plain),
        PlayUi::Tui => Ok(ResolvedUi::Tui),
        PlayUi::Auto => {
            if io::stdin().is_terminal() && io::stdout().is_terminal() {
                Ok(ResolvedUi::Tui)
            } else {
                Ok(ResolvedUi::Plain)
            }
        }
    }
}

struct PlayDriver<'a> {
    asset: &'a CompiledDialogue,
    block: &'a str,
}

impl<'a> PlayDriver<'a> {
    fn new(asset: &'a CompiledDialogue, block: &'a str) -> Self {
        Self { asset, block }
    }

    fn run<U: PlayUiAdapter>(self, ui: &mut U) -> Result<(), CliError> {
        ui.start(self.asset, self.block)?;
        let context = InteractiveContext::new(ui);
        let mut session = start_scene(self.asset, Some(self.block))?;
        let mut pending_event = None;

        loop {
            let event = match pending_event.take() {
                Some(event) => event,
                None => match runtime_next(self.asset, &mut session, &context) {
                    Ok(event) => event,
                    Err(error) => return Err(context.resolve_runtime_error(error)),
                },
            };

            match event {
                DialogueEvent::Line(line) => context.line(&line)?,
                DialogueEvent::Prompt { line, choices } => {
                    let choice_id = context.choice(line.as_ref(), &choices)?;
                    context.selected_choice(&choice_id)?;
                    let event = runtime_choose(self.asset, &mut session, choice_id, &context)
                        .map_err(|error| context.resolve_runtime_error(error))?;
                    pending_event = Some(event);
                }
                DialogueEvent::Effect(effect) => {
                    context.effect(&effect)?;
                    if effect.mode == DialogueEffectMode::Blocking {
                        context.acknowledge(&effect)?;
                        acknowledge_effect(&mut session, effect.id.clone(), EffectAck::Completed)?;
                    }
                }
                DialogueEvent::End { deferred_effects } => {
                    context.end(&deferred_effects)?;
                    break;
                }
            }
        }

        Ok(())
    }
}

struct InteractiveContext<'a, U> {
    ui: RefCell<&'a mut U>,
    interrupted: RefCell<bool>,
    ui_error: RefCell<Option<CliError>>,
}

impl<'a, U> InteractiveContext<'a, U> {
    fn new(ui: &'a mut U) -> Self {
        Self {
            ui: RefCell::new(ui),
            interrupted: RefCell::new(false),
            ui_error: RefCell::new(None),
        }
    }
}

impl<U: PlayUiAdapter> InteractiveContext<'_, U> {
    fn line(&self, line: &DialogueLine) -> Result<(), CliError> {
        self.ui.borrow_mut().line(line)
    }

    fn choice(
        &self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<ChoiceId, CliError> {
        loop {
            let choice_result = {
                let mut ui = self.ui.borrow_mut();
                ui.choice(line, choices)
            };
            let selection = match choice_result {
                Ok(selection) => selection,
                Err(CliError::PlayInvalidInput(message)) => {
                    self.ui.borrow_mut().invalid_input(message)?;
                    continue;
                }
                Err(error) => return Err(error),
            };
            match selection {
                ChoiceSelection::Index(index) => {
                    let numeric_id = index.to_string();
                    if let Some(choice) = choices
                        .iter()
                        .find(|choice| choice.id.as_str() == numeric_id)
                    {
                        if choice.is_available {
                            return Ok(choice.id.clone());
                        }
                        let message = unavailable_choice_message(&self.ui, choice);
                        self.ui.borrow_mut().invalid_input(message)?;
                        continue;
                    }
                    if index == 0 || index > choices.len() {
                        let message = self.ui.borrow().message(
                            MsgId::PlayErrorChoiceIndexOutOfRange,
                            [
                                ("index", index.to_string()),
                                ("count", choices.len().to_string()),
                            ],
                        );
                        self.ui.borrow_mut().invalid_input(message)?;
                        continue;
                    }
                    let choice = &choices[index - 1];
                    if choice.is_available {
                        return Ok(choice.id.clone());
                    }
                    let message = unavailable_choice_message(&self.ui, choice);
                    self.ui.borrow_mut().invalid_input(message)?;
                }
                ChoiceSelection::Id(id) => {
                    let choice_id = match ChoiceId::new(id.clone()) {
                        Ok(choice_id) => choice_id,
                        Err(error) => {
                            let message = self.ui.borrow().message(
                                MsgId::PlayErrorChoiceIdInvalid,
                                [("id", id), ("error", error.to_string())],
                            );
                            self.ui.borrow_mut().invalid_input(message)?;
                            continue;
                        }
                    };
                    if let Some(choice) = choices.iter().find(|choice| choice.id == choice_id) {
                        if choice.is_available {
                            return Ok(choice_id);
                        }
                        let message = unavailable_choice_message(&self.ui, choice);
                        self.ui.borrow_mut().invalid_input(message)?;
                    } else {
                        let message = self
                            .ui
                            .borrow()
                            .message(MsgId::PlayErrorChoiceIdUnavailable, [("id", id)]);
                        self.ui.borrow_mut().invalid_input(message)?;
                    }
                }
            }
        }
    }

    fn selected_choice(&self, choice_id: &ChoiceId) -> Result<(), CliError> {
        self.ui.borrow_mut().selected_choice(choice_id)
    }

    fn effect(&self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.ui.borrow_mut().effect(effect)
    }

    fn acknowledge(&self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.ui.borrow_mut().acknowledge(effect)
    }

    fn end(&self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        self.ui.borrow_mut().end(deferred_effects)
    }

    fn mark_interrupted(&self) {
        *self.interrupted.borrow_mut() = true;
    }

    fn was_interrupted(&self) -> bool {
        *self.interrupted.borrow()
    }

    fn set_ui_error(&self, error: CliError) {
        *self.ui_error.borrow_mut() = Some(error);
    }

    fn take_ui_error(&self) -> Option<CliError> {
        self.ui_error.borrow_mut().take()
    }

    fn resolve_runtime_error(&self, error: recite_runtime::DialogueError) -> CliError {
        if self.was_interrupted() {
            return CliError::PlayInterrupted;
        }
        self.take_ui_error().unwrap_or_else(|| error.into())
    }
}

impl<U: PlayUiAdapter> DialogueContext for InteractiveContext<'_, U> {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<bool, ConditionEvaluationError> {
        self.ui.borrow_mut().condition(query).map_err(|error| {
            if matches!(error, CliError::PlayInterrupted) {
                self.mark_interrupted();
            }
            let message = error.to_string();
            self.set_ui_error(error);
            ConditionEvaluationError::new(message)
        })
    }
}

trait PlayUiAdapter {
    fn message(&self, id: MsgId, args: impl IntoIterator<Item = (&'static str, String)>) -> String;
    fn start(&mut self, asset: &CompiledDialogue, block: &str) -> Result<(), CliError>;
    fn line(&mut self, line: &DialogueLine) -> Result<(), CliError>;
    fn choice(
        &mut self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<ChoiceSelection, CliError>;
    fn selected_choice(&mut self, choice_id: &ChoiceId) -> Result<(), CliError>;
    fn condition(&mut self, query: ConditionQuery<'_>) -> Result<bool, CliError>;
    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError>;
    fn acknowledge(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError>;
    fn end(&mut self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError>;
    fn invalid_input(&mut self, message: String) -> Result<(), CliError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ChoiceSelection {
    Index(usize),
    Id(String),
}

fn unavailable_choice_message<U: PlayUiAdapter>(
    ui: &RefCell<&mut U>,
    choice: &DialogueChoice,
) -> String {
    let ui = ui.borrow();
    match choice.unavailable_reason.as_deref() {
        Some(reason) if !reason.is_empty() => ui.message(
            MsgId::PlayErrorChoiceUnavailableReason,
            [
                ("id", choice.id.as_str().to_owned()),
                ("reason", reason.to_owned()),
            ],
        ),
        _ => ui.message(
            MsgId::PlayErrorChoiceUnavailable,
            [("id", choice.id.as_str().to_owned())],
        ),
    }
}

struct PlainPlayUi<'a, R: ?Sized, W: ?Sized> {
    input: &'a mut R,
    output: &'a mut W,
    messages: &'a Messages,
}

impl<'a, R: ?Sized, W: ?Sized> PlainPlayUi<'a, R, W> {
    fn new(input: &'a mut R, output: &'a mut W, messages: &'a Messages) -> Self {
        Self {
            input,
            output,
            messages,
        }
    }
}

impl<R: Read + ?Sized, W: Write + ?Sized> PlayUiAdapter for PlainPlayUi<'_, R, W> {
    fn message(&self, id: MsgId, args: impl IntoIterator<Item = (&'static str, String)>) -> String {
        self.messages.format(id, args)
    }

    fn start(&mut self, asset: &CompiledDialogue, block: &str) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlayStart,
                [
                    ("asset", asset.header.asset_id.as_str().to_owned()),
                    ("block", block.to_owned()),
                ],
            )
        )?;
        Ok(())
    }

    fn line(&mut self, line: &DialogueLine) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlayLine,
                [
                    ("id", line.id.as_str().to_owned()),
                    ("text", line.text.clone()),
                ],
            )
        )?;
        Ok(())
    }

    fn choice(
        &mut self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<ChoiceSelection, CliError> {
        if let Some(line) = line {
            writeln!(
                self.output,
                "{}",
                self.messages.format(
                    MsgId::PlayPromptLine,
                    [
                        ("id", line.id.as_str().to_owned()),
                        ("text", line.text.clone()),
                    ],
                )
            )?;
        } else {
            writeln!(self.output, "{}", self.messages.text(MsgId::PlayPrompt))?;
        }
        for (index, choice) in choices.iter().enumerate() {
            let availability = if choice.is_available {
                String::new()
            } else {
                self.messages.text(MsgId::PlayChoiceUnavailableSuffix)
            };
            writeln!(
                self.output,
                "{}",
                self.messages.format(
                    MsgId::PlayChoiceRow,
                    [
                        ("index", (index + 1).to_string()),
                        ("id", choice.id.as_str().to_owned()),
                        ("text", choice.text.clone()),
                        ("availability", availability),
                    ],
                )
            )?;
        }
        write!(
            self.output,
            "{} ",
            self.messages.text(MsgId::PlayChoicePrompt)
        )?;
        self.output.flush()?;
        let input = read_line(self.input, "choice selection")?;
        parse_choice_selection(input.trim(), self.messages)
    }

    fn condition(&mut self, query: ConditionQuery<'_>) -> Result<bool, CliError> {
        let query = condition_query_text(query);
        loop {
            write!(
                self.output,
                "{} ",
                self.messages
                    .format(MsgId::PlayConditionPrompt, [("query", query.clone())])
            )?;
            self.output.flush()?;
            let input = read_line(self.input, "condition answer")?;
            match input.trim().to_ascii_lowercase().as_str() {
                "y" | "yes" | "true" | "1" => {
                    writeln!(
                        self.output,
                        "{}",
                        self.messages.format(
                            MsgId::PlayConditionResult,
                            [("query", query.clone()), ("result", "true".to_owned())],
                        )
                    )?;
                    return Ok(true);
                }
                "n" | "no" | "false" | "0" => {
                    writeln!(
                        self.output,
                        "{}",
                        self.messages.format(
                            MsgId::PlayConditionResult,
                            [("query", query.clone()), ("result", "false".to_owned())],
                        )
                    )?;
                    return Ok(false);
                }
                _ => self.invalid_input(self.messages.text(MsgId::PlayErrorEnterYOrN))?,
            }
        }
    }

    fn selected_choice(&mut self, choice_id: &ChoiceId) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlaySelectedChoice,
                [("id", choice_id.as_str().to_owned())],
            )
        )?;
        Ok(())
    }

    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlayEffect,
                [
                    ("mode", effect.mode.to_string()),
                    ("id", effect.id.as_str().to_owned()),
                    ("function", effect.function.clone()),
                    ("args", format_effect_arguments(&effect.args)),
                ],
            )
        )?;
        Ok(())
    }

    fn acknowledge(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        loop {
            write!(
                self.output,
                "{} ",
                self.messages.format(
                    MsgId::PlayAckPrompt,
                    [("id", effect.id.as_str().to_owned())],
                )
            )?;
            self.output.flush()?;
            let input = read_line(self.input, "blocking effect acknowledgement")?;
            let input = input.trim();
            if input.is_empty() || input.eq_ignore_ascii_case("ack") {
                writeln!(
                    self.output,
                    "{}",
                    self.messages.format(
                        MsgId::PlayAckCompleted,
                        [("id", effect.id.as_str().to_owned())],
                    )
                )?;
                return Ok(());
            }
            self.invalid_input(self.messages.text(MsgId::PlayErrorPressEnterOrAck))?;
        }
    }

    fn end(&mut self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        writeln!(self.output, "{}", self.messages.text(MsgId::PlayEnd))?;
        if !deferred_effects.is_empty() {
            writeln!(
                self.output,
                "{}",
                self.messages.text(MsgId::PlayDeferredEffects)
            )?;
            for effect in deferred_effects {
                writeln!(
                    self.output,
                    "{}",
                    self.messages.format(
                        MsgId::PlayDeferredEffectRow,
                        [
                            ("function", effect.function.clone()),
                            ("args", format_effect_arguments(&effect.args)),
                        ],
                    )
                )?;
            }
        }
        Ok(())
    }

    fn invalid_input(&mut self, message: String) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages
                .format(MsgId::PlayInvalidInput, [("message", message)])
        )?;
        Ok(())
    }
}

fn read_line<R: Read + ?Sized>(input: &mut R, field: &'static str) -> Result<String, CliError> {
    let mut byte = [0_u8; 1];
    let mut line = Vec::new();
    loop {
        match input.read(&mut byte) {
            Ok(0) if line.is_empty() => return Err(CliError::PlayEof { field }),
            Ok(0) => break,
            Ok(_) if byte[0] == b'\n' => break,
            Ok(_) => line.push(byte[0]),
            Err(error) => return Err(CliError::Io(error)),
        }
    }
    Ok(String::from_utf8_lossy(&line).into_owned())
}

fn parse_choice_selection(input: &str, messages: &Messages) -> Result<ChoiceSelection, CliError> {
    if input.is_empty() {
        return Err(CliError::PlayInvalidInput(
            messages.text(MsgId::PlayErrorEmptyChoice),
        ));
    }
    if let Ok(index) = input.parse::<usize>() {
        return Ok(ChoiceSelection::Index(index));
    }
    Ok(ChoiceSelection::Id(input.to_owned()))
}

fn condition_query_text(query: ConditionQuery<'_>) -> String {
    format_condition_query(
        query.function(),
        query
            .arguments()
            .into_iter()
            .map(RuntimeDisplayArgument::from),
    )
}

fn run_tui_stdio(
    asset: &CompiledDialogue,
    block: &str,
    settings: TuiSettings,
    messages: Messages,
) -> Result<(), CliError> {
    let mut restore_guard = enter_terminal()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = TuiPlayUi::new(&mut terminal, settings, messages);
    let result = PlayDriver::new(asset, block).run(&mut ui);
    let restore_result = restore_terminal(&mut terminal);
    if restore_result.is_ok() {
        restore_guard.disarm();
    }
    match (result, restore_result) {
        (Err(CliError::PlayInterrupted), Ok(())) => Ok(()),
        (result, Ok(())) => result,
        (_, Err(error)) => Err(error),
    }
}

#[derive(Default)]
struct TuiState {
    asset: String,
    block: String,
    transcript: Vec<TuiTranscriptEntry>,
    prompt: TuiPrompt,
    status: String,
    key_hints: KeyHints,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TuiTranscriptEntry {
    kind: TuiTranscriptKind,
    id: Option<String>,
    text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TuiTranscriptKind {
    Line,
    Prompt,
    Choice,
    Condition,
    Effect,
    Ack,
    End,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
enum TuiPrompt {
    #[default]
    None,
    Choice {
        line: Option<TuiPromptLine>,
        choices: Vec<TuiChoiceRow>,
        selected: usize,
        mode: PromptMode,
        input: TextBuffer,
        command: TextBuffer,
        show_help: bool,
    },
    Condition {
        query: String,
        mode: PromptMode,
        input: TextBuffer,
        command: TextBuffer,
        show_help: bool,
    },
    Effect {
        mode: String,
        id: String,
        function: String,
        args: String,
        input_mode: PromptMode,
        input: TextBuffer,
        command: TextBuffer,
        show_help: bool,
    },
    Finished,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TuiPromptLine {
    id: String,
    text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TuiChoiceRow {
    index: usize,
    id: String,
    text: String,
    is_available: bool,
    unavailable_reason: Option<String>,
    is_visible: bool,
}

struct TuiPlayUi<'a, B: Backend> {
    terminal: &'a mut Terminal<B>,
    state: TuiState,
    settings: TuiSettings,
    messages: Messages,
}

impl<'a, B: Backend> TuiPlayUi<'a, B> {
    fn new(terminal: &'a mut Terminal<B>, settings: TuiSettings, messages: Messages) -> Self {
        let state = TuiState {
            key_hints: settings.key_hints,
            ..TuiState::default()
        };
        Self {
            terminal,
            state,
            settings,
            messages,
        }
    }

    fn push(
        &mut self,
        kind: TuiTranscriptKind,
        id: Option<String>,
        text: impl Into<String>,
    ) -> Result<(), CliError> {
        self.state.transcript.push(TuiTranscriptEntry {
            kind,
            id,
            text: text.into(),
        });
        self.render()
    }

    fn render(&mut self) -> Result<(), CliError> {
        let state = &self.state;
        let messages = &self.messages;
        self.terminal
            .draw(|frame| render_tui(frame, state, messages))?;
        Ok(())
    }

    fn read_intent(&mut self, mode: PromptMode) -> Result<TuiIntent, CliError> {
        loop {
            self.render()?;
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                return Ok(map_key(self.settings.keymap, mode, key));
            }
        }
    }

    fn wait_for_exit(&mut self) -> Result<(), CliError> {
        let mut command = TextBuffer::default();
        let mut command_mode = false;
        loop {
            self.render()?;
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
                {
                    return Ok(());
                }
                if command_mode {
                    match key.code {
                        KeyCode::Enter if command_quits(command.as_str()) => return Ok(()),
                        KeyCode::Esc => {
                            command_mode = false;
                            command.clear();
                            self.state.status = self.messages.text(MsgId::TuiFinished);
                        }
                        KeyCode::Char(ch) => {
                            command.insert(ch);
                            self.state.status = self.messages.format(
                                MsgId::TuiCommandWithValue,
                                [("command", command.as_str().to_owned())],
                            );
                        }
                        KeyCode::Backspace => {
                            command.backspace();
                            self.state.status = self.messages.format(
                                MsgId::TuiCommandWithValue,
                                [("command", command.as_str().to_owned())],
                            );
                        }
                        _ => {}
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char(':') => {
                        command_mode = true;
                        command.clear();
                        self.state.status = self.messages.text(MsgId::TuiCommand);
                    }
                    _ => {}
                }
            }
        }
    }

    fn read_command(&mut self) -> Result<bool, CliError> {
        let previous_prompt = self.state.prompt.clone();
        set_prompt_mode(&mut self.state.prompt, PromptMode::Command);
        set_command(&mut self.state.prompt, TextBuffer::default());
        self.state.status = self.messages.text(MsgId::TuiCommand);
        loop {
            match self.read_intent(PromptMode::Command)? {
                TuiIntent::Submit => {
                    let command = prompt_command(&self.state.prompt).to_owned();
                    self.state.prompt = previous_prompt;
                    if command_quits(&command) {
                        return Ok(true);
                    }
                    self.state.status = self
                        .messages
                        .format(MsgId::TuiUnknownCommand, [("command", command)]);
                    return Ok(false);
                }
                TuiIntent::Quit => return Err(CliError::PlayInterrupted),
                TuiIntent::Cancel => {
                    self.state.prompt = previous_prompt;
                    return Ok(false);
                }
                intent => {
                    mutate_prompt_command(&mut self.state.prompt, intent);
                    self.state.status = self.messages.format(
                        MsgId::TuiCommandWithValue,
                        [("command", prompt_command(&self.state.prompt).to_owned())],
                    );
                }
            }
        }
    }

    fn handle_global_prompt_intent(&mut self, intent: TuiIntent) -> Result<bool, CliError> {
        match intent {
            TuiIntent::Quit => Err(CliError::PlayInterrupted),
            TuiIntent::OpenCommand => self.read_command(),
            TuiIntent::ToggleHelp => {
                toggle_help(&mut self.state.prompt);
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn read_choice_selection(&mut self) -> Result<ChoiceSelection, CliError> {
        loop {
            let mode = prompt_mode(&self.state.prompt);
            let intent = self.read_intent(mode)?;
            if self.handle_global_prompt_intent(intent)? {
                return Err(CliError::PlayInterrupted);
            }
            match intent {
                TuiIntent::Submit => {
                    let input = prompt_input(&self.state.prompt).trim().to_owned();
                    if !input.is_empty() {
                        return parse_choice_selection(&input, &self.messages);
                    }
                    if let Some(id) = selected_choice_id(&self.state.prompt) {
                        return Ok(ChoiceSelection::Id(id.to_owned()));
                    }
                    return Err(CliError::PlayInvalidInput(
                        self.messages.text(MsgId::PlayErrorEmptyChoice),
                    ));
                }
                TuiIntent::MoveNext => {
                    move_choice_selection(&mut self.state.prompt, 1);
                    self.state.status = choice_status(&self.messages, self.settings.keymap);
                }
                TuiIntent::MovePrevious => {
                    move_choice_selection(&mut self.state.prompt, -1);
                    self.state.status = choice_status(&self.messages, self.settings.keymap);
                }
                TuiIntent::StartInsert => {
                    set_prompt_mode(&mut self.state.prompt, PromptMode::Insert);
                    self.state.status =
                        prompt_label(self.messages.text(MsgId::TuiChoiceInputPrefix));
                }
                TuiIntent::Cancel => {
                    if self.settings.keymap == Keymap::Vim && mode == PromptMode::Insert {
                        set_prompt_mode(&mut self.state.prompt, PromptMode::Normal);
                        self.state.status = choice_status(&self.messages, self.settings.keymap);
                    }
                }
                intent => {
                    mutate_prompt_input(&mut self.state.prompt, intent);
                    let input = prompt_input(&self.state.prompt);
                    if input.is_empty() {
                        self.state.status = choice_status(&self.messages, self.settings.keymap);
                    } else {
                        self.state.status = self
                            .messages
                            .format(MsgId::TuiChoiceInput, [("input", input.to_owned())]);
                    }
                }
            }
        }
    }

    fn read_text_prompt(&mut self, label: impl AsRef<str>) -> Result<String, CliError> {
        clear_prompt_input(&mut self.state.prompt);
        let label = label.as_ref();
        loop {
            let mode = prompt_mode(&self.state.prompt);
            let intent = self.read_intent(mode)?;
            if self.handle_global_prompt_intent(intent)? {
                return Err(CliError::PlayInterrupted);
            }
            match intent {
                TuiIntent::Submit => {
                    let input = prompt_input(&self.state.prompt).to_owned();
                    clear_prompt_input(&mut self.state.prompt);
                    return Ok(input);
                }
                TuiIntent::Cancel => {
                    if self.settings.keymap == Keymap::Vim && mode == PromptMode::Insert {
                        set_prompt_mode(&mut self.state.prompt, PromptMode::Normal);
                        self.state.status = self.messages.text(MsgId::TuiNormalMode);
                    }
                }
                TuiIntent::StartInsert => {
                    set_prompt_mode(&mut self.state.prompt, PromptMode::Insert);
                }
                intent => {
                    mutate_prompt_input(&mut self.state.prompt, intent);
                    self.state.status = format!("{label}{}", prompt_input(&self.state.prompt));
                }
            }
        }
    }
}

fn initial_prompt_mode(keymap: Keymap) -> PromptMode {
    match keymap {
        Keymap::Standard => PromptMode::Insert,
        Keymap::Vim => PromptMode::Normal,
    }
}

fn choice_status(messages: &Messages, keymap: Keymap) -> String {
    match keymap {
        Keymap::Standard => messages.text(MsgId::TuiChoiceStatusStandard),
        Keymap::Vim => messages.text(MsgId::TuiChoiceStatusVim),
    }
}

fn initial_choice_selection(choices: &[TuiChoiceRow]) -> usize {
    choices
        .iter()
        .position(|choice| choice.is_visible && choice.is_available)
        .or_else(|| choices.iter().position(|choice| choice.is_visible))
        .unwrap_or(0)
}

fn prompt_mode(prompt: &TuiPrompt) -> PromptMode {
    match prompt {
        TuiPrompt::Choice {
            mode, show_help, ..
        }
        | TuiPrompt::Condition {
            mode, show_help, ..
        } => {
            if *show_help {
                PromptMode::Help
            } else {
                *mode
            }
        }
        TuiPrompt::Effect {
            input_mode,
            show_help,
            ..
        } => {
            if *show_help {
                PromptMode::Help
            } else {
                *input_mode
            }
        }
        _ => PromptMode::Normal,
    }
}

fn set_prompt_mode(prompt: &mut TuiPrompt, mode: PromptMode) {
    match prompt {
        TuiPrompt::Choice {
            mode: prompt_mode,
            show_help,
            ..
        }
        | TuiPrompt::Condition {
            mode: prompt_mode,
            show_help,
            ..
        } => {
            *prompt_mode = mode;
            *show_help = false;
        }
        TuiPrompt::Effect {
            input_mode,
            show_help,
            ..
        } => {
            *input_mode = mode;
            *show_help = false;
        }
        _ => {}
    }
}

fn toggle_help(prompt: &mut TuiPrompt) {
    match prompt {
        TuiPrompt::Choice { show_help, .. }
        | TuiPrompt::Condition { show_help, .. }
        | TuiPrompt::Effect { show_help, .. } => *show_help = !*show_help,
        _ => {}
    }
}

fn set_command(prompt: &mut TuiPrompt, command: TextBuffer) {
    match prompt {
        TuiPrompt::Choice {
            command: prompt_command,
            ..
        }
        | TuiPrompt::Condition {
            command: prompt_command,
            ..
        }
        | TuiPrompt::Effect {
            command: prompt_command,
            ..
        } => *prompt_command = command,
        _ => {}
    }
}

fn prompt_command(prompt: &TuiPrompt) -> &str {
    match prompt {
        TuiPrompt::Choice { command, .. }
        | TuiPrompt::Condition { command, .. }
        | TuiPrompt::Effect { command, .. } => command.as_str(),
        _ => "",
    }
}

fn mutate_prompt_command(prompt: &mut TuiPrompt, intent: TuiIntent) {
    match prompt {
        TuiPrompt::Choice { command, .. }
        | TuiPrompt::Condition { command, .. }
        | TuiPrompt::Effect { command, .. } => mutate_buffer(command, intent),
        _ => {}
    }
}

fn prompt_input(prompt: &TuiPrompt) -> &str {
    match prompt {
        TuiPrompt::Choice { input, .. }
        | TuiPrompt::Condition { input, .. }
        | TuiPrompt::Effect { input, .. } => input.as_str(),
        _ => "",
    }
}

fn clear_prompt_input(prompt: &mut TuiPrompt) {
    match prompt {
        TuiPrompt::Choice { input, .. }
        | TuiPrompt::Condition { input, .. }
        | TuiPrompt::Effect { input, .. } => input.clear(),
        _ => {}
    }
}

fn mutate_prompt_input(prompt: &mut TuiPrompt, intent: TuiIntent) {
    match prompt {
        TuiPrompt::Choice { input, mode, .. } => {
            if matches!(intent, TuiIntent::Text(_)) {
                *mode = PromptMode::Insert;
            }
            mutate_buffer(input, intent);
        }
        TuiPrompt::Condition { input, .. } | TuiPrompt::Effect { input, .. } => {
            mutate_buffer(input, intent);
        }
        _ => {}
    }
}

fn mutate_buffer(buffer: &mut TextBuffer, intent: TuiIntent) {
    match intent {
        TuiIntent::Text(ch) => buffer.insert(ch),
        TuiIntent::Backspace => buffer.backspace(),
        TuiIntent::Delete => buffer.delete(),
        TuiIntent::MoveCursorLeft => buffer.move_left(),
        TuiIntent::MoveCursorRight => buffer.move_right(),
        TuiIntent::MoveCursorStart => buffer.move_start(),
        TuiIntent::MoveCursorEnd => buffer.move_end(),
        TuiIntent::ClearLine => buffer.clear(),
        TuiIntent::DeleteWord => buffer.delete_word_before_cursor(),
        _ => {}
    }
}

fn selected_choice_id(prompt: &TuiPrompt) -> Option<&str> {
    match prompt {
        TuiPrompt::Choice {
            choices, selected, ..
        } => choices.get(*selected).map(|choice| choice.id.as_str()),
        _ => None,
    }
}

fn move_choice_selection(prompt: &mut TuiPrompt, direction: isize) {
    let TuiPrompt::Choice {
        choices, selected, ..
    } = prompt
    else {
        return;
    };
    if choices.is_empty() {
        return;
    }
    let len = choices.len();
    let mut next = *selected;
    for _ in 0..len {
        next = if direction > 0 {
            (next + 1) % len
        } else {
            (next + len - 1) % len
        };
        if choices[next].is_visible && choices[next].is_available {
            *selected = next;
            return;
        }
    }
}

impl<B: Backend> PlayUiAdapter for TuiPlayUi<'_, B> {
    fn message(&self, id: MsgId, args: impl IntoIterator<Item = (&'static str, String)>) -> String {
        self.messages.format(id, args)
    }

    fn start(&mut self, asset: &CompiledDialogue, block: &str) -> Result<(), CliError> {
        self.state.asset = asset.header.asset_id.as_str().to_owned();
        self.state.block = block.to_owned();
        self.state.status = self.messages.text(MsgId::TuiReady);
        self.render()
    }

    fn line(&mut self, line: &DialogueLine) -> Result<(), CliError> {
        self.state.prompt = TuiPrompt::None;
        self.push(
            TuiTranscriptKind::Line,
            Some(line.id.as_str().to_owned()),
            line.text.clone(),
        )
    }

    fn choice(
        &mut self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<ChoiceSelection, CliError> {
        let rows = choices
            .iter()
            .enumerate()
            .map(|(index, choice)| TuiChoiceRow {
                index: index + 1,
                id: choice.id.as_str().to_owned(),
                text: choice.text.clone(),
                is_available: choice.is_available,
                unavailable_reason: choice.unavailable_reason.clone(),
                is_visible: self.settings.show_unavailable_choices || choice.is_available,
            })
            .collect::<Vec<_>>();
        let selected = initial_choice_selection(&rows);
        self.state.prompt = TuiPrompt::Choice {
            line: line.map(|line| TuiPromptLine {
                id: line.id.as_str().to_owned(),
                text: line.text.clone(),
            }),
            choices: rows,
            selected,
            mode: initial_prompt_mode(self.settings.keymap),
            input: TextBuffer::default(),
            command: TextBuffer::default(),
            show_help: false,
        };
        self.state.status = choice_status(&self.messages, self.settings.keymap);
        self.read_choice_selection()
    }

    fn selected_choice(&mut self, choice_id: &ChoiceId) -> Result<(), CliError> {
        if let TuiPrompt::Choice {
            line: Some(line), ..
        } = &self.state.prompt
        {
            self.state.transcript.push(TuiTranscriptEntry {
                kind: TuiTranscriptKind::Prompt,
                id: Some(line.id.clone()),
                text: line.text.clone(),
            });
        }
        self.push(
            TuiTranscriptKind::Choice,
            Some(choice_id.as_str().to_owned()),
            self.messages.text(MsgId::TuiTranscriptSelected),
        )
    }

    fn condition(&mut self, query: ConditionQuery<'_>) -> Result<bool, CliError> {
        let query = condition_query_text(query);
        self.state.prompt = TuiPrompt::Condition {
            query: query.clone(),
            mode: PromptMode::Insert,
            input: TextBuffer::default(),
            command: TextBuffer::default(),
            show_help: false,
        };
        loop {
            let label = prompt_label(self.messages.text(MsgId::TuiConditionInputPrefix));
            self.state.status = label.clone();
            let input = self.read_text_prompt(label)?;
            match input.trim().to_ascii_lowercase().as_str() {
                "y" | "yes" | "true" | "1" => {
                    self.push(TuiTranscriptKind::Condition, Some(query.clone()), "true")?;
                    return Ok(true);
                }
                "n" | "no" | "false" | "0" => {
                    self.push(TuiTranscriptKind::Condition, Some(query.clone()), "false")?;
                    return Ok(false);
                }
                _ => self.invalid_input(self.messages.text(MsgId::PlayErrorEnterYOrN))?,
            }
        }
    }

    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        let args = format_effect_arguments(&effect.args);
        self.state.prompt = TuiPrompt::Effect {
            mode: effect.mode.to_string(),
            id: effect.id.as_str().to_owned(),
            function: effect.function.clone(),
            args: args.clone(),
            input_mode: PromptMode::Insert,
            input: TextBuffer::default(),
            command: TextBuffer::default(),
            show_help: false,
        };
        self.push(
            TuiTranscriptKind::Effect,
            Some(effect.id.as_str().to_owned()),
            format!("{} {} {}", effect.mode, effect.function, args),
        )
    }

    fn acknowledge(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        loop {
            self.state.status = self
                .messages
                .format(MsgId::TuiAckStatus, [("id", effect.id.as_str().to_owned())]);
            let input =
                self.read_text_prompt(prompt_label(self.messages.text(MsgId::TuiAckInputPrefix)))?;
            if input.trim().is_empty() || input.trim().eq_ignore_ascii_case("ack") {
                self.push(
                    TuiTranscriptKind::Ack,
                    Some(effect.id.as_str().to_owned()),
                    self.messages.text(MsgId::TuiTranscriptCompleted),
                )?;
                return Ok(());
            }
            self.invalid_input(self.messages.text(MsgId::PlayErrorPressEnterOrAck))?;
        }
    }

    fn end(&mut self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        self.state.prompt = TuiPrompt::Finished;
        self.push(
            TuiTranscriptKind::End,
            None,
            self.messages.text(MsgId::PlayEnd),
        )?;
        if !deferred_effects.is_empty() {
            self.state.transcript.push(TuiTranscriptEntry {
                kind: TuiTranscriptKind::Effect,
                id: None,
                text: self.messages.text(MsgId::TuiTranscriptDeferredEffects),
            });
            for effect in deferred_effects {
                self.state.transcript.push(TuiTranscriptEntry {
                    kind: TuiTranscriptKind::Effect,
                    id: Some(effect.id.as_str().to_owned()),
                    text: format!(
                        "{} {}",
                        effect.function,
                        format_effect_arguments(&effect.args)
                    ),
                });
            }
            self.render()?;
        }
        self.state.status = self.messages.text(MsgId::TuiFinished);
        self.wait_for_exit()
    }

    fn invalid_input(&mut self, message: String) -> Result<(), CliError> {
        self.state.status = self
            .messages
            .format(MsgId::PlayInvalidInput, [("message", message)]);
        self.render()
    }
}

fn render_tui(frame: &mut ratatui::Frame<'_>, state: &TuiState, messages: &Messages) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(prompt_height(&state.prompt)),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let header = Line::from(vec![
        Span::styled(
            messages.text(MsgId::TuiHeaderTitle),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} ", messages.text(MsgId::TuiHeaderAsset)),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(state.asset.as_str()),
        Span::raw("  "),
        Span::styled(
            format!("{} ", messages.text(MsgId::TuiHeaderBlock)),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            state.block.as_str(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(vec![header, Line::from("")]), chunks[0]);

    let visible_transcript = state
        .transcript
        .iter()
        .rev()
        .take(chunks[1].height as usize)
        .rev()
        .collect::<Vec<_>>();
    let id_width = transcript_id_width(&visible_transcript);
    let transcript = visible_transcript
        .iter()
        .map(|entry| render_transcript_entry(entry, id_width, messages))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(transcript).wrap(Wrap { trim: false }),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(render_prompt(&state.prompt, messages)).wrap(Wrap { trim: false }),
        chunks[2],
    );

    frame.render_widget(Paragraph::new(render_footer(state, messages)), chunks[3]);
}

fn prompt_height(prompt: &TuiPrompt) -> u16 {
    match prompt {
        TuiPrompt::None | TuiPrompt::Finished => 2,
        TuiPrompt::Choice { choices, .. } => {
            let visible = choices.iter().filter(|choice| choice.is_visible).count();
            (visible as u16 + 5).clamp(5, 12)
        }
        TuiPrompt::Condition { show_help, .. } => {
            if *show_help {
                6
            } else {
                4
            }
        }
        TuiPrompt::Effect { show_help, .. } => {
            if *show_help {
                9
            } else {
                7
            }
        }
    }
}

fn transcript_id_width(entries: &[&TuiTranscriptEntry]) -> usize {
    entries
        .iter()
        .filter_map(|entry| entry.id.as_ref())
        .map(|id| id.chars().count().min(32))
        .max()
        .unwrap_or(12)
        .clamp(12, 32)
}

fn render_transcript_entry<'a>(
    entry: &'a TuiTranscriptEntry,
    id_width: usize,
    messages: &'a Messages,
) -> Line<'a> {
    let (label, color) = match entry.kind {
        TuiTranscriptKind::Line => (messages.text(MsgId::TuiTranscriptLine), Color::Green),
        TuiTranscriptKind::Prompt => (messages.text(MsgId::TuiTranscriptPrompt), Color::Blue),
        TuiTranscriptKind::Choice => (messages.text(MsgId::TuiTranscriptChoice), Color::Cyan),
        TuiTranscriptKind::Condition => {
            (messages.text(MsgId::TuiTranscriptCondition), Color::Yellow)
        }
        TuiTranscriptKind::Effect => (messages.text(MsgId::TuiTranscriptEffect), Color::Magenta),
        TuiTranscriptKind::Ack => (messages.text(MsgId::TuiTranscriptAck), Color::Magenta),
        TuiTranscriptKind::End => (messages.text(MsgId::TuiTranscriptEnd), Color::DarkGray),
    };
    let mut spans = vec![Span::styled(
        format!("{label:<9}"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )];
    let id = entry
        .id
        .as_deref()
        .map(|id| clamp_display(id, id_width))
        .unwrap_or_else(|| String::from(""));
    spans.push(Span::styled(
        format!("{id:<id_width$}"),
        Style::default().fg(Color::DarkGray),
    ));
    spans.push(Span::raw("  "));
    spans.push(Span::raw(entry.text.as_str()));
    Line::from(spans)
}

fn clamp_display(value: &str, max_width: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_width {
        return value.to_owned();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let prefix = value.chars().take(max_width - 3).collect::<String>();
    format!("{prefix}...")
}

fn render_prompt<'a>(prompt: &'a TuiPrompt, messages: &'a Messages) -> Vec<Line<'a>> {
    match prompt {
        TuiPrompt::None => vec![Line::from(Span::styled(
            messages.text(MsgId::TuiWaiting),
            Style::default().fg(Color::DarkGray),
        ))],
        TuiPrompt::Finished => vec![Line::from("")],
        TuiPrompt::Condition {
            query,
            input,
            command,
            show_help,
            ..
        } => {
            let mut lines = vec![
                Line::from(Span::styled(
                    messages.text(MsgId::TuiConditionTitle),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled(query.as_str(), Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::raw("y/n"),
                ]),
                input_line(messages.text(MsgId::TuiInputAnswer), input, command),
            ];
            if *show_help {
                lines.extend(help_lines("condition", messages));
            }
            lines
        }
        TuiPrompt::Effect {
            mode,
            id,
            function,
            args,
            input,
            command,
            show_help,
            ..
        } => {
            let mut lines = vec![
                Line::from(Span::styled(
                    messages.text(MsgId::TuiEffectTitle),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )),
                metadata_line(messages.text(MsgId::TuiMetadataMode), mode),
                metadata_line(messages.text(MsgId::TuiMetadataRuntimeEffectId), id),
                metadata_line(messages.text(MsgId::TuiMetadataFunction), function),
                metadata_line(messages.text(MsgId::TuiMetadataArgs), args),
                input_line(messages.text(MsgId::TuiInputAck), input, command),
            ];
            if *show_help {
                lines.extend(help_lines("effect", messages));
            }
            lines
        }
        TuiPrompt::Choice {
            line,
            choices,
            selected,
            input,
            command,
            show_help,
            ..
        } => {
            let mut lines = Vec::new();
            if let Some(line) = line {
                lines.push(Line::from(vec![
                    Span::styled(line.id.as_str(), Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(
                        line.text.as_str(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    messages.text(MsgId::TuiChoiceTitle),
                    Style::default().add_modifier(Modifier::BOLD),
                )));
            }
            let selected_index = choices.get(*selected).map(|choice| choice.index);
            for choice in choices.iter().filter(|choice| choice.is_visible) {
                let style = if choice.is_available {
                    Style::default()
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let is_selected = Some(choice.index) == selected_index;
                let marker = if is_selected { ">" } else { " " };
                let selected_style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let suffix = choice
                    .unavailable_reason
                    .as_deref()
                    .filter(|reason| !reason.is_empty())
                    .map(|reason| {
                        messages.format(
                            MsgId::TuiChoiceUnavailableReason,
                            [("reason", reason.to_owned())],
                        )
                    })
                    .unwrap_or_else(|| {
                        if choice.is_available {
                            String::new()
                        } else {
                            messages.text(MsgId::TuiChoiceUnavailable)
                        }
                    });
                lines.push(Line::from(vec![
                    Span::styled(marker, selected_style),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:>2}", choice.index),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<16}", choice.id),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(choice.text.as_str(), style),
                    Span::styled(suffix, Style::default().fg(Color::DarkGray)),
                ]));
            }
            lines.push(input_line(
                messages.text(MsgId::TuiInputChoice),
                input,
                command,
            ));
            if *show_help {
                lines.extend(help_lines("choice", messages));
            }
            lines
        }
    }
}

fn input_line<'a>(label: String, input: &'a TextBuffer, command: &'a TextBuffer) -> Line<'a> {
    if !command.is_empty() {
        return Line::from(vec![
            Span::styled(":", Style::default().fg(Color::DarkGray)),
            Span::raw(command.as_str()),
        ]);
    }
    Line::from(vec![
        Span::styled(format!("{label:<8}"), Style::default().fg(Color::DarkGray)),
        Span::raw(input.as_str()),
    ])
}

fn prompt_label(label: String) -> String {
    format!("{label} ")
}

fn help_lines(context: &str, messages: &Messages) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                messages.text(MsgId::TuiHelpLabel),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(" "),
            Span::raw(match context {
                "choice" => messages.text(MsgId::TuiHelpChoice),
                "condition" => messages.text(MsgId::TuiHelpCondition),
                "effect" => messages.text(MsgId::TuiHelpEffect),
                _ => messages.text(MsgId::TuiHelpDefault),
            }),
        ]),
    ]
}

fn metadata_line<'a>(label: String, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<18}"), Style::default().fg(Color::DarkGray)),
        Span::raw(value),
    ])
}

fn render_footer<'a>(state: &'a TuiState, messages: &'a Messages) -> Line<'a> {
    let help = match state.key_hints {
        KeyHints::Hidden => String::new(),
        KeyHints::Compact => match state.prompt {
            TuiPrompt::Choice { .. } => messages.text(MsgId::TuiFooterCompactChoice),
            TuiPrompt::Condition { .. } => messages.text(MsgId::TuiFooterCompactCondition),
            TuiPrompt::Effect { .. } => messages.text(MsgId::TuiFooterCompactEffect),
            TuiPrompt::Finished => messages.text(MsgId::TuiFooterCompactFinished),
            TuiPrompt::None => String::new(),
        },
        KeyHints::Contextual => match state.prompt {
            TuiPrompt::Choice { mode, .. } => match mode {
                PromptMode::Normal => messages.text(MsgId::TuiFooterChoiceNormal),
                PromptMode::Insert => messages.text(MsgId::TuiFooterChoiceInsert),
                PromptMode::Command => messages.text(MsgId::TuiFooterCommand),
                PromptMode::Help => messages.text(MsgId::TuiFooterHelp),
            },
            TuiPrompt::Condition { mode, .. } => match mode {
                PromptMode::Command => messages.text(MsgId::TuiFooterCommand),
                PromptMode::Help => messages.text(MsgId::TuiFooterHelp),
                _ => messages.text(MsgId::TuiFooterCondition),
            },
            TuiPrompt::Effect { input_mode, .. } => match input_mode {
                PromptMode::Command => messages.text(MsgId::TuiFooterCommand),
                PromptMode::Help => messages.text(MsgId::TuiFooterHelp),
                _ => messages.text(MsgId::TuiFooterEffect),
            },
            TuiPrompt::Finished => messages.text(MsgId::TuiFooterFinished),
            TuiPrompt::None => String::new(),
        },
    };
    if help.is_empty() {
        return Line::from(Span::styled(
            state.status.as_str(),
            Style::default().fg(Color::DarkGray),
        ));
    }
    Line::from(vec![
        Span::styled(state.status.as_str(), Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::raw(help),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use recite_compiler::{CompileInput, compile_inputs};

    use crate::fs::compile_options;

    fn asset(source: &str) -> CompiledDialogue {
        let report = compile_inputs(
            vec![CompileInput::new("test.recite", source)],
            compile_options(Path::new("test.recitec"), None).expect("options"),
        )
        .expect("compiles");
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        report.asset.expect("asset").dialogue
    }

    fn run_plain(asset: &CompiledDialogue, input: &str) -> Result<String, CliError> {
        let mut input = input.as_bytes();
        let mut output = Vec::new();
        let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
        let mut ui = PlainPlayUi::new(&mut input, &mut output, &messages);
        PlayDriver::new(asset, "start").run(&mut ui)?;
        Ok(String::from_utf8(output).expect("utf8"))
    }

    #[test]
    fn plain_play_selects_choice_by_index_answers_condition_and_acknowledges_blocking_effect() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help if trusts(player)\n",
            "    Help.\n",
            "    -> help\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: help\n",
            "! blocking grant_item(map)\n",
            "> helped\n",
            "  Helped.\n",
            "! deferred finish(help)\n",
            "-> END\n",
        ));

        let output = run_plain(&asset, "y\n1\nack\n").expect("play succeeds");

        assert!(output.contains("condition trusts(player) = true"));
        assert!(output.contains("selected choice help"));
        assert!(output.contains("effect blocking"));
        assert!(output.contains("acknowledged effect"));
        assert!(output.contains("line helped: Helped."));
        assert!(output.contains("deferred effects:"));
    }

    #[test]
    fn plain_play_selects_choice_by_id() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help\n",
            "    Help.\n",
            "    -> help\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: help\n",
            "> helped\n",
            "  Helped.\n",
            "-> END\n",
        ));

        let output = run_plain(&asset, "help\n").expect("play succeeds");

        assert!(output.contains("selected choice help"));
        assert!(output.contains("line helped: Helped."));
    }

    #[test]
    fn plain_play_can_select_numeric_choice_id() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? skip\n",
            "    Skip.\n",
            "    -> skip\n",
            "  ? 2\n",
            "    Numeric.\n",
            "    -> numeric\n",
            ":: skip\n",
            "> skipped\n",
            "  Skipped.\n",
            "-> END\n",
            ":: numeric\n",
            "> numeric_line\n",
            "  Numeric ID selected.\n",
            "-> END\n",
        ));

        let output = run_plain(&asset, "2\n").expect("play succeeds");

        assert!(output.contains("selected choice 2"));
        assert!(output.contains("line numeric_line: Numeric ID selected."));
        assert!(!output.contains("selected choice skip"));
    }

    #[test]
    fn plain_play_reprompts_after_invalid_choice_and_condition_input() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help if trusts(player)\n",
            "    Help.\n",
            "    -> END\n",
        ));

        let output = run_plain(&asset, "maybe\ny\n\nbad id\n99\n1\n").expect("play succeeds");

        assert!(output.contains("invalid input: enter y or n"));
        assert!(output.contains("invalid input: choice selection cannot be empty"));
        assert!(output.contains("invalid input: choice ID `bad id` is not available here"));
        assert!(output.contains("invalid input: choice index 99 is out of range"));
        assert!(output.contains("selected choice help"));
    }

    #[test]
    fn plain_play_reprompts_for_unavailable_choice_without_recording_selection() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help if trusts(player)\n",
            "    Help.\n",
            "    -> help\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> leave\n",
            ":: help\n",
            "> helped\n",
            "  Helped.\n",
            "-> END\n",
            ":: leave\n",
            "> left\n",
            "  Left.\n",
            "-> END\n",
        ));

        let output = run_plain(&asset, "n\n1\nleave\n").expect("play succeeds");

        assert!(output.contains("condition trusts(player) = false"));
        assert!(output.contains("invalid input: choice `help` is unavailable"));
        assert!(!output.contains("selected choice help"));
        assert!(output.contains("selected choice leave"));
        assert!(output.contains("line left: Left."));
    }

    #[test]
    fn plain_play_reports_eof() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help\n",
            "    Help.\n",
            "    -> END\n",
        ));

        let error = run_plain(&asset, "").expect_err("eof fails");

        assert!(error.to_string().contains("reached EOF"));
    }

    #[test]
    fn plain_play_reports_condition_prompt_eof_as_cli_error() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help if trusts(player)\n",
            "    Help.\n",
            "    -> END\n",
        ));

        let error = run_plain(&asset, "").expect_err("eof fails");

        assert!(matches!(
            error,
            CliError::PlayEof {
                field: "condition answer"
            }
        ));
    }

    #[test]
    fn plain_play_reports_post_choice_condition_eof_as_cli_error() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help\n",
            "    Help.\n",
            "    -> help\n",
            ":: help\n",
            ":if trusts(player)\n",
            "  > helped\n",
            "    Helped.\n",
            "-> END\n",
        ));

        let error = run_plain(&asset, "help\n").expect_err("eof fails");

        assert!(matches!(
            error,
            CliError::PlayEof {
                field: "condition answer"
            }
        ));
    }

    #[test]
    fn choice_selection_prefers_first_visible_available_choice() {
        let choices = vec![
            TuiChoiceRow {
                index: 1,
                id: "locked".to_owned(),
                text: "Locked.".to_owned(),
                is_available: false,
                unavailable_reason: Some("missing key".to_owned()),
                is_visible: true,
            },
            TuiChoiceRow {
                index: 2,
                id: "open".to_owned(),
                text: "Open.".to_owned(),
                is_available: true,
                unavailable_reason: None,
                is_visible: true,
            },
        ];

        assert_eq!(initial_choice_selection(&choices), 1);
    }

    #[test]
    fn choice_navigation_skips_hidden_and_unavailable_choices() {
        let mut prompt = TuiPrompt::Choice {
            line: None,
            choices: vec![
                TuiChoiceRow {
                    index: 1,
                    id: "first".to_owned(),
                    text: "First.".to_owned(),
                    is_available: true,
                    unavailable_reason: None,
                    is_visible: true,
                },
                TuiChoiceRow {
                    index: 2,
                    id: "locked".to_owned(),
                    text: "Locked.".to_owned(),
                    is_available: false,
                    unavailable_reason: None,
                    is_visible: true,
                },
                TuiChoiceRow {
                    index: 3,
                    id: "hidden".to_owned(),
                    text: "Hidden.".to_owned(),
                    is_available: true,
                    unavailable_reason: None,
                    is_visible: false,
                },
                TuiChoiceRow {
                    index: 4,
                    id: "last".to_owned(),
                    text: "Last.".to_owned(),
                    is_available: true,
                    unavailable_reason: None,
                    is_visible: true,
                },
            ],
            selected: 0,
            mode: PromptMode::Insert,
            input: TextBuffer::default(),
            command: TextBuffer::default(),
            show_help: false,
        };

        move_choice_selection(&mut prompt, 1);
        assert_eq!(selected_choice_id(&prompt), Some("last"));
        move_choice_selection(&mut prompt, 1);
        assert_eq!(selected_choice_id(&prompt), Some("first"));
    }

    #[test]
    fn transcript_ids_are_aligned_and_clamped() {
        let entries = [
            TuiTranscriptEntry {
                kind: TuiTranscriptKind::Line,
                id: Some("short".to_owned()),
                text: "Line.".to_owned(),
            },
            TuiTranscriptEntry {
                kind: TuiTranscriptKind::Effect,
                id: Some("effect:very-long-source-location:123:45#9".to_owned()),
                text: "blocking grant_item (map)".to_owned(),
            },
        ];
        let visible = entries.iter().collect::<Vec<_>>();
        let width = transcript_id_width(&visible);

        assert_eq!(width, 32);
        let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
        assert!(
            format!(
                "{:?}",
                render_transcript_entry(&entries[1], width, &messages)
            )
            .contains("...")
        );
    }

    #[test]
    fn tui_render_includes_header_and_choice_prompt() {
        let state = TuiState {
            asset: "asset".to_owned(),
            block: "start".to_owned(),
            transcript: vec![TuiTranscriptEntry {
                kind: TuiTranscriptKind::Line,
                id: Some("intro".to_owned()),
                text: "Welcome.".to_owned(),
            }],
            prompt: TuiPrompt::Choice {
                line: Some(TuiPromptLine {
                    id: "intro".to_owned(),
                    text: "Welcome.".to_owned(),
                }),
                choices: vec![TuiChoiceRow {
                    index: 1,
                    id: "help".to_owned(),
                    text: "Help.".to_owned(),
                    is_available: true,
                    unavailable_reason: None,
                    is_visible: true,
                }],
                selected: 0,
                mode: PromptMode::Insert,
                input: TextBuffer::default(),
                command: TextBuffer::default(),
                show_help: false,
            },
            status: "choice> ".to_owned(),
            key_hints: KeyHints::Contextual,
        };
        let content = render_tui_content(&state, 80, 20);

        assert!(content.contains("recite play"));
        assert!(content.contains("asset"));
        assert!(content.contains("block"));
        assert!(content.contains("intro"));
        assert!(content.contains("Welcome."));
        assert!(content.contains("help"));
        assert!(content.contains("Help."));
        assert!(content.contains("Type choice ID/index"));
    }

    #[test]
    fn tui_render_finished_state_without_inactive_prompt_filler() {
        let state = TuiState {
            asset: "asset".to_owned(),
            block: "start".to_owned(),
            transcript: vec![
                TuiTranscriptEntry {
                    kind: TuiTranscriptKind::Choice,
                    id: Some("help".to_owned()),
                    text: "selected".to_owned(),
                },
                TuiTranscriptEntry {
                    kind: TuiTranscriptKind::Line,
                    id: Some("helped".to_owned()),
                    text: "Helped.".to_owned(),
                },
                TuiTranscriptEntry {
                    kind: TuiTranscriptKind::End,
                    id: None,
                    text: "end".to_owned(),
                },
            ],
            prompt: TuiPrompt::Finished,
            status: "finished".to_owned(),
            key_hints: KeyHints::Contextual,
        };
        let content = render_tui_content(&state, 80, 20);

        assert!(content.contains("choice"));
        assert!(content.contains("help"));
        assert!(content.contains("line"));
        assert!(content.contains("Helped."));
        assert!(content.contains("end"));
        assert!(content.contains("Enter/Esc/q to exit"));
        assert!(!content.contains("No active prompt"));
    }

    #[test]
    fn tui_render_stays_structured_on_narrow_terminal() {
        let state = TuiState {
            asset: "/tmp/recite-play.recitec".to_owned(),
            block: "start".to_owned(),
            transcript: vec![TuiTranscriptEntry {
                kind: TuiTranscriptKind::Effect,
                id: Some("grant#1".to_owned()),
                text: "blocking grant_item (map)".to_owned(),
            }],
            prompt: TuiPrompt::Effect {
                mode: "blocking".to_owned(),
                id: "grant#1".to_owned(),
                function: "grant_item".to_owned(),
                args: "(map)".to_owned(),
                input_mode: PromptMode::Insert,
                input: TextBuffer::default(),
                command: TextBuffer::default(),
                show_help: false,
            },
            status: "ack grant#1 with Enter or ack".to_owned(),
            key_hints: KeyHints::Contextual,
        };
        let content = render_tui_content(&state, 60, 16);

        assert!(content.contains("recite play"));
        assert!(content.contains("Blocking Effect"));
        assert!(content.contains("runtime effect ID"));
        assert!(content.contains("grant#1"));
        assert!(content.contains("Enter or ack"));
    }

    fn render_tui_content(state: &TuiState, width: u16, height: u16) -> String {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");

        terminal
            .draw(|frame| render_tui(frame, state, &messages))
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }
}
