use std::collections::BTreeMap;

use super::super::schema_index::SchemaIndex;

pub(super) fn carry_schema_authorities(
    old_schemas: &BTreeMap<String, SchemaIndex>,
    schemas: &mut BTreeMap<String, SchemaIndex>,
) {
    for schema in schemas.values_mut() {
        let Some(target) = schema.target_identity() else {
            continue;
        };
        let Some(protocol_uri) = old_schemas
            .values()
            .filter(|old| old.target_identity().as_deref() == Some(target.as_str()))
            .filter_map(SchemaIndex::protocol_uri)
            .min_by_key(|uri| uri.as_str().to_owned())
        else {
            continue;
        };
        *schema = std::mem::replace(schema, SchemaIndex::empty()).with_protocol_uri(protocol_uri);
    }
}
