use chapa::extract_bits;

// --- MSB0 tests (bit 0 = MSB of u32) ---

#[test]
fn msb0_single_bit() {
    // Keep only bit 0 (MSB): 1 << 31 = 0x8000_0000
    let masked = extract_bits!(msb0 u32; 0xFFFF_FFFFu32; 0);
    assert_eq!(masked, 0x8000_0000);
}

#[test]
fn msb0_range() {
    // Keep bits 5..=9: 0x07C0_0000
    let masked = extract_bits!(msb0 u32; 0xFFFF_FFFFu32; 5..=9);
    assert_eq!(masked, 0x07C0_0000);
}

#[test]
fn msb0_mixed_single_and_range() {
    // SRR1 <- MSR mapping: keep bits 0, 5-9, 16-31; clear bits 1-4 and 10-15
    let masked = extract_bits!(msb0 u32; 0xFFFF_FFFFu32; 0, 5..=9, 16..=31);
    assert_eq!(masked, !0x783F_0000u32);
}

#[test]
fn msb0_trailing_comma() {
    let a = extract_bits!(msb0 u32; 0xFFFF_FFFFu32; 0, 5..=9, 16..=31,);
    let b = extract_bits!(msb0 u32; 0xFFFF_FFFFu32; 0, 5..=9, 16..=31);
    assert_eq!(a, b);
}

#[test]
fn msb0_zeroes_cleared_bits() {
    // All-ones input: bits 1-4 and 10-15 should be zero
    let masked = extract_bits!(msb0 u32; 0xFFFF_FFFFu32; 0, 5..=9, 16..=31);
    assert_eq!(masked & 0x7800_0000, 0); // bits 1-4 cleared
    assert_eq!(masked & 0x003F_0000, 0); // bits 10-15 cleared
}

#[test]
fn msb0_u8() {
    // u8 is 8 bits wide; bit 0 = 0x80, bit 7 = 0x01
    let masked = extract_bits!(msb0 u8; 0xFFu8; 0, 7);
    assert_eq!(masked, 0x81);
}

#[test]
fn msb0_preserves_zero() {
    let masked = extract_bits!(msb0 u32; 0u32; 0, 5..=9, 16..=31);
    assert_eq!(masked, 0);
}

// --- LSB0 tests (bit 0 = LSB) ---

#[test]
fn lsb0_range() {
    // Keep bits 0..=3 (low nibble)
    let masked = extract_bits!(lsb0 u16; 0xABCDu16; 0..=3);
    assert_eq!(masked, 0x000D);
}

#[test]
fn lsb0_mixed() {
    // Keep bits 0..=3 and 12..=15
    let masked = extract_bits!(lsb0 u16; 0xFFFFu16; 0..=3, 12..=15);
    assert_eq!(masked, 0xF00F);
}

#[test]
fn lsb0_single_bit() {
    let masked = extract_bits!(lsb0 u8; 0xFFu8; 3);
    assert_eq!(masked, 1 << 3);
}