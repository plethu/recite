#![cfg(test)]

use recite_compiler::{
    AuthoringRequest, QueryResult, SavedDocument, SemanticFact, SnapshotGeneration,
};
use recite_core::DocumentKey;

use super::{fixture, key, position};

#[test]
fn typed_sites_and_hover_facts_preserve_parsed_spans() {
    let mut kernel = fixture();
    let source = concat!(
        ":: start\n",
        "? ask@11111111111111111111 requires=(knows_secret()) reason=innkeeper_trust_hint\n",
        ":if knows_secret\n",
        "  ordinary prose requires=(knows_secret())\n",
        "-> target.recite::finish\n",
    );
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key("main.recite"), source)],
            [],
        ))
        .expect("typed context fixture accepted");
    let snapshot = kernel.snapshot();
    let document = key("main.recite");

    let block_site = snapshot
        .completion_site(&document, position(5, 8))
        .expect("block completion site");
    assert_eq!(
        block_site.kind(),
        recite_compiler::CompletionSiteKind::Block
    );
    assert!(block_site.block_target().is_none());

    let qualified_site = snapshot
        .completion_site(&document, position(5, 20))
        .expect("qualified block completion site");
    assert_eq!(
        qualified_site.block_target().map(DocumentKey::as_str),
        Some("target.recite")
    );

    let reason_position = position(2, 76);
    let QueryResult::Ready(reason_hover) = snapshot.hover(&document, reason_position) else {
        panic!("availability reason hover is ready");
    };
    assert!(matches!(
        reason_hover.facts(),
        [SemanticFact::AvailabilityReason {
            name,
            template,
            parameters: 0
        }] if name == "innkeeper_trust_hint" && template == "Trust is too low."
    ));
    assert_eq!(reason_hover.location().span().start.column(), 61);
    assert_eq!(
        reason_hover
            .location()
            .span()
            .end
            .as_ref()
            .map(|end| end.column()),
        Some(80)
    );

    let QueryResult::Ready(requires_hover) = snapshot.hover(&document, position(2, 28)) else {
        panic!("requires clause hover is ready");
    };
    assert!(matches!(
        requires_hover.facts(),
        [SemanticFact::Clause {
            kind: recite_compiler::ClauseKind::Requires
        }]
    ));
    assert_eq!(requires_hover.location().span().start.column(), 28);
    assert_eq!(
        requires_hover
            .location()
            .span()
            .end
            .as_ref()
            .map(|end| end.column()),
        Some(52)
    );

    let QueryResult::Ready(if_hover) = snapshot.hover(&document, position(3, 1)) else {
        panic!("if clause hover is ready");
    };
    assert!(matches!(
        if_hover.facts(),
        [SemanticFact::Clause {
            kind: recite_compiler::ClauseKind::If
        }]
    ));
    assert_eq!(if_hover.location().span().start.column(), 1);
    assert_eq!(
        if_hover
            .location()
            .span()
            .end
            .as_ref()
            .map(|end| end.column()),
        Some(3)
    );

    assert!(matches!(
        snapshot.hover(&document, position(4, 20)),
        QueryResult::NoMatch
    ));
}

#[test]
fn typed_condition_sites_follow_parser_marker_boundaries() {
    let mut kernel = fixture();
    let source = concat!(
        ":: start\n",
        "\t:if\n",
        "\t:match\n",
        "\t:if\tknows_\n",
        "\tordinary prose :if knows_secret\n",
    );
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key("main.recite"), source)],
            [],
        ))
        .expect("condition marker fixture accepted");
    let snapshot = kernel.snapshot();
    let document = key("main.recite");

    for (line, column) in [(2, 5), (3, 8)] {
        let site = snapshot
            .completion_site(&document, position(line, column))
            .expect("bare condition marker site");
        assert_eq!(site.kind(), recite_compiler::CompletionSiteKind::Condition);
    }

    let tab_site = snapshot
        .completion_site(&document, position(4, 8))
        .expect("tab-separated condition site");
    assert_eq!(
        tab_site.span().start.column(),
        6,
        "condition token starts after tab whitespace"
    );
    assert_eq!(
        tab_site.span().end.as_ref().map(|end| end.column()),
        Some(7)
    );

    let QueryResult::Ready(marker_hover) = snapshot.hover(&document, position(4, 3)) else {
        panic!("hover on the exact :if marker is ready");
    };
    assert!(matches!(
        marker_hover.facts(),
        [SemanticFact::Clause {
            kind: recite_compiler::ClauseKind::If
        }]
    ));
    assert_eq!(marker_hover.location().span().start.column(), 2);
    assert_eq!(
        marker_hover
            .location()
            .span()
            .end
            .as_ref()
            .map(|end| end.column()),
        Some(4)
    );

    assert!(matches!(
        snapshot.hover(&document, position(4, 1)),
        QueryResult::NoMatch
    ));
    assert!(matches!(
        snapshot.hover(&document, position(4, 5)),
        QueryResult::NoMatch
    ));
    assert!(matches!(
        snapshot.hover(&document, position(5, 17)),
        QueryResult::NoMatch
    ));
}

#[test]
fn malformed_reason_and_partial_reference_queries_remain_conservative() {
    let mut kernel = fixture();
    let source = concat!(
        ":: start\n",
        "? ask@11111111111111111111 reason=innkeeper_trust_hint,\n",
        "-> missing\n",
    );
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key("main.recite"), source)],
            [],
        ))
        .expect("malformed context fixture accepted with recovery");
    let snapshot = kernel.snapshot();
    let document = key("main.recite");
    assert!(matches!(
        snapshot.hover(&document, position(2, 47)),
        QueryResult::NoMatch
    ));
    assert!(matches!(
        snapshot.navigate(&document, position(3, 4)),
        QueryResult::Ready(recite_compiler::NavigationResult::Missing)
    ));
}

#[test]
fn partial_reference_queries_expose_incomplete_coverage() {
    let mut kernel = fixture();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(
                key("main.recite"),
                ":: start\n-> target\n->\n:: target\n",
            )],
            [],
        ))
        .expect("partial reference fixture accepted with recovery");
    let document = key("main.recite");
    assert!(matches!(
        kernel.snapshot().navigate(&document, position(2, 5)),
        QueryResult::Partial {
            value: recite_compiler::NavigationResult::Unique(_),
            ..
        }
    ));
    assert!(matches!(
        kernel.snapshot().references(
            &document,
            position(2, 5),
            recite_compiler::SymbolQueryOptions::default(),
        ),
        QueryResult::Partial { .. }
    ));
}
