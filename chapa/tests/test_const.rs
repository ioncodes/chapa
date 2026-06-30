use chapa::bitfield;

#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ConstReg {
    #[bits(0..=7)]
    low: u8,
    #[bits(8..=15)]
    high: u8,
}

#[test]
fn const_new() {
    const R: ConstReg = ConstReg::zeroed();
    assert_eq!(R.raw(), 0);
}

#[test]
fn const_from_raw() {
    const R: ConstReg = ConstReg::from_raw(0xABCD);
    assert_eq!(R.raw(), 0xABCD);
}

#[test]
fn const_with() {
    const R: ConstReg = ConstReg::zeroed().with_low(0x12).with_high(0x34);
    assert_eq!(R.raw(), 0x3412);
}

#[test]
fn const_getter() {
    const R: ConstReg = ConstReg::from_raw(0xABCD);
    const LOW: u8 = R.low();
    const HIGH: u8 = R.high();
    assert_eq!(LOW, 0xCD);
    assert_eq!(HIGH, 0xAB);
}

#[test]
fn const_set() {
    const R: ConstReg = {
        let mut r = ConstReg::zeroed();
        r.set_low(0x12);
        r.set_high(0x34);
        r
    };
    assert_eq!(R.raw(), 0x3412);
}
