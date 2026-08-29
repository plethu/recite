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
  "producer": { "kind": "adapter", "id": "authoring-fixtures" },
  "content_fingerprint": {
    "algorithm": "blake3",
    "value": "0000000000000000000000000000000000000000000000000000000000000000"
  },
  "producer_fingerprints": [
    { "id": "fixtures", "kind": "file", "algorithm": "blake3", "value": "fixture-v1" }
  ],
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
  "registries": {
    "item": {
      "values": ["map"],
      "producer_fingerprints": [
        { "id": "items", "kind": "fixture", "algorithm": "blake3", "value": "items-v1" }
      ]
    }
  },
  "metadata_domains": {
    "portrait_by_speaker": {
      "kind": "contextual",
      "selector": "field:speaker",
      "values_by_context": {
        "hazel": ["smile", "wry", "hazel", "hazel_only"],
        "rhea": ["flat", "rhea.face"]
      },
      "missing_context": { "policy": "fallback", "domain": "portrait_all" },
      "producer_fingerprints": [
        { "id": "portraits", "kind": "fixture", "algorithm": "blake3", "value": "portraits-v1" }
      ]
    },
    "portrait_all": {
      "kind": "flat",
      "values": ["flat", "smile", "wry"],
      "value_origins": {
        "wry": { "kind": "fixture", "id": "schema.json#portrait_all/wry" }
      }
    },
    "stage_by_mood": {
      "kind": "contextual",
      "selector": "metadata:mood",
      "values_by_context": {
        "warm": ["market"],
        "warm.tone": ["market"]
      },
      "missing_context": { "policy": "fallback", "domain": "stage_all" }
    },
    "stage_all": {
      "kind": "flat",
      "values": ["fallback_stage"],
      "value_origins": {
        "fallback_stage": { "kind": "fixture", "id": "schema.json#stage_all/fallback_stage" }
      }
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
      "targets": ["line", "choice"],
      "type": "symbol",
      "domain": "mood_by_tone"
    },
    "portrait": {
      "targets": ["line", "choice"],
      "type": "symbol",
      "domain": "portrait_by_speaker"
    },
    "stage": {
      "targets": ["line", "choice"],
      "type": "symbol",
      "domain": "stage_by_mood"
    }
  },
  "projection_queries": {
    "actor_skill": {
      "params": [{ "name": "skill", "type": "string" }],
      "returns": "int",
      "max_calls_per_event": 1
    }
  },
  "presentation_projectors": {
    "choice_skill_prefix": {
      "candidates": { "kind": "metadata_set", "target": "choice", "required_keys": ["mood", "stage"] },
      "inputs": [
        { "name": "skill", "source": { "kind": "literal", "value": "speech" }, "type": "string" },
        { "name": "threshold", "source": { "kind": "literal", "value": 3 }, "type": "int" }
      ],
      "queries": {
        "current": { "function": "actor_skill", "args": [{ "input": "skill" }] }
      },
      "outputs": {
        "prefix": {
          "target": "candidate",
          "kind": "badge",
          "slot": "prefix",
          "label": {
            "template_id": "skill_check_prefix",
            "source_text": "[{skill} {current}/{threshold}]",
            "args": {
              "skill": { "source": { "input": "skill" }, "type": "string" },
              "current": { "source": { "query_result": "current" }, "type": "int" },
              "threshold": { "source": { "input": "threshold" }, "type": "int" }
            }
          },
          "fields": {
            "current": { "source": { "kind": "query_result", "name": "current" }, "type": "int" },
            "threshold": { "source": { "kind": "input", "name": "threshold" }, "type": "int" }
          }
        }
      }
    }
  }
}"#
}
