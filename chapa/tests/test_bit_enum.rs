use chapa::{bitfield, BitEnum, BitField};

#[derive(Debug, PartialEq, Clone, Copy, BitEnum)]
pub enum VideoFormat {
    Ntsc = 0,
    Pal = 1,
    Mpal = 2,
    #[fallback]
    Debug = 3,
}

#[test]
fn from_raw_valid() {
    assert_eq!(VideoFormat::from_raw(0), VideoFormat::Ntsc);
    assert_eq!(VideoFormat::from_raw(1), VideoFormat::Pal);
    assert_eq!(VideoFormat::from_raw(2), VideoFormat::Mpal);
    assert_eq!(VideoFormat::from_raw(3), VideoFormat::Debug);
}

#[test]
fn from_raw_invalid_uses_fallback() {
    // `Debug` is marked `#[fallback]`, so unrecognized values coerce to it.
    assert_eq!(VideoFormat::from_raw(4), VideoFormat::Debug);
    assert_eq!(VideoFormat::from_raw(255), VideoFormat::Debug);
}

#[test]
fn try_from_raw_reports_invalid() {
    assert_eq!(VideoFormat::try_from_raw(2), Ok(VideoFormat::Mpal));
    // Unlike `from_raw`, the fallback variant does not absorb bad values here.
    assert_eq!(
        VideoFormat::try_from_raw(4),
        Err(chapa::InvalidBitPattern::new(4))
    );
    assert_eq!(VideoFormat::try_from_raw(4).unwrap_err().raw, 4);
}

#[test]
fn try_from_trait() {
    assert_eq!(VideoFormat::try_from(1u8), Ok(VideoFormat::Pal));
    assert!(VideoFormat::try_from(7u8).is_err());
}

#[test]
fn raw_roundtrip() {
    for v in [
        VideoFormat::Ntsc,
        VideoFormat::Pal,
        VideoFormat::Mpal,
        VideoFormat::Debug,
    ] {
        assert_eq!(VideoFormat::from_raw(v.raw()), v);
    }
}

#[test]
fn storage_is_u8() {
    let _: u8 = VideoFormat::Ntsc.raw();
}

#[derive(Debug, PartialEq, Clone, Copy, BitEnum)]
pub enum AutoDiscrim {
    A,
    B,
    #[fallback]
    C,
}

#[test]
fn auto_discriminants() {
    assert_eq!(AutoDiscrim::A.raw(), 0);
    assert_eq!(AutoDiscrim::B.raw(), 1);
    assert_eq!(AutoDiscrim::C.raw(), 2);
}

#[test]
fn auto_discrim_from_raw() {
    assert_eq!(AutoDiscrim::from_raw(0), AutoDiscrim::A);
    assert_eq!(AutoDiscrim::from_raw(1), AutoDiscrim::B);
    assert_eq!(AutoDiscrim::from_raw(2), AutoDiscrim::C);
    assert_eq!(AutoDiscrim::from_raw(99), AutoDiscrim::C); // fallback
}

#[derive(Debug, PartialEq, Clone, Copy, BitEnum)]
pub enum Sparse {
    Low = 0,
    Mid = 5,
    #[fallback]
    High = 10,
}

#[test]
fn sparse_discriminants() {
    assert_eq!(Sparse::Low.raw(), 0);
    assert_eq!(Sparse::Mid.raw(), 5);
    assert_eq!(Sparse::High.raw(), 10);
    assert_eq!(Sparse::from_raw(5), Sparse::Mid);
    assert_eq!(Sparse::from_raw(7), Sparse::High); // fallback
}

// Enum used inside a bitfield struct.
#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DisplayConfig {
    #[bits(0)]
    rst: bool,
    #[bits(1..=2)]
    fmt: VideoFormat,
    #[bits(3..=15)]
    _reserved: u16,
}

#[test]
fn enum_in_bitfield_roundtrip() {
    let dc = DisplayConfig::new()
        .with_rst(true)
        .with_fmt(VideoFormat::Mpal);
    assert_eq!(dc.rst(), true);
    assert_eq!(dc.fmt(), VideoFormat::Mpal);
}

#[test]
fn enum_in_bitfield_all_variants() {
    for (variant, expected_raw) in [
        (VideoFormat::Ntsc, 0u8),
        (VideoFormat::Pal, 1),
        (VideoFormat::Mpal, 2),
        (VideoFormat::Debug, 3),
    ] {
        let dc = DisplayConfig::new().with_fmt(variant);
        assert_eq!(dc.fmt(), variant);
        assert_eq!(variant.raw(), expected_raw);
    }
}
