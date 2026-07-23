use chapa::{bitfield, extract_bits, insert_bits};

// Explicit LSB0 forms (bit 0 is the LSB).

#[test]
fn lsb0_multi_range_merge() {
    // Replace bits 0..=3 and 8..=15 of `dst` with those of `src`.
    let dst: u32 = 0x1234_5678;
    let src: u32 = 0xFFFF_FFFF;
    let merged = insert_bits!(lsb0 u32; dst; 0..=3, 8..=15; src);
    assert_eq!(merged, 0x1234_FF7F);
}

#[test]
fn lsb0_only_masked_src_bits_used() {
    // Bits of `src` outside the selected ranges are ignored.
    let merged = insert_bits!(lsb0 u32; 0u32; 8..=15; 0xFFFF_FFFFu32);
    assert_eq!(merged, 0x0000_FF00);
}

#[test]
fn lsb0_single_bit() {
    let merged = insert_bits!(lsb0 u8; 0u8; 3; 0xFFu8);
    assert_eq!(merged, 0b0000_1000);
}

#[test]
fn lsb0_trailing_comma() {
    let a = insert_bits!(lsb0 u16; 0u16; 0..=3, 12..=15; 0xFFFFu16);
    let b = insert_bits!(lsb0 u16; 0u16; 0..=3, 12..=15; 0xFFFFu16);
    assert_eq!(a, b);
    assert_eq!(a, 0xF00F);
}

// Explicit MSB0 forms (bit 0 is the MSB).

#[test]
fn msb0_single_and_range() {
    // Bit 0 (MSB) plus bits 16..=31.
    let merged = insert_bits!(msb0 u32; 0u32; 0, 16..=31; 0xFFFF_FFFFu32);
    assert_eq!(merged, 0x8000_FFFF);
}

#[test]
fn msb0_preserves_unselected_bits() {
    let dst: u32 = 0x1234_5678;
    let merged = insert_bits!(msb0 u32; dst; 0, 16..=31; 0u32);
    // Selected bits cleared (src is 0), the rest preserved.
    assert_eq!(merged, 0x1234_5678 & !0x8000_FFFF);
}

// The explicit form works in const contexts.

#[test]
fn explicit_form_is_const() {
    const MERGED: u32 = insert_bits!(lsb0 u32; 0x1234_5678u32; 0..=7; 0xFFu32);
    assert_eq!(MERGED, 0x1234_56FF);
}

// Using insert_bits! with extract_bits!.

#[test]
fn round_trip_with_extract_bits() {
    let dst: u32 = 0xAAAA_AAAA;
    let src: u32 = 0x1234_5678;
    // Inserting the extracted bits keeps `src` on the range and `dst` elsewhere.
    let merged = insert_bits!(lsb0 u32; dst; 8..=23; extract_bits!(lsb0 u32; src; 8..=23));
    assert_eq!(merged & 0x00FF_FF00, src & 0x00FF_FF00); // src on the range
    assert_eq!(merged & !0x00FF_FF00, dst & !0x00FF_FF00); // dst elsewhere
}

// Bitfield form.

#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct Reg {
    #[bits(0..=7)]
    a: u8,
    #[bits(8..=31)]
    b: u32,
}

#[test]
fn struct_form_replaces_range() {
    let reg = Reg::from_raw(0x1234_5678);
    // MSB0 bits 0..=7 are the physical high byte.
    let updated = insert_bits!(reg; 0..=7; 0xFF00_0000u32);
    assert_eq!(updated.raw(), 0xFF34_5678);
    assert_eq!(updated.a(), 0xFF);
    assert_eq!(updated.b(), reg.b()); // untouched
}

#[test]
fn struct_form_widens_narrow_src() {
    // A narrower `src` is zero-extended to the storage type via `as _`.
    let reg = Reg::from_raw(0xFFFF_FFFF);
    let updated = insert_bits!(reg; 24..=31; 0u8);
    assert_eq!(updated.raw(), 0xFFFF_FF00);
}

#[test]
fn runtime_bits_and_ranges() {
    let offset = 8u8;
    let bit = 31u8;
    assert_eq!(
        insert_bits!(lsb0 u32; 0u32; offset..offset + 8, bit; u32::MAX),
        0x8000_FF00,
    );

    let reg = Reg::from_raw(0);
    let updated = insert_bits!(reg; offset..offset + 8; 0x00FF_0000u32);
    assert_eq!(updated.raw(), 0x00FF_0000);
}

#[test]
fn empty_half_open_range_changes_nothing() {
    let offset = 0u8;
    assert_eq!(
        insert_bits!(lsb0 u32; 0x1234_5678u32; 0..0; u32::MAX),
        0x1234_5678,
    );

    let reg = Reg::from_raw(0x1234_5678);
    let updated = insert_bits!(reg; offset..offset; u32::MAX);
    assert_eq!(updated.raw(), reg.raw());
}
