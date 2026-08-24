use chapa::{bitfield, FieldSegment};

/// ARM Thumb T3-style immediate: `imm16 = imm4:i:imm3:imm8`.
#[bitfield(u32, order = msb0)]
#[derive(Debug, PartialEq, Default)]
pub struct ArmMovT3 {
    #[bits(15..=12, 5, 19..=17, 31..=24, default = 0xABCD)]
    imm16: u16,
}

#[test]
fn arm_msb0_immediate_scatters_and_gathers_in_declared_order() {
    let instruction = ArmMovT3::zeroed().with_imm16(0xABCD);

    // 0xABCD = imm4(0xA) : i(1) : imm3(3) : imm8(0xCD).
    assert_eq!(instruction.raw(), 0x040A_30CD);
    assert_eq!(instruction.imm16(), 0xABCD);
    assert_eq!(ArmMovT3::IMM16_MASK, 0x040F_70FF);
    assert_eq!(ArmMovT3::IMM16_SHIFT, 0);
    assert_eq!(
        ArmMovT3::IMM16_SEGMENTS,
        &[
            FieldSegment {
                offset: 16,
                value_offset: 12,
                width: 4,
            },
            FieldSegment {
                offset: 26,
                value_offset: 11,
                width: 1,
            },
            FieldSegment {
                offset: 12,
                value_offset: 8,
                width: 3,
            },
            FieldSegment {
                offset: 0,
                value_offset: 0,
                width: 8,
            },
        ]
    );
}

#[test]
fn split_setter_preserves_unselected_bits() {
    let mut instruction = ArmMovT3::from_raw(u32::MAX);
    instruction.set_imm16(0);

    assert_eq!(instruction.raw(), !ArmMovT3::IMM16_MASK);
}

#[test]
fn split_primitive_accessors_remain_const() {
    const INSTRUCTION: ArmMovT3 = ArmMovT3::zeroed().with_imm16(0xABCD);
    const IMMEDIATE: u16 = INSTRUCTION.imm16();

    assert_eq!(INSTRUCTION.raw(), 0x040A_30CD);
    assert_eq!(IMMEDIATE, 0xABCD);
}

#[test]
fn split_default_uses_the_same_scatter_logic() {
    assert_eq!(ArmMovT3::default().raw(), 0x040A_30CD);
    assert_eq!(ArmMovT3::default().imm16(), 0xABCD);
}

#[bitfield(u32, order = lsb0)]
#[derive(Debug, PartialEq)]
pub struct LsbImmediate {
    // Half-open ranges also work. Declaration order, rather than physical
    // position or bit order, controls concatenation.
    #[bits(28..32, 20, 16..19, 0..8)]
    imm16: u16,
}

#[test]
fn lsb0_ranges_still_concatenate_most_significant_chunk_first() {
    let value = LsbImmediate::zeroed().with_imm16(0xABCD);

    assert_eq!(value.raw(), 0xA013_00CD);
    assert_eq!(value.imm16(), 0xABCD);
}

#[bitfield(u16, order = lsb0)]
#[derive(Debug, PartialEq)]
pub struct DescendingRanges {
    #[bits(10..=0)]
    inclusive: u16,

    #[bits(15..11)]
    half_open: u8,
}

#[test]
fn descending_endpoints_select_the_same_bits_as_ascending_endpoints() {
    let inclusive = DescendingRanges::zeroed().with_inclusive(0x5A5);
    assert_eq!(inclusive.raw(), 0x05A5);
    assert_eq!(inclusive.inclusive(), 0x5A5);

    // The endpoint remains exclusive: 15..11 selects bits 15 through 12.
    let half_open = DescendingRanges::zeroed().with_half_open(0xA);
    assert_eq!(half_open.raw(), 0xA000);
    assert_eq!(half_open.half_open(), 0xA);
}

#[bitfield(u16, order = lsb0)]
#[derive(Debug, PartialEq)]
pub struct SplitSigned {
    #[bits(12..=14, 0..=4)]
    value: i8,
}

#[test]
fn signed_split_field_sign_extends_after_gathering() {
    let negative = SplitSigned::zeroed().with_value(-2);
    assert_eq!(negative.raw(), 0x701E);
    assert_eq!(negative.value(), -2);

    let positive = SplitSigned::zeroed().with_value(62);
    assert_eq!(positive.raw(), 0x101E);
    assert_eq!(positive.value(), 62);
}

#[bitfield(u8, order = lsb0, width = 4)]
#[derive(Debug, PartialEq)]
pub struct Nibble {
    #[bits(0..=3)]
    value: u8,
}

#[bitfield(u16, order = lsb0)]
#[derive(Debug, PartialEq)]
pub struct SplitNested {
    #[bits(12..=13, 0..=1)]
    nibble: Nibble,
}

#[test]
fn nested_split_field_round_trips() {
    let nibble = Nibble::zeroed().with_value(0b1011);
    let parent = SplitNested::zeroed().with_nibble(nibble);

    assert_eq!(parent.raw(), 0x2003);
    assert_eq!(parent.nibble(), nibble);
}
