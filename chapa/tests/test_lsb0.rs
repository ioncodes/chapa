use chapa::bitfield;

#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StatusReg {
    #[bits(0..=3)]
    low_nibble: u8,
    #[bits(12..=15)]
    high_nibble: u8,
}

#[test]
fn new_is_zero() {
    let r = StatusReg::new();
    assert_eq!(r.raw(), 0);
}

#[test]
fn with_low_nibble() {
    let r = StatusReg::new().with_low_nibble(0xA);
    assert_eq!(r.raw(), 0x000A);
    assert_eq!(r.low_nibble(), 0xA);
    assert_eq!(r.high_nibble(), 0);
}

#[test]
fn with_high_nibble() {
    let r = StatusReg::new().with_high_nibble(0xB);
    assert_eq!(r.raw(), 0xB000);
    assert_eq!(r.high_nibble(), 0xB);
    assert_eq!(r.low_nibble(), 0);
}

#[test]
fn round_trip() {
    let r = StatusReg::new().with_low_nibble(0x5).with_high_nibble(0xC);
    assert_eq!(r.raw(), 0xC005);
    assert_eq!(r.low_nibble(), 0x5);
    assert_eq!(r.high_nibble(), 0xC);
}

#[test]
fn from_raw() {
    let r = StatusReg::from_raw(0xF00F);
    assert_eq!(r.low_nibble(), 0xF);
    assert_eq!(r.high_nibble(), 0xF);
}

#[test]
fn set_mutate() {
    let mut r = StatusReg::new();
    r.set_low_nibble(3);
    r.set_high_nibble(7);
    assert_eq!(r.low_nibble(), 3);
    assert_eq!(r.high_nibble(), 7);
}

#[test]
fn all_ones() {
    let r = StatusReg::from_raw(0xFFFF);
    assert_eq!(r.low_nibble(), 0xF);
    assert_eq!(r.high_nibble(), 0xF);
}
