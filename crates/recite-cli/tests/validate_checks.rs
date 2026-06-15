#![cfg(test)]

use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn validate_reports_diagnostics_and_success() {
    let temp = TempDir::new().expect("tempdir");
    let valid = write_recite(
        temp.path(),
        "valid.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    recite()
        .arg("validate")
        .arg(&valid)
        .assert_success()
        .assert_stderr("");

    let invalid = write_recite(
        temp.path(),
        "invalid.recite",
        ":: start default\n>\n  Missing id.\n",
    );
    let output = run(recite().arg("validate").arg(&invalid));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("error RECITE_ID001");
    output.assert_stderr_contains("invalid.recite:2:1");
}

#[test]
fn operational_failures_keep_error_prefix() {
    let temp = TempDir::new().expect("tempdir");
    let output = run(recite().arg("validate").arg(temp.path()));

    output.assert_failure();
    output.assert_exit_code(1);
    output.assert_stderr("error: no .recite inputs found\n");
}

#[test]
fn check_ids_reports_only_id_diagnostics() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "ids.recite",
        ":: start\n>\n  Missing id and missing default.\n> repeated\n  Plain IDs are malformed.\n> first@22222222222222222222\n  First.\n> second@22222222222222222222\n  Duplicate anchor.\n",
    );

    let output = run(recite().arg("check-ids").arg(&source));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_ID001");
    output.assert_stderr_contains("RECITE_ID007");
    output.assert_stderr_contains("RECITE_ID003");
    output.assert_stderr_not_contains("RECITE_VALIDATE005");

    let unrelated = write_recite(
        temp.path(),
        "unrelated.recite",
        ":: start\n> intro@11111111111111111111\n  IDs are present, but no default block.\n",
    );
    recite()
        .arg("check-ids")
        .arg(unrelated)
        .assert_success()
        .assert_stderr("");
}

#[test]
fn check_markup_uses_schema_when_supplied_and_skips_schema_policy_without_one() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "markup.recite",
        ":: start default\n> intro@11111111111111111111\n  [ghost]Hello[/ghost]\n-> END\n",
    );

    recite()
        .arg("check-markup")
        .arg(&source)
        .assert_success()
        .assert_stderr("");

    let malformed = write_recite(
        temp.path(),
        "malformed.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n    Mixed indent.\n",
    );
    let output = run(recite().arg("check-markup").arg(malformed));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_PARSE007");

    let schema = write_file(
        temp.path(),
        "schema.json",
        r#"{"schema_version":1,"markup":{"em":{"requires_closing":true,"translatable":true,"allows_nesting":true}}}"#,
    );
    let output = run(recite()
        .arg("check-markup")
        .arg("--schema")
        .arg(schema)
        .arg(source));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_VALIDATE022");
}

#[test]
fn check_metadata_requires_schema_and_reports_schema_validation() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "metadata.recite",
        ":: start default\n> intro@11111111111111111111 mood=\"angry\"\n  Hello.\n-> END\n",
    );
    let schema = write_file(
        temp.path(),
        "schema.json",
        r#"{"schema_version":1,"metadata":{"mood":{"targets":["line"],"type":"string","repeatable":false}}}"#,
    );

    recite()
        .arg("check-metadata")
        .arg("--schema")
        .arg(&schema)
        .arg(&source)
        .assert_success();

    let invalid_metadata = write_recite(
        temp.path(),
        "invalid-metadata.recite",
        ":: start default\n> intro@11111111111111111111 unknown=flat\n  Hello.\n-> END\n",
    );
    let output = run(recite()
        .arg("check-metadata")
        .arg("--schema")
        .arg(&schema)
        .arg(invalid_metadata));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_VALIDATE026");

    let bad_schema = write_file(
        temp.path(),
        "bad-schema.json",
        r#"{"schema_version":"one"}"#,
    );
    let output = run(recite()
        .arg("check-metadata")
        .arg("--schema")
        .arg(bad_schema)
        .arg(source));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_SCHEMA001");
}

#[test]
fn check_metadata_reports_domain_diagnostics_without_unrelated_validation() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "metadata-domain.recite",
        ":: start\n> intro@11111111111111111111 portrait=missing\n  Hello.\n",
    );
    let schema = write_file(
        temp.path(),
        "schema.json",
        r#"{
  "schema_version": 1,
  "metadata_domains": {
    "portrait_values": {
      "kind": "flat",
      "values": ["flat"]
    }
  },
  "metadata": {
    "portrait": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "portrait_values"
    }
  }
}"#,
    );

    let output = run(recite()
        .arg("check-metadata")
        .arg("--schema")
        .arg(schema)
        .arg(source));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_VALIDATE031");
    output.assert_stderr_not_contains("RECITE_VALIDATE005");
}

#[test]
fn check_metadata_reports_projection_schema_diagnostics_with_spans() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "metadata.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    let schema = write_file(
        temp.path(),
        "projection-schema.json",
        r#"{
  "schema_version": 1,
  "metadata": {
    "skill": { "targets": ["line"], "type": "string" },
    "tag": { "targets": ["choice"], "type": "symbol" }
  },
  "projection_queries": {
    "actor_skill": {
      "params": [{ "name": "skill", "type": "string" }],
      "returns": "int"
    }
  },
  "presentation_projectors": {
    "choice_skill_prefix": {
      "candidates": { "kind": "metadata_key", "target": "choice", "key": "skill" },
      "inputs": [
        { "name": "skill", "source": { "kind": "candidate_metadata", "key": "skill" }, "type": "int" },
        { "name": "tags", "source": { "kind": "candidate_metadata", "key": "tag", "occurrence": "all" }, "type": "symbol" }
      ],
      "queries": {
        "current": { "function": "missing", "args": [{ "input": "skill" }] }
      },
      "outputs": {
        "prefix": {
          "target": "candidate",
          "kind": "badge",
          "slot": "prefix",
          "label": {
            "template_id": "skill_check_prefix",
            "source_text": "[{skill} {current}]",
            "args": {
              "skill": { "source": { "input": "skill" }, "type": "string" }
            }
          },
          "fields": {
            "current": { "source": { "kind": "query_result", "name": "current" }, "type": "int" }
          }
        }
      }
    }
  }
}"#,
    );

    let output = run(recite()
        .arg("check-metadata")
        .arg("--schema")
        .arg(schema)
        .arg(source));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_SCHEMA001");
    output.assert_stderr_contains("projection-schema.json:15:");
    output.assert_stderr_contains("unknown projection query function 'missing'");
}
