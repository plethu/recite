//! A structured model of a generated Recite program and its schema.
//!
//! Programs are valid by construction: every ID is unique, every `:match`
//! is exhaustive for its scrutinee's enum, prompts and diverts end the
//! statement list that holds them, and a divert only moves to a later block,
//! so traversal reaches a player-visible event or the end in bounded steps.

/// The schema every generated program is validated against.
pub const SCHEMA_PATH: &str = "schema/generated.schema.json";

/// Conditions, effects, enums and speakers the generator draws from.
pub const SCHEMA_JSON: &str = r#"{
  "schema_version": 1,
  "types": {
    "actor": {"kind": "enum", "values": ["player", "rhea"]},
    "mood_kind": {"kind": "enum", "values": ["calm", "tired", "wary"]},
    "stage_kind": {"kind": "enum", "values": ["early", "late"]}
  },
  "speakers": {"s0": {}, "s1": {}},
  "conditions": {
    "flag": {"params": [{"name": "who", "type": "enum:actor"}]},
    "ready": {"params": []},
    "mood": {"params": [], "returns": "enum:mood_kind"},
    "stage": {"params": [{"name": "who", "type": "enum:actor"}], "returns": "enum:stage_kind"}
  },
  "effects": {
    "note": {"modes": ["immediate", "deferred", "blocking"], "params": [{"name": "who", "type": "enum:actor"}]},
    "ping": {"modes": ["immediate"], "params": []},
    "grant": {"modes": ["blocking"], "params": [{"name": "who", "type": "enum:actor"}]}
  }
}"#;

/// The asset ID every generated program compiles to, so a patched program
/// can resume saves from its baseline.
pub const ASSET_ID: &str = "generated.recitec";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Actor {
    Player,
    Rhea,
}

impl Actor {
    pub const ALL: [Self; 2] = [Self::Player, Self::Rhea];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Rhea => "rhea",
        }
    }

    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Player => 0,
            Self::Rhea => 1,
        }
    }
}

/// An enum-returning condition a `:match` dispatches on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scrutinee {
    Mood,
    Stage(Actor),
}

impl Scrutinee {
    pub const MOODS: [&'static str; 3] = ["calm", "tired", "wary"];
    pub const STAGES: [&'static str; 2] = ["early", "late"];

    /// The declared variants of the scrutinee's enum, in schema order.
    #[must_use]
    pub fn variants(self) -> &'static [&'static str] {
        match self {
            Self::Mood => &Self::MOODS,
            Self::Stage(_) => &Self::STAGES,
        }
    }
}

/// A boolean condition expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Cond {
    Flag(Actor),
    Ready,
    Not(Box<Cond>),
    And(Vec<Cond>),
    Or(Vec<Cond>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectMode {
    Immediate,
    Deferred,
    Blocking,
}

impl EffectMode {
    #[must_use]
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
            Self::Deferred => "deferred",
            Self::Blocking => "blocking",
        }
    }
}

/// An effect call, with a mode its schema declaration supports.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectSpec {
    Note(EffectMode, Actor),
    Ping,
    Grant(Actor),
}

impl EffectSpec {
    #[must_use]
    pub fn mode(self) -> EffectMode {
        match self {
            Self::Note(mode, _) => mode,
            Self::Ping => EffectMode::Immediate,
            Self::Grant(_) => EffectMode::Blocking,
        }
    }

    #[must_use]
    pub fn function(self) -> &'static str {
        match self {
            Self::Note(..) => "note",
            Self::Ping => "ping",
            Self::Grant(_) => "grant",
        }
    }

    #[must_use]
    pub fn actor(self) -> Option<Actor> {
        match self {
            Self::Note(_, actor) | Self::Grant(actor) => Some(actor),
            Self::Ping => None,
        }
    }
}

/// Where a divert or choice leads: a block, by file and block index, or the
/// end of the scene.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    End,
    Block { file: usize, block: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineSpec {
    pub id: String,
    pub speaker: Option<u8>,
    /// Binds `{count}` to the caller-supplied `$count` value.
    pub count: bool,
    pub text: String,
    /// Choices nested in the line make it a prompt.
    pub choices: Vec<ChoiceSpec>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChoiceSpec {
    pub id: String,
    pub text: String,
    pub requires: Option<Cond>,
    pub target: Target,
    /// A comment between this choice and the previous one in a standalone
    /// prompt, which must not split the prompt.
    pub comment_before: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArmSpec {
    /// A declared variant, or `None` for `:case _`.
    pub pattern: Option<&'static str>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Stmt {
    Line(LineSpec),
    /// A prompt without a line.
    Choices(Vec<ChoiceSpec>),
    Effect(EffectSpec),
    If {
        condition: Cond,
        then: Vec<Stmt>,
        otherwise: Vec<Stmt>,
    },
    Match {
        scrutinee: Scrutinee,
        arms: Vec<ArmSpec>,
    },
    Divert(Target),
    Comment,
}

impl Stmt {
    /// Prompts and diverts leave their statement list, so nothing follows
    /// them in it.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        match self {
            Self::Line(line) => !line.choices.is_empty(),
            Self::Choices(_) | Self::Divert(_) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockSpec {
    pub id: String,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSpec {
    pub path: String,
    pub blocks: Vec<BlockSpec>,
}

/// A generated program. The first block of the first file is the default.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub files: Vec<FileSpec>,
    /// How many line and choice IDs have been issued; the next fresh ID is
    /// numbered after them, so patches never reuse one.
    pub ids_issued: u32,
}

impl Program {
    /// Block positions in declaration order, which is also the order diverts
    /// may move forward in.
    #[must_use]
    pub fn block_positions(&self) -> Vec<(usize, usize)> {
        self.files
            .iter()
            .enumerate()
            .flat_map(|(file, spec)| (0..spec.blocks.len()).map(move |block| (file, block)))
            .collect()
    }

    /// The block's position in declaration order.
    #[must_use]
    pub fn global_index(&self, file: usize, block: usize) -> usize {
        self.files[..file]
            .iter()
            .map(|spec| spec.blocks.len())
            .sum::<usize>()
            + block
    }

    /// Takes the next fresh line or choice ID with `prefix`.
    pub fn fresh_id(&mut self, prefix: char) -> String {
        let id = format!("{prefix}{}", self.ids_issued);
        self.ids_issued += 1;
        id
    }
}
