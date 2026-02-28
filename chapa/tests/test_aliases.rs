use chapa::bitfield;

#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AliasReg {
    #[bits(0..=3, alias = "lo")]
    low: u8,
    #[bits(4..=7, alias = ["hi", "upper"])]
    high: u8,
}

#[test]
fn single_alias_getter() {
    let r = AliasReg::new().with_low(5);
    assert_eq!(r.lo(), 5);
}

#[test]
fn single_alias_setter() {
    let mut r = AliasReg::new();
    r.set_lo(0xA);
    assert_eq!(r.low(), 0xA);
}

#[test]
fn single_alias_builder() {
    let r = AliasReg::new().with_lo(0xC);
    assert_eq!(r.low(), 0xC);
}

#[test]
fn multi_alias() {
    let r = AliasReg::new().with_hi(0xD);
    assert_eq!(r.high(), 0xD);
    assert_eq!(r.hi(), 0xD);
    assert_eq!(r.upper(), 0xD);
}

#[test]
fn multi_alias_setter() {
    let mut r = AliasReg::new();
    r.set_upper(0xF);
    assert_eq!(r.high(), 0xF);
    assert_eq!(r.hi(), 0xF);
}
