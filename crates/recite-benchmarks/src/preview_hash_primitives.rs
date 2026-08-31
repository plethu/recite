pub(super) fn tag(hasher: &mut blake3::Hasher, value: u8) {
    hasher.update(&[value]);
}

pub(super) fn hash_u64(hasher: &mut blake3::Hasher, value: u64) {
    hasher.update(&value.to_le_bytes());
}

pub(super) fn hash_i64(hasher: &mut blake3::Hasher, value: i64) {
    hasher.update(&value.to_le_bytes());
}

pub(super) fn hash_len(hasher: &mut blake3::Hasher, value: usize) {
    hash_u64(hasher, value as u64);
}

pub(super) fn hash_bool(hasher: &mut blake3::Hasher, value: bool) {
    tag(hasher, u8::from(value));
}

pub(super) fn hash_text(hasher: &mut blake3::Hasher, value: &str) {
    hash_len(hasher, value.len());
    hasher.update(value.as_bytes());
}

pub(super) fn hash_optional_text(hasher: &mut blake3::Hasher, value: Option<&str>) {
    if let Some(value) = value {
        tag(hasher, 1);
        hash_text(hasher, value);
    } else {
        tag(hasher, 0);
    }
}
