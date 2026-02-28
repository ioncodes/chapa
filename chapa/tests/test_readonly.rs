use chapa::bitfield;

#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ReadonlyReg {
    #[bits(0..=3, readonly)]
    status: u8,
    #[bits(4..=7)]
    _reserved: u8,
    // Note: _reserved uses underscore prefix -> readonly
}

#[test]
fn readonly_getter_works() {
    let r = ReadonlyReg::from_raw(0xAB);
    assert_eq!(r.status(), 0xB);
}

#[test]
fn underscore_prefix_readonly() {
    let r = ReadonlyReg::from_raw(0xAB);
    assert_eq!(r.reserved(), 0xA);
}
