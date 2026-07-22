use chapa::bitfield;

#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ByteReg {
    #[bits(0..=3)]
    low: u8,
    #[bits(4..=7)]
    high: u8,
}

#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WordReg {
    #[bits(0..=7)]
    low: u8,
    #[bits(8..=15)]
    high: u8,
}

#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Msb0Reg {
    #[bits(0..=7)]
    opcode: u8,
    #[bits(8..=31)]
    payload: u32,
}

#[test]
fn wrapping_add_wraps_at_storage_width() {
    let r = ByteReg::from_raw(0xFF).wrapping_add(1);
    assert_eq!(r.raw(), 0x00);

    let r = WordReg::from_raw(0xFFFF).wrapping_add(0x10);
    assert_eq!(r.raw(), 0x000F);
}

#[test]
fn wrapping_sub_wraps_at_storage_width() {
    let r = ByteReg::from_raw(0x00).wrapping_sub(1);
    assert_eq!(r.raw(), 0xFF);

    let r = WordReg::from_raw(0x0005).wrapping_sub(0x10);
    assert_eq!(r.raw(), 0xFFF5);
}

#[test]
fn carries_cross_field_boundaries() {
    // low is bits 0..=3; adding 1 to 0x0F carries into high.
    let r = ByteReg::from_raw(0x0F).wrapping_add(1);
    assert_eq!(r.low(), 0x0);
    assert_eq!(r.high(), 0x1);
}

#[test]
fn saturating_clamps_at_storage_bounds() {
    assert_eq!(ByteReg::from_raw(0xF0).saturating_add(0x20).raw(), 0xFF);
    assert_eq!(ByteReg::from_raw(0x05).saturating_sub(0x10).raw(), 0x00);
    assert_eq!(WordReg::from_raw(0x1234).saturating_add(1).raw(), 0x1235);
}

#[test]
fn checked_returns_none_on_overflow() {
    assert_eq!(ByteReg::from_raw(0xFF).checked_add(1), None);
    assert_eq!(ByteReg::from_raw(0x00).checked_sub(1), None);
    assert_eq!(
        ByteReg::from_raw(0xFE).checked_add(1),
        Some(ByteReg::from_raw(0xFF))
    );
    assert_eq!(
        WordReg::from_raw(0x0001).checked_sub(1),
        Some(WordReg::from_raw(0x0000))
    );
}

#[test]
fn overflowing_reports_carry_and_borrow() {
    let (r, overflowed) = ByteReg::from_raw(0xFF).overflowing_add(2);
    assert_eq!(r.raw(), 0x01);
    assert!(overflowed);

    let (r, overflowed) = ByteReg::from_raw(0x01).overflowing_sub(2);
    assert_eq!(r.raw(), 0xFF);
    assert!(overflowed);

    let (r, overflowed) = WordReg::from_raw(0x1234).overflowing_add(1);
    assert_eq!(r.raw(), 0x1235);
    assert!(!overflowed);
}

#[test]
fn msb0_operates_on_storage() {
    // Bit ordering affects field extraction, not arithmetic: the ops act on
    // the raw storage value.
    let r = Msb0Reg::from_raw(0xFFFF_FFFF).wrapping_add(1);
    assert_eq!(r.raw(), 0x0000_0000);

    let r = Msb0Reg::from_raw(0xDEAD_BEEF).wrapping_sub(0xEF);
    assert_eq!(r.raw(), 0xDEAD_BE00);
}

#[test]
fn const_context() {
    const R: WordReg = WordReg::from_raw(0xFFFF).wrapping_add(1);
    assert_eq!(R.raw(), 0x0000);

    const S: Option<WordReg> = WordReg::from_raw(0xFFFF).checked_add(1);
    assert!(S.is_none());

    const T: (WordReg, bool) = WordReg::from_raw(0xFFFF).overflowing_add(1);
    assert_eq!(T.0.raw(), 0x0000);
    assert!(T.1);
}
