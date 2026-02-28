use chapa::bitfield;

#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MsbReg {
    #[bits(0..=3)]
    high: u8,
    #[bits(28..=31)]
    low: u8,
}

#[test]
fn msb0_high_field() {
    // MSB-0: bits 0..=3 -> physical shift 28, mask 0xF0000000
    let r = MsbReg::new().with_high(0xA);
    assert_eq!(r.raw(), 0xA000_0000);
    assert_eq!(r.high(), 0xA);
}

#[test]
fn msb0_low_field() {
    // MSB-0: bits 28..=31 -> physical shift 0, mask 0x0000000F
    let r = MsbReg::new().with_low(0x5);
    assert_eq!(r.raw(), 0x0000_0005);
    assert_eq!(r.low(), 0x5);
}

#[test]
fn msb0_round_trip() {
    let r = MsbReg::new().with_high(0xC).with_low(0x3);
    assert_eq!(r.raw(), 0xC000_0003);
    assert_eq!(r.high(), 0xC);
    assert_eq!(r.low(), 0x3);
}

#[test]
fn msb0_from_raw() {
    let r = MsbReg::from_raw(0xF000_000F);
    assert_eq!(r.high(), 0xF);
    assert_eq!(r.low(), 0xF);
}

#[test]
fn msb0_consts() {
    assert_eq!(MsbReg::HIGH_SHIFT, 28);
    assert_eq!(MsbReg::HIGH_MASK, 0xF000_0000);
    assert_eq!(MsbReg::LOW_SHIFT, 0);
    assert_eq!(MsbReg::LOW_MASK, 0x0000_000F);
}
