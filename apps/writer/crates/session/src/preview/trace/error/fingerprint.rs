use super::Field;
use recite_runtime::snapshot::DialogueSchemaFingerprintSnapshot;

pub(super) fn fields<'a>(
    expected: &'a DialogueSchemaFingerprintSnapshot,
    actual: &'a DialogueSchemaFingerprintSnapshot,
) -> Vec<Field<'a>> {
    let mut fields = Vec::with_capacity(4);
    for (fingerprint, schema, algorithm, digest) in [
        (
            expected,
            "Expected schema",
            "Expected schema algorithm",
            "Expected schema digest",
        ),
        (
            actual,
            "Actual schema",
            "Actual schema algorithm",
            "Actual schema digest",
        ),
    ] {
        match fingerprint {
            DialogueSchemaFingerprintSnapshot::NoSchema => {
                fields.push(Field::text(schema, "No schema"))
            }
            DialogueSchemaFingerprintSnapshot::Fingerprint(fingerprint) => {
                fields.push(Field::text(algorithm, fingerprint.algorithm.as_str()));
                fields.push(Field::hex(digest, &fingerprint.digest));
            }
        }
    }
    fields
}
