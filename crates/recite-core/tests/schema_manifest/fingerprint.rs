use std::collections::{BTreeMap, BTreeSet};

use recite_core::{
    BLAKE3_DIGEST_LEN, ConditionDefinition, ConditionReturnType, ContextualMetadataDomain,
    EffectDefinition, EffectMode, EnumTypeDefinition, FlatMetadataDomain, MarkupDefinition,
    MetadataContextSelector, MetadataDefinition, MetadataDomainDefinition, MetadataTarget,
    MissingMetadataContextPolicy, ParameterDefinition, ProjectSchema, RegistryDefinition,
    SchemaFingerprint, SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition,
    canonical_schema_fingerprint,
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
}

#[derive(Clone, Copy)]
enum Order {
    Forward,
    Reverse,
}

fn schema_with_order(order: Order) -> ProjectSchema {
    let mut schema = ProjectSchema::empty_v1();

    insert_entries(
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
    insert_entries(
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
    insert_entries(
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
    insert_entries(
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
                },
            ),
        ],
    );
    insert_entries(
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
    insert_entries(
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
    insert_entries(
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
    insert_entries(
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

fn insert_entries<T, const N: usize>(
    map: &mut BTreeMap<String, T>,
    order: Order,
    entries: [(&str, T); N],
) {
    let mut entries = entries.into_iter().collect::<Vec<_>>();
    if matches!(order, Order::Reverse) {
        entries.reverse();
    }

    for (key, value) in entries {
        map.insert(key.to_owned(), value);
    }
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
