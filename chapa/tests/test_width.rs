use chapa::bitfield;

#[bitfield(u8, order = msb0, width = 4)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Nibble {
    #[bits(0)]
    bit0: bool,
    #[bits(3)]
    bit3: bool,
    #[bits(1..=2)]
    mid: u8,
}

#[test]
fn width4_bit0_is_msb() {
    // MSB-0 width=4: bit 0 -> shift 3, mask 0x08
    let r = Nibble::new().with_bit0(true);
    assert_eq!(r.raw(), 0x08);
}

#[test]
fn width4_bit3_is_lsb() {
    // MSB-0 width=4: bit 3 -> shift 0, mask 0x01
    let r = Nibble::new().with_bit3(true);
    assert_eq!(r.raw(), 0x01);
}

#[test]
fn width4_mid() {
    let r = Nibble::new().with_mid(3);
    // bits 1..=2, MSB-0 width=4: shift 1, mask 0x06
    assert_eq!(r.raw(), 0x06);
    assert_eq!(r.mid(), 3);
}

#[test]
fn upper_bits_untouched() {
    // Start with upper bits set, ensure width-4 operations don't clear them
    let r = Nibble::from_raw(0xF0).with_bit0(true);
    assert_eq!(r.raw(), 0xF8); // upper nibble preserved
}

#[test]
fn consts() {
    assert_eq!(Nibble::BIT0_SHIFT, 3);
    assert_eq!(Nibble::BIT0_MASK, 0x08);
    assert_eq!(Nibble::BIT3_SHIFT, 0);
    assert_eq!(Nibble::BIT3_MASK, 0x01);
    assert_eq!(Nibble::MID_SHIFT, 1);
    assert_eq!(Nibble::MID_MASK, 0x06);
}
