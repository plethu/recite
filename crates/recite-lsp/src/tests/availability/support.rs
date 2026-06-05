use lsp_types::Position;

pub(super) fn position_after(source: &str, needle: &str) -> Position {
    let index = source
        .find(needle)
        .unwrap_or_else(|| panic!("needle not found: {needle}"))
        + needle.len();
    position_for_byte_index(source, index)
}

pub(super) fn position_inside(source: &str, needle: &str) -> Position {
    let index = source
        .find(needle)
        .unwrap_or_else(|| panic!("needle not found: {needle}"))
        + 1;
    position_for_byte_index(source, index)
}

fn position_for_byte_index(source: &str, byte_index: usize) -> Position {
    let mut line = 0_u32;
    let mut character = 0_u32;
    for character_value in source[..byte_index].chars() {
        if character_value == '\n' {
            line = line.saturating_add(1);
            character = 0;
        } else {
            character = character.saturating_add(character_value.len_utf16() as u32);
        }
    }
    Position::new(line, character)
}

pub(super) fn authoring_schema() -> &'static str {
    r#"{
  "schema_version": 1,
  "speakers": {
    "hazel": { "display_name": "Hazel" },
    "rhea": {}
  },
  "conditions": {
    "can_talk": { "params": [] }
  },
  "effects": {
    "play_sfx": {
      "modes": ["immediate"],
      "params": []
    }
  },
  "metadata_domains": {
    "portrait_by_speaker": {
      "kind": "contextual",
      "selector": "field:speaker",
      "values_by_context": {
        "hazel": ["smile", "wry"],
        "rhea": ["flat"]
      },
      "missing_context": { "policy": "fallback", "domain": "portrait_all" }
    },
    "portrait_all": {
      "kind": "flat",
      "values": ["flat", "smile", "wry"]
    },
    "stage_by_mood": {
      "kind": "contextual",
      "selector": "metadata:mood",
      "values_by_context": {
        "warm": ["market"]
      },
      "missing_context": { "policy": "fallback", "domain": "stage_all" }
    },
    "stage_all": {
      "kind": "flat",
      "values": ["fallback_stage"]
    },
    "mood_by_tone": {
      "kind": "contextual",
      "selector": "metadata:tone",
      "values_by_context": {
        "bright": ["warm"]
      },
      "missing_context": { "policy": "empty" }
    }
  },
  "metadata": {
    "mood": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "mood_by_tone"
    },
    "portrait": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "portrait_by_speaker"
    },
    "stage": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "stage_by_mood"
    }
  }
}"#
}
