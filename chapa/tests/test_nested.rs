use chapa::bitfield;

#[bitfield(u8, order = msb0, width = 4)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Child {
    #[bits(0..=1)]
    high: u8,
    #[bits(2..=3)]
    low: u8,
}

#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Parent {
    #[bits(0..=3)]
    child: Child,
    #[bits(4..=7)]
    other: u8,
    #[bits(28..=31)]
    tail: u8,
}

#[test]
fn nested_set_get() {
    let c = Child::new().with_high(2).with_low(1);
    assert_eq!(c.raw(), 0b1001);
    assert_eq!(c.high(), 2);
    assert_eq!(c.low(), 1);
}

#[test]
fn parent_with_child() {
    let c = Child::new().with_high(3).with_low(2);
    let p = Parent::new().with_child(c);
    // Child raw = 0b1110 = 0xE, placed at MSB-0 bits 0..=3 -> shift 28
    assert_eq!(p.raw(), 0xE000_0000);
    let got = p.child();
    assert_eq!(got.high(), 3);
    assert_eq!(got.low(), 2);
}

#[test]
fn parent_mixed() {
    let c = Child::new().with_high(1).with_low(0);
    let p = Parent::new().with_child(c).with_other(0xF).with_tail(0xA);
    let got_child = p.child();
    assert_eq!(got_child.high(), 1);
    assert_eq!(got_child.low(), 0);
    assert_eq!(p.other(), 0xF);
    assert_eq!(p.tail(), 0xA);
}

#[test]
fn nested_round_trip() {
    for i in 0u8..16 {
        let c = Child::from_raw(i);
        let p = Parent::new().with_child(c);
        let c2 = p.child();
        assert_eq!(c2.raw(), i);
    }
}
