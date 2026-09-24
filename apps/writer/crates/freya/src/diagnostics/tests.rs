use super::*;
#[test]
fn positions_count_characters_and_clamp_stale_locations() -> Result<(), Box<dyn std::error::Error>>
{
    let rope = Rope::from_str("één 🐉\nCymraeg");
    assert_eq!(position_offset(&rope, SourcePosition::new(2, 2)?), 8);
    assert_eq!(
        position_offset(&rope, SourcePosition::new(50, 90)?),
        rope.len_utf16_cu()
    );
    Ok(())
}
