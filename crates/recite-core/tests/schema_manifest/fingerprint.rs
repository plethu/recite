use std::collections::{BTreeMap, BTreeSet};

use recite_core::{
    AvailabilityReasonArgBinding, AvailabilityReasonDefinition, AvailabilityReasonId,
    BLAKE3_DIGEST_LEN, ConditionAvailabilityReasonMapping, ConditionDefinition,
    ConditionReturnType, ContextualMetadataDomain, EffectDefinition, EffectMode,
    EnumTypeDefinition, FlatMetadataDomain, MarkupDefinition, MetadataContextSelector,
    MetadataDefinition, MetadataDomainDefinition, MetadataTarget, MissingMetadataContextPolicy,
    ParameterDefinition, PresentationAffordanceFieldDefinition, PresentationAffordanceFieldSource,
    PresentationAffordanceOutputDefinition, PresentationLabelArgDefinition,
    PresentationLabelDefinition, ProjectSchema, ProjectionInput, ProjectionInputRef,
    ProjectionOutputTarget, ProjectionQueryDefinition, ProjectionQueryFunctionDefinition,
    RegistryDefinition, SchemaFingerprint, SchemaLiteralValue,
    SchemaPresentationProjectorDefinition, SchemaProjectionInputSource, SchemaProjectionSelector,
    SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition, canonical_schema_fingerprint,
};

#[test]
fn schema_fingerprint_is_deterministic_for_different_insertion_orders() {
    let forward = schema_with_order(Order::Forward);
    let reverse = schema_with_order(Order::Reverse);

    assert_eq!(
        canonical_schema_fingerprint(&forward),
        canonical_schema_fingerprint(&reverse)
    );
}

#[test]
fn schema_fingerprint_uses_blake3_digest_shape() {
    let SchemaFingerprint::Fingerprint(fingerprint) =
        canonical_schema_fingerprint(&schema_with_order(Order::Forward))
    else {
        panic!("schema helper always emits a content fingerprint");
    };

    assert_eq!(fingerprint.algorithm().as_str(), "blake3");
    assert_eq!(fingerprint.digest().as_bytes().len(), BLAKE3_DIGEST_LEN);
}

#[test]
fn schema_fingerprint_changes_when_freshness_relevant_fields_change() {
    let base = schema_with_order(Order::Forward);
    let base_fingerprint = canonical_schema_fingerprint(&base);

    let mut version = base.clone();
    version.schema_version = 2;
    assert_ne!(base_fingerprint, canonical_schema_fingerprint(&version));

    let mut type_values = base.clone();
    let SchemaTypeDefinition::Enum(thread_stage) = type_values
        .types
        .get_mut("thread_stage")
        .expect("type exists");
    thread_stage.values.insert("completed".to_owned());
    assert_ne!(base_fingerprint, canonical_schema_fingerprint(&type_values));

    let mut registry_origin = base.clone();
    registry_origin
        .registries
        .get_mut("sound")
        .expect("registry exists")
        .origin = Some("data/changed.toml".to_owned());
    assert_ne!(
        base_fingerprint,
        canonical_schema_fingerprint(&registry_origin)
    );

    let mut speaker = base.clone();
    speaker
        .speakers
        .get_mut("hazel")
        .expect("speaker exists")
        .display_name = Some("Hazel".to_owned());
    assert_ne!(base_fingerprint, canonical_schema_fingerprint(&speaker));

    let mut condition = base.clone();
    condition
        .conditions
        .get_mut("thread_stage")
        .expect("condition exists")
        .returns = ConditionReturnType::Bool;
    assert_ne!(base_fingerprint, canonical_schema_fingerprint(&condition));

    let mut condition_reason_mapping = base.clone();
    condition_reason_mapping
        .conditions
        .get_mut("trust_gte")
        .expect("condition exists")
        .availability_reason
        .as_mut()
        .expect("mapping exists")
        .args
        .insert(
            "threshold".to_owned(),
            AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Int(4)),
        );
    assert_ne!(
        base_fingerprint,
        canonical_schema_fingerprint(&condition_reason_mapping)
    );

    let mut availability_reason = base.clone();
    availability_reason
        .availability_reasons
        .get_mut(&AvailabilityReasonId::new("trust_too_low").expect("valid reason id"))
        .expect("availability reason exists")
        .template = "{subject} needs more trust.".to_owned();
    assert_ne!(
        base_fingerprint,
        canonical_schema_fingerprint(&availability_reason)
    );

    let mut effect = base.clone();
    effect
        .effects
        .get_mut("play_sfx")
        .expect("effect exists")
        .modes
        .insert(EffectMode::Blocking);
    assert_ne!(base_fingerprint, canonical_schema_fingerprint(&effect));

    let mut metadata = base.clone();
    metadata
        .metadata
        .get_mut("sfx")
        .expect("metadata exists")
        .repeatable = false;
    assert_ne!(base_fingerprint, canonical_schema_fingerprint(&metadata));

    let mut metadata_domain = base.clone();
    let MetadataDomainDefinition::Flat(portrait_all) = metadata_domain
        .metadata_domains
        .get_mut("portrait_all")
        .expect("metadata domain exists")
    else {
        panic!("portrait_all is flat");
    };
    portrait_all.values.insert("wry".to_owned());
    assert_ne!(
        base_fingerprint,
        canonical_schema_fingerprint(&metadata_domain)
    );

    let mut metadata_domain_selector = base.clone();
    let MetadataDomainDefinition::Contextual(portrait_by_speaker) = metadata_domain_selector
        .metadata_domains
        .get_mut("portrait_by_speaker")
        .expect("metadata domain exists")
    else {
        panic!("portrait_by_speaker is contextual");
    };
    portrait_by_speaker.selector = MetadataContextSelector::MetadataKey("subject".to_owned());
    assert_ne!(
        base_fingerprint,
        canonical_schema_fingerprint(&metadata_domain_selector)
    );

    let mut metadata_domain_context = base.clone();
    let MetadataDomainDefinition::Contextual(portrait_by_speaker) = metadata_domain_context
        .metadata_domains
        .get_mut("portrait_by_speaker")
        .expect("metadata domain exists")
    else {
        panic!("portrait_by_speaker is contextual");
    };
    portrait_by_speaker
        .values_by_context
        .entry("hazel".to_owned())
        .or_default()
        .insert("concerned".to_owned());
    assert_ne!(
        base_fingerprint,
        canonical_schema_fingerprint(&metadata_domain_context)
    );

    let mut metadata_domain_missing = base.clone();
    let MetadataDomainDefinition::Contextual(portrait_by_speaker) = metadata_domain_missing
        .metadata_domains
        .get_mut("portrait_by_speaker")
        .expect("metadata domain exists")
    else {
        panic!("portrait_by_speaker is contextual");
    };
    portrait_by_speaker.missing_context = MissingMetadataContextPolicy::Empty;
    assert_ne!(
        base_fingerprint,
        canonical_schema_fingerprint(&metadata_domain_missing)
    );

    let mut metadata_domain_reference = base.clone();
    metadata_domain_reference
        .metadata
        .get_mut("portrait")
        .expect("metadata exists")
        .domain = Some("portrait_all".to_owned());
    assert_ne!(
        base_fingerprint,
        canonical_schema_fingerprint(&metadata_domain_reference)
    );

    let mut markup = base.clone();
    markup
        .markup
        .get_mut("slow")
        .expect("markup exists")
        .allows_nesting = false;
    assert_ne!(base_fingerprint, canonical_schema_fingerprint(&markup));

    let mut projection_query = base.clone();
    projection_query
        .projection_queries
        .get_mut("actor_skill")
        .expect("projection query exists")
        .returns = SchemaTypeRef::Float;
    assert_ne!(
        base_fingerprint,
        canonical_schema_fingerprint(&projection_query)
    );

    let mut projector_label = base.clone();
    projector_label
        .presentation_projectors
        .get_mut("choice_skill_prefix")
        .expect("projector exists")
        .outputs
        .get_mut("prefix")
        .expect("output exists")
        .label
        .as_mut()
        .expect("label exists")
        .source_text = "[{skill}]".to_owned();
    assert_ne!(
        base_fingerprint,
        canonical_schema_fingerprint(&projector_label)
    );
}

#[derive(Clone, Copy)]
enum Order {
    Forward,
    Reverse,
}

fn schema_with_order(order: Order) -> ProjectSchema {
    let mut schema = ProjectSchema::empty_v1();

    insert_entries_str(
        &mut schema.types,
        order,
        [
            (
                "thread_stage",
                SchemaTypeDefinition::Enum(EnumTypeDefinition {
                    values: set(order, ["fresh", "tired", "fine"]),
                }),
            ),
            (
                "mood",
                SchemaTypeDefinition::Enum(EnumTypeDefinition {
                    values: set(order, ["calm", "tense"]),
                }),
            ),
        ],
    );
    insert_entries_str(
        &mut schema.registries,
        order,
        [
            (
                "sound",
                RegistryDefinition {
                    values: set(order, ["door", "rain"]),
                    origin: Some("data/sounds.toml".to_owned()),
                },
            ),
            (
                "thread",
                RegistryDefinition {
                    values: set(order, ["opening", "ending"]),
                    origin: None,
                },
            ),
        ],
    );
    insert_entries_str(
        &mut schema.speakers,
        order,
        [
            ("hazel", SpeakerDefinition { display_name: None }),
            (
                "rhea",
                SpeakerDefinition {
                    display_name: Some("Rhea".to_owned()),
                },
            ),
        ],
    );
    insert_entries_str(
        &mut schema.conditions,
        order,
        [
            (
                "thread_stage",
                ConditionDefinition {
                    params: vec![param(
                        "thread_id",
                        SchemaTypeRef::Registry("thread".to_owned()),
                    )],
                    returns: ConditionReturnType::Enum("thread_stage".to_owned()),
                    availability_reason: None,
                },
            ),
            (
                "trust_gte",
                ConditionDefinition {
                    params: vec![
                        param("speaker", SchemaTypeRef::Speaker),
                        param("threshold", SchemaTypeRef::Int),
                    ],
                    returns: ConditionReturnType::Bool,
                    availability_reason: Some(ConditionAvailabilityReasonMapping {
                        reason: AvailabilityReasonId::new("trust_too_low")
                            .expect("valid reason id"),
                        args: map(
                            order,
                            [
                                (
                                    "subject",
                                    AvailabilityReasonArgBinding::ConditionParam(
                                        "speaker".to_owned(),
                                    ),
                                ),
                                (
                                    "threshold",
                                    AvailabilityReasonArgBinding::ConditionParam(
                                        "threshold".to_owned(),
                                    ),
                                ),
                            ],
                        ),
                    }),
                },
            ),
        ],
    );
    insert_entries(
        &mut schema.availability_reasons,
        order,
        [
            (
                AvailabilityReasonId::new("trust_too_low").expect("valid reason id"),
                AvailabilityReasonDefinition {
                    template: "{subject} needs {threshold} trust.".to_owned(),
                    params: vec![
                        param("subject", SchemaTypeRef::Speaker),
                        param("threshold", SchemaTypeRef::Int),
                    ],
                    origin: Some("schema/reasons.rs".to_owned()),
                },
            ),
            (
                AvailabilityReasonId::new("need_key").expect("valid reason id"),
                AvailabilityReasonDefinition {
                    template: "Needs a key.".to_owned(),
                    params: Vec::new(),
                    origin: None,
                },
            ),
        ],
    );
    insert_entries_str(
        &mut schema.effects,
        order,
        [
            (
                "play_sfx",
                EffectDefinition {
                    modes: effect_modes(order, [EffectMode::Deferred, EffectMode::Immediate]),
                    params: vec![param("sound", SchemaTypeRef::Registry("sound".to_owned()))],
                },
            ),
            (
                "mark_mood",
                EffectDefinition {
                    modes: effect_modes(order, [EffectMode::Deferred]),
                    params: vec![param("mood", SchemaTypeRef::Enum("mood".to_owned()))],
                },
            ),
        ],
    );
    insert_entries_str(
        &mut schema.metadata_domains,
        order,
        [
            (
                "portrait_all",
                MetadataDomainDefinition::Flat(FlatMetadataDomain {
                    values: set(order, ["flat", "neutral"]),
                }),
            ),
            (
                "portrait_by_speaker",
                MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
                    selector: MetadataContextSelector::FieldSpeaker,
                    values_by_context: map(
                        order,
                        [
                            ("hazel", set(order, ["flat", "neutral"])),
                            ("rhea", set(order, ["flat"])),
                        ],
                    ),
                    missing_context: MissingMetadataContextPolicy::Fallback {
                        domain: "portrait_all".to_owned(),
                    },
                }),
            ),
        ],
    );
    insert_entries_str(
        &mut schema.metadata,
        order,
        [
            (
                "sfx",
                MetadataDefinition {
                    targets: targets(order, [MetadataTarget::Line, MetadataTarget::Choice]),
                    type_ref: SchemaTypeRef::Registry("sound".to_owned()),
                    repeatable: true,
                    domain: None,
                },
            ),
            (
                "portrait",
                MetadataDefinition {
                    targets: targets(order, [MetadataTarget::Block, MetadataTarget::Line]),
                    type_ref: SchemaTypeRef::Symbol,
                    repeatable: false,
                    domain: Some("portrait_by_speaker".to_owned()),
                },
            ),
        ],
    );
    insert_entries_str(
        &mut schema.projection_queries,
        order,
        [(
            "actor_skill",
            ProjectionQueryFunctionDefinition {
                params: vec![param("skill", SchemaTypeRef::String)],
                returns: SchemaTypeRef::Int,
                max_calls_per_event: Some(1),
            },
        )],
    );
    insert_entries_str(
        &mut schema.presentation_projectors,
        order,
        [(
            "choice_skill_prefix",
            SchemaPresentationProjectorDefinition {
                candidates: SchemaProjectionSelector::MetadataKey {
                    target: MetadataTarget::Choice,
                    key: "sfx".to_owned(),
                },
                inputs: vec![ProjectionInput {
                    name: "skill".to_owned(),
                    source: SchemaProjectionInputSource::Literal(SchemaLiteralValue::String(
                        "speech".to_owned(),
                    )),
                    type_ref: SchemaTypeRef::String,
                    required: true,
                }],
                queries: map(
                    order,
                    [(
                        "current",
                        ProjectionQueryDefinition {
                            function: "actor_skill".to_owned(),
                            args: vec![ProjectionInputRef::Input {
                                name: "skill".to_owned(),
                            }],
                        },
                    )],
                ),
                outputs: map(
                    order,
                    [(
                        "prefix",
                        PresentationAffordanceOutputDefinition {
                            target: ProjectionOutputTarget::Candidate,
                            kind: "badge".to_owned(),
                            slot: "prefix".to_owned(),
                            label: Some(PresentationLabelDefinition {
                                template_id: "skill_check_prefix".to_owned(),
                                source_text: "[{skill} {current}]".to_owned(),
                                args: map(
                                    order,
                                    [
                                        (
                                            "skill",
                                            PresentationLabelArgDefinition {
                                                source: ProjectionInputRef::Input {
                                                    name: "skill".to_owned(),
                                                },
                                                type_ref: SchemaTypeRef::String,
                                            },
                                        ),
                                        (
                                            "current",
                                            PresentationLabelArgDefinition {
                                                source: ProjectionInputRef::QueryResult {
                                                    name: "current".to_owned(),
                                                },
                                                type_ref: SchemaTypeRef::Int,
                                            },
                                        ),
                                    ],
                                ),
                            }),
                            fields: map(
                                order,
                                [(
                                    "current",
                                    PresentationAffordanceFieldDefinition {
                                        source: PresentationAffordanceFieldSource::QueryResult {
                                            name: "current".to_owned(),
                                        },
                                        type_ref: SchemaTypeRef::Int,
                                    },
                                )],
                            ),
                        },
                    )],
                ),
            },
        )],
    );
    insert_entries_str(
        &mut schema.markup,
        order,
        [
            (
                "slow",
                MarkupDefinition {
                    requires_closing: true,
                    translatable: true,
                    allows_nesting: true,
                },
            ),
            (
                "shake",
                MarkupDefinition {
                    requires_closing: true,
                    translatable: false,
                    allows_nesting: false,
                },
            ),
        ],
    );

    schema
}

fn insert_entries<K, T, const N: usize>(
    map: &mut BTreeMap<K, T>,
    order: Order,
    entries: [(K, T); N],
) where
    K: Ord,
{
    let mut entries = entries.into_iter().collect::<Vec<_>>();
    if matches!(order, Order::Reverse) {
        entries.reverse();
    }

    for (key, value) in entries {
        map.insert(key, value);
    }
}

fn insert_entries_str<T, const N: usize>(
    map: &mut BTreeMap<String, T>,
    order: Order,
    entries: [(&str, T); N],
) {
    insert_entries(
        map,
        order,
        entries.map(|(key, value)| (key.to_owned(), value)),
    );
}

fn set<const N: usize>(order: Order, values: [&str; N]) -> BTreeSet<String> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    if matches!(order, Order::Reverse) {
        values.reverse();
    }

    values.into_iter().map(str::to_owned).collect()
}

fn map<T, const N: usize>(order: Order, entries: [(&str, T); N]) -> BTreeMap<String, T> {
    let mut entries = entries.into_iter().collect::<Vec<_>>();
    if matches!(order, Order::Reverse) {
        entries.reverse();
    }

    entries
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

fn effect_modes<const N: usize>(order: Order, values: [EffectMode; N]) -> BTreeSet<EffectMode> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    if matches!(order, Order::Reverse) {
        values.reverse();
    }

    values.into_iter().collect()
}

fn targets<const N: usize>(order: Order, values: [MetadataTarget; N]) -> BTreeSet<MetadataTarget> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    if matches!(order, Order::Reverse) {
        values.reverse();
    }

    values.into_iter().collect()
}

fn param(name: &str, type_ref: SchemaTypeRef) -> ParameterDefinition {
    ParameterDefinition {
        name: name.to_owned(),
        type_ref,
    }
}
