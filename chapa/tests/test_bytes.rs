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
fn u8_single_byte() {
    let r = ByteReg::zeroed().with_low(0x2).with_high(0xA);
    assert_eq!(r.to_le_bytes(), [0xA2]);
    assert_eq!(r.to_be_bytes(), [0xA2]);
    assert_eq!(ByteReg::from_le_bytes([0xA2]), r);
    assert_eq!(ByteReg::from_be_bytes([0xA2]), r);
}

#[test]
fn u16_known_values() {
    let r = WordReg::zeroed().with_low(0xCD).with_high(0xAB);
    assert_eq!(r.raw(), 0xABCD);
    assert_eq!(r.to_le_bytes(), [0xCD, 0xAB]);
    assert_eq!(r.to_be_bytes(), [0xAB, 0xCD]);
    assert_eq!(WordReg::from_le_bytes([0xCD, 0xAB]), r);
    assert_eq!(WordReg::from_be_bytes([0xAB, 0xCD]), r);
}

#[test]
fn le_be_are_reversed() {
    let r = WordReg::from_raw(0x1234);
    let mut le = r.to_le_bytes();
    le.reverse();
    assert_eq!(le, r.to_be_bytes());
}

#[test]
fn round_trips() {
    let r = WordReg::from_raw(0xBEEF);
    assert_eq!(WordReg::from_le_bytes(r.to_le_bytes()).raw(), r.raw());
    assert_eq!(WordReg::from_be_bytes(r.to_be_bytes()).raw(), r.raw());
    assert_eq!(WordReg::from_ne_bytes(r.to_ne_bytes()).raw(), r.raw());
}

#[test]
fn ne_matches_storage() {
    let r = WordReg::from_raw(0x1234);
    assert_eq!(r.to_ne_bytes(), 0x1234u16.to_ne_bytes());
}

#[test]
fn msb0_operates_on_storage() {
    // Bit ordering affects field extraction, not byte conversion: the
    // conversions operate on the raw storage value.
    let r = Msb0Reg::from_raw(0xDEADBEEF);
    assert_eq!(r.to_be_bytes(), [0xDE, 0xAD, 0xBE, 0xEF]);
    assert_eq!(r.to_le_bytes(), [0xEF, 0xBE, 0xAD, 0xDE]);
    assert_eq!(
        Msb0Reg::from_be_bytes([0xDE, 0xAD, 0xBE, 0xEF]).raw(),
        0xDEADBEEF
    );
}

#[test]
fn const_context() {
    const R: WordReg = WordReg::from_le_bytes([0xCD, 0xAB]);
    assert_eq!(R.raw(), 0xABCD);
    const BYTES: [u8; 2] = R.to_be_bytes();
    assert_eq!(BYTES, [0xAB, 0xCD]);
}
