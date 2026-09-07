pub(super) fn messagepack_asset_format_versions(bytes: &[u8]) -> Option<(u16, u16)> {
    let (item_count, mut offset) = messagepack_array_len(bytes)?;
    if item_count == 0 {
        return None;
    }
    let (header_count, header_offset) = messagepack_array_len(&bytes[offset..])?;
    if header_count < 2 {
        return None;
    }
    offset += header_offset;

    let format_version = messagepack_u16(bytes, &mut offset)?;
    let compiler_compatibility_version = messagepack_u16(bytes, &mut offset)?;
    Some((format_version, compiler_compatibility_version))
}

/// Read a MessagePack array length and the number of bytes consumed by its header.
pub fn messagepack_array_len(bytes: &[u8]) -> Option<(u32, usize)> {
    let marker = *bytes.first()?;
    match marker {
        0x90..=0x9f => Some((u32::from(marker & 0x0f), 1)),
        0xdc => Some((u32::from(u16::from_be_bytes(read_bytes(bytes, 1)?)), 3)),
        0xdd => Some((u32::from_be_bytes(read_bytes(bytes, 1)?), 5)),
        _ => None,
    }
}

/// Read a MessagePack unsigned integer that fits in a `u16`.
pub fn messagepack_u16(bytes: &[u8], offset: &mut usize) -> Option<u16> {
    let marker = *bytes.get(*offset)?;
    *offset += 1;

    match marker {
        0x00..=0x7f => Some(u16::from(marker)),
        0xcc => {
            let value = *bytes.get(*offset)?;
            *offset += 1;
            Some(u16::from(value))
        }
        0xcd => {
            let value = u16::from_be_bytes(read_bytes(bytes, *offset)?);
            *offset += 2;
            Some(value)
        }
        _ => None,
    }
}

fn read_bytes<const N: usize>(bytes: &[u8], offset: usize) -> Option<[u8; N]> {
    let end = offset.checked_add(N)?;
    bytes.get(offset..end)?.try_into().ok()
}
