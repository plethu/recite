use recite_core::TableRange;

pub(in crate::wire) fn range_to_u32<I: Copy>(
    range: TableRange<I>,
    index: impl Fn(I) -> u32,
) -> (u32, u32) {
    (index(range.start), range.len)
}

pub(in crate::wire) fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
