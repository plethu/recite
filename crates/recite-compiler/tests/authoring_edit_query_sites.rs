#![cfg(test)]

use recite_compiler::{AuthoringKernel, AuthoringRequest, SavedDocument, SnapshotGeneration};
use recite_core::{DocumentKey, SourcePosition};

fn key(value: &str) -> DocumentKey {
    match DocumentKey::new(value.to_owned()) {
        Ok(key) => key,
        Err(error) => panic!("invalid test document key {value:?}: {error:?}"),
    }
}

fn position(line: u32, column: u32) -> SourcePosition {
    match SourcePosition::new(line, column) {
        Ok(position) => position,
        Err(error) => panic!("invalid test position {line}:{column}: {error:?}"),
    }
}

fn kernel(source: &str) -> AuthoringKernel {
    let mut kernel = AuthoringKernel::new();
    match kernel.apply(AuthoringRequest::new(
        SnapshotGeneration::initial(),
        [SavedDocument::new(key("main.recite"), source)],
        [],
    )) {
        Ok(_) => kernel,
        Err(error) => panic!("test source was rejected: {error:?}"),
    }
}

#[test]
fn non_block_sites_have_no_block_target_resolution() {
    let cases = [
        (
            "> speaker=hazel\n",
            position(1, 12),
            recite_compiler::CompletionSiteKind::Speaker,
        ),
        (
            "> mood=calm\n",
            position(1, 8),
            recite_compiler::CompletionSiteKind::MetadataValue,
        ),
        (
            "> speaker=hazel mood\n",
            position(1, 20),
            recite_compiler::CompletionSiteKind::MetadataKey,
        ),
        (
            ":if knows_secret\n",
            position(1, 8),
            recite_compiler::CompletionSiteKind::Condition,
        ),
        (
            "! do_thing\n",
            position(1, 5),
            recite_compiler::CompletionSiteKind::Effect,
        ),
        (
            "? reason=low\n",
            position(1, 10),
            recite_compiler::CompletionSiteKind::AvailabilityReason,
        ),
    ];
    for (source, position, kind) in cases {
        let kernel = kernel(source);
        let Some(site) = kernel
            .snapshot()
            .completion_site(&key("main.recite"), position)
        else {
            panic!("completion site should be classified for {source:?}");
        };
        assert_eq!(site.kind(), kind);
        assert!(site.block_target_resolution().is_none());
        assert!(site.block_target().is_none());
    }
}

#[test]
fn block_sites_preserve_local_qualified_and_invalid_target_resolution() {
    let kernel = kernel("-> missing\n-> target.recite::missing\n-> ../target.recite::missing\n");
    let snapshot = kernel.snapshot();
    let Some(local) = snapshot.completion_site(&key("main.recite"), position(1, 9)) else {
        panic!("local block site should be classified");
    };
    assert!(matches!(
        local.block_target_resolution(),
        Some(recite_compiler::BlockTarget::Local)
    ));
    let Some(qualified) = snapshot.completion_site(&key("main.recite"), position(2, 23)) else {
        panic!("qualified block site should be classified");
    };
    assert!(matches!(
        qualified.block_target_resolution(),
        Some(recite_compiler::BlockTarget::Qualified(target)) if target.as_str() == "target.recite"
    ));
    let Some(invalid) = snapshot.completion_site(&key("main.recite"), position(3, 23)) else {
        panic!("invalid qualified block site should be classified");
    };
    assert!(matches!(
        invalid.block_target_resolution(),
        Some(recite_compiler::BlockTarget::InvalidQualified { target })
            if target == "../target.recite"
    ));
}
