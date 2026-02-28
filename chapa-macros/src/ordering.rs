use crate::model::{BitOrder, BitRange};

/// Physical bit-manipulation parameters derived from a logical [`BitRange`].
#[derive(Debug, Clone, Copy)]
pub struct PhysicalBits {
    /// Number of positions to shift the field value before OR-ing into storage.
    pub shift: u32,
    /// Width of the field in bits.
    pub field_width: u32,
    /// Bitmask with the field's bits set, in storage position (already shifted).
    pub mask: u128,
}

/// Converts a logical `range` + `order` into the shift and mask needed to
/// read/write the field in a `effective_width`-bit storage value.
///
/// - **LSB-0**: physical shift equals `range.start`.
/// - **MSB-0**: bit 0 is the most-significant bit, so the physical shift is
///   `effective_width - 1 - range.end`.
pub fn compute(order: BitOrder, range: &BitRange, effective_width: u32) -> PhysicalBits {
    let field_width = range.width();
    let shift = match order {
        BitOrder::Lsb0 => range.start,
        BitOrder::Msb0 => effective_width - 1 - range.end,
    };
    let mask = if field_width >= 128 {
        u128::MAX
    } else {
        ((1u128 << field_width) - 1) << shift
    };
    PhysicalBits {
        shift,
        field_width,
        mask,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;

    fn range(start: u32, end: u32) -> BitRange {
        BitRange {
            start,
            end,
            span: Span::call_site(),
        }
    }

    #[test]
    fn lsb0_single_bit() {
        let p = compute(BitOrder::Lsb0, &range(3, 3), 8);
        assert_eq!(p.shift, 3);
        assert_eq!(p.field_width, 1);
        assert_eq!(p.mask, 0x08);
    }

    #[test]
    fn lsb0_range() {
        let p = compute(BitOrder::Lsb0, &range(0, 3), 8);
        assert_eq!(p.shift, 0);
        assert_eq!(p.field_width, 4);
        assert_eq!(p.mask, 0x0F);
    }

    #[test]
    fn msb0_single_bit() {
        let p = compute(BitOrder::Msb0, &range(0, 0), 8);
        assert_eq!(p.shift, 7);
        assert_eq!(p.field_width, 1);
        assert_eq!(p.mask, 0x80);
    }

    #[test]
    fn msb0_range() {
        let p = compute(BitOrder::Msb0, &range(0, 3), 32);
        assert_eq!(p.shift, 28);
        assert_eq!(p.field_width, 4);
        assert_eq!(p.mask, 0xF000_0000);
    }

    #[test]
    fn msb0_with_custom_width() {
        let p = compute(BitOrder::Msb0, &range(0, 0), 4);
        assert_eq!(p.shift, 3);
        assert_eq!(p.field_width, 1);
        assert_eq!(p.mask, 0x08);
    }

    #[test]
    fn msb0_last_bit() {
        let p = compute(BitOrder::Msb0, &range(3, 3), 4);
        assert_eq!(p.shift, 0);
        assert_eq!(p.field_width, 1);
        assert_eq!(p.mask, 0x01);
    }

    #[test]
    fn lsb0_bit_0() {
        let p = compute(BitOrder::Lsb0, &range(0, 0), 8);
        assert_eq!(p.shift, 0);
        assert_eq!(p.field_width, 1);
        assert_eq!(p.mask, 0x01);
    }

    #[test]
    fn msb0_full_width() {
        let p = compute(BitOrder::Msb0, &range(28, 31), 32);
        assert_eq!(p.shift, 0);
        assert_eq!(p.field_width, 4);
        assert_eq!(p.mask, 0x0000_000F);
    }
}
