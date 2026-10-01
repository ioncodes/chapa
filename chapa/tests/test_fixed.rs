use chapa::{bitfield, extract_bits, insert_bits, place_bits, BitField};

#[bitfield(u8, order = lsb0)]
#[derive(Debug, PartialEq)]
pub struct Fixed {
    #[bits(0..=7, default = 0xFF)]
    data: u8,
    #[bits(2, fixed = true, alias = "required")]
    one: bool,
    #[bits(5..=6, fixed = 0)]
    zeros: u8,
}

fn normalize(raw: u8) -> u8 {
    (raw & !0x64) | 4
}

#[test]
fn constructors_and_validation() {
    const ZERO: Fixed = Fixed::zeroed();
    const RAW: Fixed = Fixed::from_raw(0xFF);
    const BUILT: Fixed = Fixed::zeroed().with_data(0xFF);
    const MUTATED: Fixed = {
        let mut r = Fixed::zeroed();
        r.set_data(0xFF);
        r
    };
    const SUM: Fixed = Fixed::zeroed().wrapping_add(0xFF);
    const BYTES: Fixed = Fixed::from_be_bytes([0xFF]);
    assert_eq!(ZERO.raw(), 4);
    assert_eq!(RAW.raw(), 0x9F);
    assert_eq!(BYTES, RAW);
    assert_eq!(BUILT, RAW);
    assert_eq!(MUTATED, RAW);
    assert_eq!(SUM.raw(), 7);
    assert_eq!(Fixed::default(), RAW);
    assert_eq!(Fixed::FIXED_MASK, 0x64);
    assert_eq!(Fixed::FIXED_VALUE, 4);
    for raw in 0..=u8::MAX {
        let expected = normalize(raw);
        let r = Fixed::from_raw(raw);
        assert_eq!(r.raw(), expected);
        assert_eq!(Fixed::from(raw), r);
        assert_eq!(<Fixed as BitField>::from_raw(raw), r);
        assert_eq!(Fixed::from_le_bytes([raw]), r);
        assert_eq!(Fixed::from_ne_bytes([raw]), r);
        assert_eq!(r.to_le_bytes(), [expected]);
        assert_eq!(r.to_be_bytes(), [expected]);
        assert_eq!(r.to_ne_bytes(), [expected]);
        assert_eq!(u8::from(r), expected);
        assert!(r.one());
        assert!(r.required());
        assert_eq!(r.zeros(), 0);
        match Fixed::try_from_raw(raw) {
            Ok(valid) => {
                assert_eq!(raw, expected);
                assert_eq!(valid, r);
            }
            Err(err) => {
                assert_ne!(raw, expected);
                assert_eq!(err.raw, raw);
            }
        }
    }
}

#[test]
fn overlapping_setters_and_helpers_preserve_constraints() {
    for raw in 0..=u8::MAX {
        let expected = normalize(raw);
        let mut r = Fixed::zeroed();
        r.set_data(raw);
        assert_eq!(r.raw(), expected);
        assert_eq!(Fixed::zeroed().with_data(raw).raw(), expected);
        assert_eq!(insert_bits!(Fixed::zeroed(); 0..=7; raw).raw(), expected);
        assert_eq!(place_bits!(Fixed::zeroed(); 0..=7; raw).raw(), expected);
        assert_eq!(extract_bits!(r; 0..=1).raw(), (expected & 3) | 4);
    }
}

#[test]
fn every_raw_operator_normalizes_but_preserves_overflow_reporting() {
    for raw in 0..=u8::MAX {
        let r = Fixed::from_raw(raw);
        let raw = r.raw();
        assert_eq!((!r).raw(), normalize(!raw));
        for rhs in 0..=u8::MAX {
            assert_eq!((r & rhs).raw(), normalize(raw & rhs));
            assert_eq!((r | rhs).raw(), normalize(raw | rhs));
            assert_eq!((r ^ rhs).raw(), normalize(raw ^ rhs));
            let mut a = r;
            a &= rhs;
            assert_eq!(a, r & rhs);
            a = r;
            a |= rhs;
            assert_eq!(a, r | rhs);
            a = r;
            a ^= rhs;
            assert_eq!(a, r ^ rhs);
            assert_eq!(r.wrapping_add(rhs).raw(), normalize(raw.wrapping_add(rhs)));
            assert_eq!(r.wrapping_sub(rhs).raw(), normalize(raw.wrapping_sub(rhs)));
            assert_eq!(
                r.saturating_add(rhs).raw(),
                normalize(raw.saturating_add(rhs))
            );
            assert_eq!(
                r.saturating_sub(rhs).raw(),
                normalize(raw.saturating_sub(rhs))
            );
            assert_eq!(
                r.checked_add(rhs).map(|v| v.raw()),
                raw.checked_add(rhs).map(normalize)
            );
            assert_eq!(
                r.checked_sub(rhs).map(|v| v.raw()),
                raw.checked_sub(rhs).map(normalize)
            );
            let (value, overflow) = r.overflowing_add(rhs);
            let (expected, expected_overflow) = raw.overflowing_add(rhs);
            assert_eq!(
                (value.raw(), overflow),
                (normalize(expected), expected_overflow)
            );
            let (value, overflow) = r.overflowing_sub(rhs);
            let (expected, expected_overflow) = raw.overflowing_sub(rhs);
            assert_eq!(
                (value.raw(), overflow),
                (normalize(expected), expected_overflow)
            );
        }
    }
}

const SIGNATURE: u8 = 0b10_01;
#[bitfield(u16, order = msb0, width = 8)]
#[derive(Debug)]
pub struct Split {
    #[bits(0..=1, 6..=7, fixed = SIGNATURE)]
    signature: u8,
    #[bits(2..=4, fixed = -2)]
    signed: i8,
    #[bits(0, readonly, fixed = true)]
    duplicate: bool,
}

#[test]
fn signed_split_msb0_and_logical_width() {
    let r = Split::from_raw(0xFF00);
    assert_eq!(r.raw(), 0xFFB1);
    assert_eq!(r.signature(), SIGNATURE);
    assert_eq!(r.signed(), -2);
    assert!(r.duplicate());
    assert_eq!(Split::default().raw(), 0xB1);
    assert!(Split::try_from_raw(0xFFB1).is_ok());
    assert!(Split::try_from_raw(0xFF00).is_err());
}

#[bitfield(u128, order = lsb0)]
pub struct Wide {
    #[bits(0..=127, fixed = u128::MAX)]
    all: u128,
}
#[bitfield(u128, order = lsb0)]
pub struct SignedWide {
    #[bits(0..=127, fixed = i128::MIN)]
    all: i128,
}

#[test]
fn full_width_constants() {
    assert_eq!(Wide::zeroed().raw(), u128::MAX);
    assert_eq!(SignedWide::zeroed().all(), i128::MIN);
}

#[bitfield(u16, order = lsb0)]
#[derive(Debug, Default)]
pub struct Parent {
    #[bits(8..=11, 0..=3)]
    child: Fixed,
}

#[test]
fn nested_constraints_are_scattered_into_parent_storage() {
    let p = Parent::zeroed();
    assert_eq!(p.raw(), 4);
    assert_eq!(Parent::from_raw(u16::MAX).raw(), 0xF9FF);
    assert!(Parent::try_from_raw(0).is_err());
    assert!(Parent::try_from_raw(4).is_ok());
    assert_eq!(Parent::default().raw(), 4);
    assert_eq!((!p).raw() & 0x604, 4);
    assert_eq!(p.with_child(Fixed::from_raw(0xFF)).child().raw(), 0x9F);
}

#[cfg(feature = "reflection")]
#[test]
fn reflection_reports_fixed_values() {
    let one = &Fixed::FIELDS[1];
    assert!(one.readonly);
    assert!(!one.writeonly);
    assert_eq!(one.fixed, Some(1));
    assert_eq!(Split::FIELDS[1].fixed, Some(0b110));
    assert_eq!(Fixed::FIELDS[0].fixed, None);
}
