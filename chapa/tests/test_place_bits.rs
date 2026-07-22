use chapa::{bitfield, place_bits};

// Explicit LSB0 forms (bit 0 is the LSB).

#[test]
fn lsb0_byte_lane() {
    // Write byte 0xAB into bits 8..=15 of a zeroed u32.
    let reg = place_bits!(lsb0 u32; 0u32; 8..=15; 0xABu8);
    assert_eq!(reg, 0x0000_AB00);
}

#[test]
fn lsb0_preserves_other_bits() {
    // Only bits 8..=15 change; the rest of `dst` is preserved.
    let reg = place_bits!(lsb0 u32; 0x1234_5678u32; 8..=15; 0xABu8);
    assert_eq!(reg, 0x1234_AB78);
}

#[test]
fn lsb0_single_bit() {
    let reg = place_bits!(lsb0 u8; 0u8; 3; 1u8);
    assert_eq!(reg, 0b0000_1000);
}

#[test]
fn lsb0_low_nibble() {
    let reg = place_bits!(lsb0 u16; 0xFFFFu16; 0..=3; 0xAu8);
    assert_eq!(reg, 0xFFFA);
}

#[test]
fn lsb0_truncates_wide_value() {
    // A value wider than the 8-bit range is truncated to the range width.
    let reg = place_bits!(lsb0 u32; 0u32; 8..=15; 0x1_2345u32);
    assert_eq!(reg, 0x0000_4500);
}

// Explicit MSB0 forms (bit 0 is the MSB).

#[test]
fn msb0_byte_lane() {
    // Bits 8..=15 counted from the MSB of a u32 -> physical bits 16..=23.
    let reg = place_bits!(msb0 u32; 0u32; 8..=15; 0xABu8);
    assert_eq!(reg, 0x00AB_0000);
}

#[test]
fn msb0_single_bit() {
    // Bit 0 (MSB) of a u8.
    let reg = place_bits!(msb0 u8; 0u8; 0; 1u8);
    assert_eq!(reg, 0x80);
}

#[test]
fn msb0_range() {
    // Bits 5..=9 (MSB0) of a u32 -> 0x07C0_0000.
    let reg = place_bits!(msb0 u32; 0u32; 5..=9; 0x1Fu8);
    assert_eq!(reg, 0x07C0_0000);
}

#[test]
fn msb0_truncates_wide_value() {
    let reg = place_bits!(msb0 u32; 0u32; 8..=15; 0xFFFFu32);
    assert_eq!(reg, 0x00FF_0000);
}

// The explicit form works in const contexts.

#[test]
fn explicit_form_is_const() {
    const REG: u32 = place_bits!(lsb0 u32; 0u32; 8..=15; 0xABu32);
    assert_eq!(REG, 0x0000_AB00);
}

// Bitfield form.

#[bitfield(u32, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct LsbReg {
    #[bits(0..=7)]
    lo: u8,
    #[bits(8..=15)]
    mid: u8,
    #[bits(16..=31)]
    hi: u16,
}

#[test]
fn struct_form_lsb0() {
    let reg = LsbReg::zeroed().with_hi(0xBEEF);
    let reg = place_bits!(reg; 8..=15; 0xABu8);
    assert_eq!(reg.mid(), 0xAB);
    assert_eq!(reg.hi(), 0xBEEF); // untouched
    assert_eq!(reg.raw(), 0xBEEF_AB00);
}

#[test]
fn struct_form_single_bit() {
    let reg = place_bits!(LsbReg::zeroed(); 0; 1u8);
    assert_eq!(reg.lo(), 0x01);
}

#[bitfield(u16, order = msb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct MsbReg {
    #[bits(0..=7)]
    hi: u8,
    #[bits(8..=15)]
    lo: u8,
}

#[test]
fn struct_form_msb0() {
    // MSB0 bits 0..=7 are the physical high byte.
    let reg = place_bits!(MsbReg::zeroed(); 0..=7; 0xCDu8);
    assert_eq!(reg.hi(), 0xCD);
    assert_eq!(reg.raw(), 0xCD00);
}
