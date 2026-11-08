#![cfg(test)]

use recite_compiler::authoring::{
    AuthoringEditError, AuthoringEditOperation, AuthoringError, AuthoringKernel, AuthoringRequest,
    DocumentVersion, OpenDocument, SavedDocument, SnapshotGeneration, StableIdRefusal,
    StableIdTarget,
};
use recite_core::{DocumentKey, SourcePosition, StableId, StableIdGenerator};

const MAIN: &str = concat!(
    ":: start\n",
    "> speaker=mara @desk_greeting\n",
    "  Hello there.\n",
    "  ? @ask_room\n",
    "    I need a room.\n",
    "    -> END\n",
);

const OTHER: &str = ":: other\r\n> @k7q3m9x\r\n  Elsewhere.\r\n";

const FRENCH: &str = concat!(
    "msgid \"\"\nmsgstr \"Language: fr\\n\"\n\n",
    "msgctxt \"desk_greeting\"\nmsgid \"Hello there.\"\nmsgstr \"Bonjour.\"\n\n",
    "msgctxt \"desk_greeting&formal\"\nmsgid \"Hello there.\"\nmsgstr \"Bonjour, madame.\"\n\n",
    "#, fuzzy\n#| msgctxt \"desk_greeting\"\n#| msgid \"Hello.\"\nmsgctxt \"desk_greeting&casual\"\nmsgid \"Hello there.\"\nmsgstr \"Salut.\"\n\n",
    "msgctxt \"ask_room\"\nmsgid \"I need a room.\"\nmsgstr \"Il me faut une chambre.\"\n",
);

const GERMAN: &str = "msgctxt \"ask_room\"\r\nmsgid \"I need a room.\"\r\nmsgstr \"Ich brauche ein Zimmer.\"\r\n\r\n#~ msgctxt \"retired_id\"\r\n#~ msgid \"Gone.\"\r\n#~ msgstr \"Weg.\"\r\n";

fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value).expect("test document key is valid")
}

fn id(value: &str) -> StableId {
    StableId::new(value).expect("test stable ID is valid")
}

fn kernel_with(sources: &[(&str, &str)], catalogs: &[(&str, &str)]) -> AuthoringKernel {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(
            AuthoringRequest::new(
                SnapshotGeneration::initial(),
                sources
                    .iter()
                    .map(|(path, text)| SavedDocument::new(key(path), *text)),
                [],
            )
            .with_catalogs(
                catalogs
                    .iter()
                    .map(|(path, text)| SavedDocument::new(key(path), *text)),
                [],
            ),
        )
        .expect("inputs accepted");
    kernel
}

fn project() -> AuthoringKernel {
    kernel_with(
        &[("main.recite", MAIN), ("other.recite", OTHER)],
        &[("locales/de.po", GERMAN), ("locales/fr.po", FRENCH)],
    )
}

fn applied(
    kernel: &AuthoringKernel,
    plan: &recite_compiler::authoring::AuthoringEditPlan,
) -> Vec<(String, String)> {
    plan.apply(kernel.snapshot())
        .expect("fresh plan applies")
        .into_iter()
        .map(|(key, text)| (key.as_str().to_owned(), text))
        .collect()
}

#[test]
fn rename_updates_header_and_every_catalogue_context() {
    let kernel = project();
    let plan = kernel
        .snapshot()
        .plan_rename_stable_id(
            &StableIdTarget::Id(id("desk_greeting")),
            "@front_desk_hello",
        )
        .expect("rename is planable");
    assert_eq!(
        plan.operation(),
        &AuthoringEditOperation::RenameStableId {
            from: id("desk_greeting"),
            to: id("front_desk_hello"),
        }
    );
    // All sources and catalogues guard the plan.
    assert_eq!(plan.preconditions().len(), 4);

    let documents = applied(&kernel, &plan);
    assert_eq!(
        documents
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["locales/fr.po", "main.recite"]
    );
    let french = &documents[0].1;
    assert_eq!(
        french,
        &FRENCH
            .replace("\"desk_greeting\"", "\"front_desk_hello\"")
            .replace("\"desk_greeting&", "\"front_desk_hello&")
    );
    assert!(french.contains("#| msgctxt \"front_desk_hello\""));
    assert_eq!(
        documents[1].1,
        MAIN.replace("@desk_greeting", "@front_desk_hello")
    );
}

#[test]
fn rename_by_position_and_line_select_the_same_header() {
    let kernel = project();
    let snapshot = kernel.snapshot();
    let token = recite_compiler::authoring::CancellationToken::new();
    let query = snapshot.query(&token);
    let at = |line, column| {
        query.stable_id_occurrence(&StableIdTarget::position(
            key("main.recite"),
            SourcePosition::new(line, column).expect("position"),
        ))
    };
    let occurrence = at(4, 6).expect("cursor on `@ask_room`");
    assert_eq!(occurrence.id(), &id("ask_room"));
    assert_eq!(occurrence.id_range().start().column(), 6);
    assert_eq!(occurrence.field_range().start().column(), 5);
    // The cursor just after the ID still selects it.
    assert_eq!(at(4, 14).expect("cursor after ID").id(), &id("ask_room"));
    assert!(matches!(at(3, 3), Err(AuthoringEditError::NoSymbol { .. })));

    let by_line = query
        .stable_id_occurrence(&StableIdTarget::line(key("main.recite"), 4))
        .expect("line selects the choice");
    assert_eq!(by_line, occurrence);

    let plan = snapshot
        .plan_rename_stable_id(&StableIdTarget::line(key("main.recite"), 4), "room_request")
        .expect("rename by line");
    let documents = applied(&kernel, &plan);
    let german = documents
        .iter()
        .find(|(key, _)| key == "locales/de.po")
        .expect("German catalogue changes");
    assert_eq!(german.1, GERMAN.replace("\"ask_room\"", "\"room_request\""));
}

#[test]
fn rename_refuses_invalid_reserved_and_existing_ids() {
    let kernel = project();
    let snapshot = kernel.snapshot();
    let target = StableIdTarget::Id(id("desk_greeting"));
    let refusal = |new_id| snapshot.plan_rename_stable_id(&target, new_id);

    for (spelled, character) in [("desk&formal", '&'), ("pack:desk", ':'), ("dlc/desk", '/')] {
        assert_eq!(
            refusal(spelled),
            Err(AuthoringEditError::InvalidStableId {
                id: spelled.to_owned(),
                reason: StableIdRefusal::ReservedCharacter(character),
            })
        );
    }
    for spelled in ["", "Desk", "9desk", "desk greeting", "@@desk"] {
        assert!(
            matches!(
                refusal(spelled),
                Err(AuthoringEditError::InvalidStableId {
                    reason: StableIdRefusal::Malformed,
                    ..
                })
            ),
            "{spelled:?}"
        );
    }
    assert_eq!(
        refusal("k7q3m9x"),
        Err(AuthoringEditError::StableIdCollision {
            id: id("k7q3m9x"),
            document: None,
        })
    );
    // An obsolete translation still keys the ID; reusing it would revive it.
    assert_eq!(
        refusal("retired_id"),
        Err(AuthoringEditError::StableIdCollision {
            id: id("retired_id"),
            document: Some(key("locales/de.po")),
        })
    );
    assert_eq!(refusal("desk_greeting"), Err(AuthoringEditError::NoEdits));
    assert_eq!(
        snapshot.plan_rename_stable_id(&StableIdTarget::Id(id("absent")), "fresh"),
        Err(AuthoringEditError::UnknownStableId {
            id: "absent".to_owned()
        })
    );
}

#[test]
fn reassigning_one_side_of_a_duplicate_leaves_catalogues_with_the_other() {
    let branch = ":: branch\n> @desk_greeting\n  Added on another branch.\n";
    let kernel = kernel_with(
        &[("branch.recite", branch), ("main.recite", MAIN)],
        &[("locales/fr.po", FRENCH)],
    );
    let snapshot = kernel.snapshot();
    assert_eq!(
        snapshot.plan_rename_stable_id(&StableIdTarget::Id(id("desk_greeting")), "fresh"),
        Err(AuthoringEditError::AmbiguousStableId {
            id: id("desk_greeting")
        })
    );

    let target = StableIdTarget::line(key("branch.recite"), 2);
    let plan = snapshot
        .plan_reassign_stable_id(&target, &mut StableIdGenerator::seeded(7))
        .expect("duplicate reassignment is planable");
    let repeated = snapshot
        .plan_reassign_stable_id(&target, &mut StableIdGenerator::seeded(7))
        .expect("seeded reassignment repeats");
    assert_eq!(plan, repeated);
    let AuthoringEditOperation::ReassignDuplicateStableId { from, to } = plan.operation() else {
        panic!("unexpected operation {:?}", plan.operation());
    };
    assert_eq!(from, &id("desk_greeting"));
    assert_eq!(to.as_str().len(), recite_core::GENERATED_STABLE_ID_LEN);

    let documents = applied(&kernel, &plan);
    assert_eq!(documents.len(), 1, "catalogues are untouched");
    assert_eq!(documents[0].0, "branch.recite");
    assert_eq!(
        documents[0].1,
        branch.replace("@desk_greeting", &format!("@{to}"))
    );
}

#[test]
fn reassigning_a_unique_id_moves_its_catalogue_contexts() {
    let kernel = project();
    let plan = kernel
        .snapshot()
        .plan_reassign_stable_id(
            &StableIdTarget::Id(id("ask_room")),
            &mut StableIdGenerator::seeded(3),
        )
        .expect("reassignment is planable");
    let AuthoringEditOperation::RenameStableId { to, .. } = plan.operation() else {
        panic!("unexpected operation {:?}", plan.operation());
    };
    let documents = applied(&kernel, &plan);
    assert_eq!(documents.len(), 3);
    assert!(
        documents
            .iter()
            .all(|(_, text)| text.contains(to.as_str()) && !text.contains("ask_room"))
    );
}

#[test]
fn plans_refuse_stale_catalogues_and_malformed_or_partial_inputs() {
    let mut kernel = project();
    let plan = kernel
        .snapshot()
        .plan_rename_stable_id(&StableIdTarget::Id(id("desk_greeting")), "renamed")
        .expect("rename is planable");
    let generation = kernel.snapshot().generation();
    kernel
        .apply(
            AuthoringRequest::new(
                generation,
                [
                    SavedDocument::new(key("main.recite"), MAIN),
                    SavedDocument::new(key("other.recite"), OTHER),
                ],
                [],
            )
            .with_catalogs(
                [SavedDocument::new(key("locales/de.po"), GERMAN)],
                [OpenDocument::new(
                    key("locales/fr.po"),
                    DocumentVersion::new(2),
                    FRENCH.replace("Salut.", "Coucou."),
                )],
            ),
        )
        .expect("catalogue overlay accepted");
    assert!(matches!(
        plan.apply(kernel.snapshot()),
        Err(AuthoringEditError::StaleGeneration { .. })
    ));
    let replanned = kernel
        .snapshot()
        .plan_rename_stable_id(&StableIdTarget::Id(id("desk_greeting")), "renamed")
        .expect("fresh plan");
    let french = replanned
        .preconditions()
        .iter()
        .find(|precondition| precondition.document() == &key("locales/fr.po"))
        .expect("catalogue precondition");
    assert_eq!(french.expected_version(), Some(DocumentVersion::new(2)));

    let malformed = kernel_with(
        &[("main.recite", MAIN)],
        &[("locales/fr.po", "msgid \"unterminated\n")],
    );
    assert_eq!(
        malformed
            .snapshot()
            .plan_rename_stable_id(&StableIdTarget::Id(id("desk_greeting")), "renamed"),
        Err(AuthoringEditError::MalformedCatalog {
            document: key("locales/fr.po")
        })
    );

    let mut partial = AuthoringKernel::new();
    partial
        .apply_with_incomplete_project(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key("main.recite"), MAIN)],
            [],
        ))
        .expect("partial project accepted");
    assert_eq!(
        partial
            .snapshot()
            .plan_rename_stable_id(&StableIdTarget::Id(id("desk_greeting")), "renamed"),
        Err(AuthoringEditError::IncompleteProject)
    );
}

#[test]
fn catalogue_keys_cannot_shadow_source_documents() {
    let mut kernel = AuthoringKernel::new();
    let error = kernel
        .apply(
            AuthoringRequest::new(
                SnapshotGeneration::initial(),
                [SavedDocument::new(key("shared.po"), MAIN)],
                [],
            )
            .with_catalogs([SavedDocument::new(key("shared.po"), FRENCH)], []),
        )
        .expect_err("key conflict");
    assert_eq!(
        error,
        AuthoringError::CatalogKeyConflict {
            key: key("shared.po")
        }
    );
}
