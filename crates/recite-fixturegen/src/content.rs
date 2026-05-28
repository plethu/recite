use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};

const WORDS: &[&str] = &[
    "amber", "bridge", "circuit", "delta", "ember", "fable", "garden", "harbor", "ion", "jasmine",
    "keystone", "lantern", "meadow", "needle", "orbit", "prairie", "quartz", "ripple", "signal",
    "thicket", "umbra", "velvet", "willow", "xenial", "yonder", "zenith",
];

pub(crate) struct GeneratedText {
    seed: u64,
    words_per_entry: u32,
}

impl GeneratedText {
    pub(crate) fn new(seed: u64, words_per_entry: u32) -> Self {
        Self {
            seed,
            words_per_entry,
        }
    }

    pub(crate) fn entry(&self, kind: &str, block: u32, index: u32) -> String {
        let mut text = format!("{kind} {block:05} {index:03}");
        let mut rng = FixtureRng::for_entry(self.seed, kind, block, index);
        for _ in 0..self.words_per_entry {
            let word = WORDS[rng.index(WORDS.len() as u32) as usize];
            text.push(' ');
            text.push_str(word);
        }
        text.push('.');
        text
    }
}

struct FixtureRng {
    rng: ChaCha8Rng,
}

impl FixtureRng {
    fn for_entry(seed: u64, kind: &str, block: u32, index: u32) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&seed.to_le_bytes());
        hasher.update(kind.as_bytes());
        hasher.update(&block.to_le_bytes());
        hasher.update(&index.to_le_bytes());
        let seed = *hasher.finalize().as_bytes();
        Self {
            rng: ChaCha8Rng::from_seed(seed),
        }
    }

    fn index(&mut self, upper_bound: u32) -> u32 {
        self.rng.next_u32() % upper_bound
    }
}
