use super::model::PreviewState;

/// Stable, dependency-free digest of the persisted control projection.
///
/// The debug representation is produced from typed fields in declaration
/// order and is only used as a compact integrity witness inside the preview
/// snapshot. It is not a cryptographic authenticity mechanism.
pub(super) fn projection_fingerprint(state: &PreviewState) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in format!("{state:?}").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3_u64);
    }
    format!("{hash:016x}")
}
