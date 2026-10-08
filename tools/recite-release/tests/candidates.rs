use serde_json::{Value, json};
use std::fs;

mod support;
use support::{Candidate, failure, git, success};

#[test]
fn numbered_channels_reject_ambiguous_or_invalid_versions() {
    let candidate = Candidate::new();
    for version in [
        "0.2.0-alpha.1",
        "0.2.0-beta.2",
        "0.2.0-rc.10",
        "0.2.0",
        "1.0.0",
    ] {
        success(candidate.run(&["version", version]));
    }
    for version in [
        "v0.2.0",
        "0.2.0-beta",
        "0.2.0-beta.0",
        "0.2.0-rc.01",
        "0.2.0+build.1",
        "0.2.0-dev.1",
    ] {
        assert!(
            !candidate.run(&["version", version]).status.success(),
            "{version}"
        );
    }
}

#[test]
fn candidate_requires_clean_exact_source_and_prepared_versions() {
    let candidate = Candidate::new();
    success(candidate.run(&["candidate", "0.2.0-beta.1", &candidate.commit]));
    failure(
        candidate.run(&[
            "candidate",
            "0.2.0-beta.1",
            "0000000000000000000000000000000000000000",
        ]),
        "does not match",
    );
    failure(
        candidate.run(&["candidate", "0.2.0-rc.1", &candidate.commit]),
        "prepare the product version",
    );
    candidate.write("Cargo.toml", b"[workspace.package]\nversion='0.2.0-rc.1'\n");
    failure(
        candidate.run(&["candidate", "0.2.0-rc.1", &candidate.commit]),
        "uncommitted changes",
    );
}

#[test]
fn receipt_rejects_changed_missing_extra_and_wrong_identity_artifacts() {
    let candidate = Candidate::new();
    success(candidate.seal());
    success(candidate.verify());
    failure(
        candidate.run(&[
            "verify",
            "0.2.0-rc.1",
            &candidate.commit,
            "receipt.json",
            "artifacts",
        ]),
        "different version or commit",
    );
    failure(
        candidate.run(&[
            "verify",
            "0.2.0-beta.1",
            "bad-sha",
            "receipt.json",
            "artifacts",
        ]),
        "different version or commit",
    );
    candidate.write("artifacts/cli/recite-cli.tar.xz", b"changed archive");
    failure(candidate.verify(), "differ from");
    candidate.write("artifacts/cli/recite-cli.tar.xz", b"archive");
    candidate.write("artifacts/extra", b"unexpected");
    failure(candidate.verify(), "differ from");
    fs::remove_file(candidate.root().join("artifacts/extra")).unwrap();
    fs::remove_file(candidate.root().join("artifacts/cli/recite-cli.tar.xz")).unwrap();
    failure(candidate.seal(), "missing planned distribution artifact");
    failure(candidate.verify(), "differ from");
}

#[test]
fn incomplete_targets_and_writer_groups_cannot_be_sealed() {
    let candidate = Candidate::new();
    let mut plan: Value =
        serde_json::from_slice(&fs::read(candidate.root().join("plan.json")).unwrap()).unwrap();
    plan["artifacts"]["recite-cli.tar.xz"]["target_triples"] = json!([]);
    candidate.write("plan.json", &serde_json::to_vec(&plan).unwrap());
    failure(candidate.seal(), "every configured distribution target");
    candidate.artifacts();
    fs::remove_dir_all(
        candidate
            .root()
            .join("artifacts/writer/recite-writer-macos-arm64-preview"),
    )
    .unwrap();
    failure(candidate.seal(), "missing Writer candidate artifact group");
}

#[test]
fn public_assets_preserve_dist_urls_and_disambiguate_host_metadata() {
    let candidate = Candidate::new();
    for platform in ["linux-x86_64", "macos-arm64"] {
        candidate.write(
            &format!("artifacts/writer/recite-writer-{platform}-preview/runtime-abi.json"),
            b"abi",
        );
    }
    success(candidate.seal());
    success(candidate.run(&[
        "assets",
        "0.2.0-beta.1",
        &candidate.commit,
        "receipt.json",
        "artifacts",
        "assets",
    ]));
    assert!(candidate.root().join("assets/recite-cli.tar.xz").is_file());
    assert!(
        candidate
            .root()
            .join("assets/recite-writer-linux-x86_64-preview-runtime-abi.json")
            .is_file()
    );
    assert!(
        candidate
            .root()
            .join("assets/recite-writer-macos-arm64-preview-runtime-abi.json")
            .is_file()
    );
    assert!(
        candidate
            .run(&[
                "assets",
                "0.2.0-beta.1",
                &candidate.commit,
                "receipt.json",
                "artifacts",
                "assets"
            ])
            .status
            .code()
            .is_some_and(|code| code != 0)
    );
}

#[test]
fn publication_requires_protected_workflow_origin_and_matching_run() {
    let candidate = Candidate::new();
    success(candidate.seal());
    failure(
        candidate.run(&["provenance", "receipt.json", "12", &candidate.commit]),
        "no hosted workflow",
    );
    let mut command = candidate.command(&[
        "seal",
        "0.2.0-beta.1",
        &candidate.commit,
        "plan.json",
        "artifacts",
        "receipt.json",
    ]);
    command
        .env("GITHUB_RUN_ID", "12")
        .env("GITHUB_SHA", &candidate.commit)
        .env(
            "GITHUB_WORKFLOW_REF",
            "plethu/recite/.github/workflows/release.yml@refs/heads/main",
        );
    success(command.output().unwrap());
    success(candidate.run(&["provenance", "receipt.json", "12", &candidate.commit]));
    failure(
        candidate.run(&["provenance", "receipt.json", "13", &candidate.commit]),
        "approved release workflow",
    );
    let mut receipt: Value =
        serde_json::from_slice(&fs::read(candidate.root().join("receipt.json")).unwrap()).unwrap();
    receipt["workflow"]["reference"] =
        json!("plethu/recite/.github/workflows/release.yml@refs/heads/topic");
    candidate.write("receipt.json", &serde_json::to_vec(&receipt).unwrap());
    failure(
        candidate.run(&["provenance", "receipt.json", "12", &candidate.commit]),
        "approved release workflow",
    );
}

#[test]
fn product_tag_must_be_annotated_and_identify_the_verified_commit() {
    let candidate = Candidate::new();
    git(candidate.root(), &["tag", "v0.2.0-beta.1"]);
    failure(
        candidate.run(&["tag", "0.2.0-beta.1", &candidate.commit]),
        "must be annotated",
    );
    git(candidate.root(), &["tag", "-d", "v0.2.0-beta.1"]);
    git(
        candidate.root(),
        &[
            "-c",
            "tag.gpgsign=false",
            "tag",
            "-a",
            "v0.2.0-beta.1",
            "-m",
            "candidate",
        ],
    );
    success(candidate.run(&["tag", "0.2.0-beta.1", &candidate.commit]));
    failure(
        candidate.run(&[
            "tag",
            "0.2.0-beta.1",
            "0000000000000000000000000000000000000000",
        ]),
        "does not identify",
    );
}
