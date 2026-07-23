//! Utilities for masking raw integers by bit ranges.

/// Build a u128 bitmask in MSB0 ordering (bit 0 = MSB).
///
/// `width` is the total bit width of the storage type (e.g. 32 for `u32`).
/// Each element of `ranges` is an inclusive `(start, end)` pair.
pub const fn msb0_mask(width: u32, ranges: &[(u8, u8)]) -> u128 {
    let mut mask = 0u128;
    let mut i = 0;
    while i < ranges.len() {
        let (start, end) = ranges[i];
        let mut bit = start;
        while bit <= end {
            mask |= 1u128 << (width - 1 - bit as u32);
            bit += 1;
        }
        i += 1;
    }
    mask
}

/// Build a u128 bitmask in LSB0 ordering (bit 0 = LSB).
///
/// Each element of `ranges` is an inclusive `(start, end)` pair.
pub const fn lsb0_mask(ranges: &[(u8, u8)]) -> u128 {
    let mut mask = 0u128;
    let mut i = 0;
    while i < ranges.len() {
        let (start, end) = ranges[i];
        let mut bit = start;
        while bit <= end {
            mask |= 1u128 << bit;
            bit += 1;
        }
        i += 1;
    }
    mask
}

/// Converts a half-open range to the inclusive pair used by the mask helpers.
///
/// Empty and reversed ranges use a `start > end` pair, which the mask helpers
/// naturally treat as selecting no bits.
#[doc(hidden)]
#[inline]
pub const fn __half_open_pair(start: u8, end: u8) -> (u8, u8) {
    if start < end {
        (start, end - 1)
    } else {
        (1, 0)
    }
}

/// A runtime bit or range accepted by the bit manipulation macros.
#[doc(hidden)]
pub trait __BitSpec {
    /// Converts the specification to an inclusive `(start, end)` pair.
    fn __inclusive_pair(self) -> (u8, u8);
}

macro_rules! impl_runtime_bit {
    ($($ty:ty),* $(,)?) => {
        $(
            impl __BitSpec for $ty {
                #[inline]
                fn __inclusive_pair(self) -> (u8, u8) {
                    let bit = match u8::try_from(self) {
                        Ok(bit) => bit,
                        Err(_) => panic!("bit index must fit in u8"),
                    };
                    (bit, bit)
                }
            }
        )*
    };
}

impl_runtime_bit!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

impl<T> __BitSpec for core::ops::Range<T>
where
    T: TryInto<u8>,
{
    #[inline]
    fn __inclusive_pair(self) -> (u8, u8) {
        let start = match self.start.try_into() {
            Ok(start) => start,
            Err(_) => panic!("range start must fit in u8"),
        };
        let end = match self.end.try_into() {
            Ok(end) => end,
            Err(_) => panic!("range end must fit in u8"),
        };
        __half_open_pair(start, end)
    }
}

impl<T> __BitSpec for core::ops::RangeInclusive<T>
where
    T: TryInto<u8>,
{
    #[inline]
    fn __inclusive_pair(self) -> (u8, u8) {
        let (start, end) = self.into_inner();
        let start = match start.try_into() {
            Ok(start) => start,
            Err(_) => panic!("range start must fit in u8"),
        };
        let end = match end.try_into() {
            Ok(end) => end,
            Err(_) => panic!("range end must fit in u8"),
        };
        if start <= end {
            (start, end)
        } else {
            (1, 0)
        }
    }
}

/// Normalizes a runtime bit or range for macro expansion.
#[doc(hidden)]
#[inline]
pub fn __bit_spec_pair(spec: impl __BitSpec) -> (u8, u8) {
    spec.__inclusive_pair()
}

/// Internal helper: convert a mixed list of single bits, `start..=end`, and
/// `start..end` ranges into an array of `(u8, u8)` inclusive pairs via
/// tt-munching.
///
/// Single bit `n` becomes `(n, n)`; half-open `s..e` becomes `(s, e - 1)`.
/// Runtime integer and range expressions are normalized through [`__BitSpec`].
/// Not for direct use.
#[doc(hidden)]
#[macro_export]
macro_rules! __bits_pairs {
    // Base: nothing left, emit the collected pairs as an array
    (@acc [$($pairs:tt)*]) => {
        [$($pairs)*]
    };
    // Inclusive range followed by comma + rest
    (@acc [$($pairs:tt)*] $s:literal ..= $e:literal, $($rest:tt)*) => {
        $crate::__bits_pairs!(@acc [$($pairs)* ($s as u8, $e as u8),] $($rest)*)
    };
    // Inclusive range at end (no trailing comma)
    (@acc [$($pairs:tt)*] $s:literal ..= $e:literal) => {
        $crate::__bits_pairs!(@acc [$($pairs)* ($s as u8, $e as u8),])
    };
    // Half-open range followed by comma + rest
    (@acc [$($pairs:tt)*] $s:literal .. $e:literal, $($rest:tt)*) => {
        $crate::__bits_pairs!(
            @acc [$($pairs)* $crate::mask::__half_open_pair($s as u8, $e as u8),]
            $($rest)*
        )
    };
    // Half-open range at end (no trailing comma)
    (@acc [$($pairs:tt)*] $s:literal .. $e:literal) => {
        $crate::__bits_pairs!(
            @acc [$($pairs)* $crate::mask::__half_open_pair($s as u8, $e as u8),]
        )
    };
    // Single bit followed by comma + rest
    (@acc [$($pairs:tt)*] $bit:literal, $($rest:tt)*) => {
        $crate::__bits_pairs!(@acc [$($pairs)* ($bit as u8, $bit as u8),] $($rest)*)
    };
    // Single bit at end (no trailing comma)
    (@acc [$($pairs:tt)*] $bit:literal) => {
        $crate::__bits_pairs!(@acc [$($pairs)* ($bit as u8, $bit as u8),])
    };
    // Runtime bit or range expression followed by comma + rest
    (@acc [$($pairs:tt)*] $spec:expr, $($rest:tt)*) => {
        $crate::__bits_pairs!(
            @acc [$($pairs)* $crate::mask::__bit_spec_pair($spec),]
            $($rest)*
        )
    };
    // Runtime bit or range expression at end (no trailing comma)
    (@acc [$($pairs:tt)*] $spec:expr) => {
        $crate::__bits_pairs!(
            @acc [$($pairs)* $crate::mask::__bit_spec_pair($spec),]
        )
    };
    // Anything else is unsupported. Without this arm the failed `@acc` stream
    // falls through to the catch-all entry point below and recurses until the
    // recursion limit, hiding the actual problem.
    (@acc [$($pairs:tt)*] $($rest:tt)+) => {
        ::core::compile_error!(
            "unsupported bit spec: expected integer or range expressions as \
             `N`, `N..=M`, or `N..M`, separated by commas"
        )
    };
    // Entry point
    ($($tokens:tt)*) => {
        $crate::__bits_pairs!(@acc [] $($tokens)*)
    };
}

/// Builds the mask used by the explicit macro forms. It remains const-evaluable
/// when every specification is a literal.
#[doc(hidden)]
#[macro_export]
macro_rules! __const_mask {
    (msb0 $ty:ty; $($specs:tt)*) => {
        $crate::mask::msb0_mask(<$ty>::BITS, &$crate::__bits_pairs!($($specs)*)) as $ty
    };
    (lsb0 $ty:ty; $($specs:tt)*) => {
        $crate::mask::lsb0_mask(&$crate::__bits_pairs!($($specs)*)) as $ty
    };
}

/// Compute the mask for a chapa bitfield struct, using ordering and width from its [`BitField`] impl.
///
/// Used by the no-prefix form of [`extract_bits!`].
///
/// [`BitField`]: crate::BitField
#[inline]
pub fn extract_mask<T: crate::BitField>(ranges: &[(u8, u8)]) -> T::Storage {
    let raw = if T::IS_MSB0 {
        msb0_mask(<T::Storage as crate::BitStorage>::BITS, ranges)
    } else {
        lsb0_mask(ranges)
    };
    <T::Storage as crate::BitStorage>::from_u128(raw)
}

/// Apply a bit mask to a chapa bitfield struct, keeping only the specified bits.
///
/// Used by the no-prefix form of [`extract_bits!`]. The ordering and storage
/// width are taken from the struct's [`BitField`] impl (generated by `#[bitfield]`).
///
/// [`BitField`]: crate::BitField
#[inline]
pub fn extract_bits_auto<T>(val: T, ranges: &[(u8, u8)]) -> T
where
    T: crate::BitField,
    T: core::ops::BitAnd<T::Storage, Output = T>,
{
    val & extract_mask::<T>(ranges)
}

/// Copies already-positioned bits from `src` into selected ranges of `dst`.
///
/// This supports the bitfield form of [`insert_bits!`]. Bits outside the ranges
/// keep their value from `dst`.
///
#[inline]
pub fn insert_bits_auto<T>(dst: T, src: T::Storage, ranges: &[(u8, u8)]) -> T
where
    T: crate::BitField
        + core::ops::BitAnd<T::Storage, Output = T>
        + core::ops::BitOr<T::Storage, Output = T>,
    T::Storage: core::ops::BitAnd<Output = T::Storage> + core::ops::Not<Output = T::Storage>,
{
    let mask = extract_mask::<T>(ranges);
    (dst & !mask) | (src & mask)
}

/// Shifts a right-aligned value into the range `lo..=hi` of `dst`.
///
/// This supports the bitfield form of [`place_bits!`]. Bits above the range's
/// width are dropped.
///
#[inline]
pub fn place_bits_auto<T>(dst: T, lo: u8, hi: u8, val: T::Storage) -> T
where
    T: crate::BitField
        + core::ops::BitAnd<T::Storage, Output = T>
        + core::ops::BitOr<T::Storage, Output = T>,
    T::Storage: core::ops::BitAnd<Output = T::Storage>
        + core::ops::Not<Output = T::Storage>
        + core::ops::Shl<u32, Output = T::Storage>,
{
    let shift = if T::IS_MSB0 {
        <T::Storage as crate::BitStorage>::BITS - 1 - hi as u32
    } else {
        lo as u32
    };
    insert_bits_auto(dst, val << shift, &[(lo, hi)])
}

/// Keep only the specified bits from a value.
///
/// You can list multiple bits and ranges (`N`, `N..=M`, or `N..M`). Literal
/// specs are usable in const contexts; runtime integer and range expressions
/// are evaluated when the macro is called.
///
/// # Syntax
///
/// ```
/// # use chapa::extract_bits;
/// let val: u32 = 0xFFFF_FFFF;
///
/// // MSB0 ordering (bit 0 = MSB): keep bits 0, 5–9, and 16–31
/// let masked = extract_bits!(msb0 u32; val; 0, 5..=9, 16..=31);
/// assert_eq!(masked, 0x87C0_FFFF);
///
/// // LSB0 ordering (bit 0 = LSB): keep bits 0–3 and 12–15
/// let masked = extract_bits!(lsb0 u16; val as u16; 0..=3, 12..=15);
/// assert_eq!(masked, 0xF00F);
/// ```
///
/// For chapa bitfield structs, the ordering and storage type can be omitted;
/// they are deduced from the struct's [`BitField::IS_MSB0`] constant.
/// The result is the same struct type with the non-selected bits zeroed out.
///
/// **Note on const:** The explicit (`msb0`/`lsb0`) form is usable in const
/// contexts when its value and specs are literals. The struct form
/// calls an [`#[inline]`](inline) helper; LLVM can constant-fold literal specs
/// in optimized builds, but there is no language-level `const` guarantee.
///
/// ```
/// # use chapa::{bitfield, extract_bits};
/// # #[bitfield(u32, order = msb0)]
/// # #[derive(Copy, Clone)]
/// # struct Packet {
/// #     #[bits(0)] priority: bool,
/// #     #[bits(5..=9)] kind: u8,
/// #     #[bits(16..=31)] payload: u16,
/// # }
/// let packet = Packet::from_raw(0xFFFF_FFFF);
/// let masked: Packet = extract_bits!(packet; 0, 5..=9, 16..=31);
/// assert_eq!(masked.raw(), 0x87C0_FFFF);
/// ```
///
/// [`BitField::IS_MSB0`]: crate::BitField::IS_MSB0
#[macro_export]
macro_rules! extract_bits {
    // Explicit MSB0 with type: extract_bits!(msb0 u32; val; specs...)
    (msb0 $ty:ty; $val:expr; $($specs:tt)*) => {{
        let mask: $ty = $crate::__const_mask!(msb0 $ty; $($specs)*);
        ($val) & mask
    }};
    // Explicit LSB0 with type: extract_bits!(lsb0 u8; val; specs...)
    (lsb0 $ty:ty; $val:expr; $($specs:tt)*) => {{
        let mask: $ty = $crate::__const_mask!(lsb0 $ty; $($specs)*);
        ($val) & mask
    }};
    // Chapa struct (ordering deduced): extract_bits!(struct_val; specs...)
    ($val:expr; $($specs:tt)*) => {
        $crate::mask::extract_bits_auto($val, &$crate::__bits_pairs!($($specs)*))
    };
}

/// Copies already-positioned bits into selected ranges of a value.
///
/// Bits outside the selected ranges keep their value from `dst`. The bits in
/// `src` must already be in the correct position. Use [`place_bits!`] to shift a
/// right-aligned value into one range.
///
/// You can list multiple bits and ranges (`N`, `N..=M`, or `N..M`). Literal
/// specs are usable in const contexts; runtime integer and range expressions
/// are evaluated when the macro is called.
///
/// # Syntax
///
/// ```
/// # use chapa::insert_bits;
/// let dst: u32 = 0x0000_0000;
/// let src: u32 = 0xFFFF_FFFF;
///
/// // MSB0 ordering: replace bits 0 and 16..=31 of `dst` with those of `src`
/// let merged = insert_bits!(msb0 u32; dst; src; 0, 16..=31);
/// assert_eq!(merged, 0x8000_FFFF);
///
/// // LSB0 ordering: replace bits 0..=3 and 8..=15
/// let merged = insert_bits!(lsb0 u32; dst; src; 0..=3, 8..=15);
/// assert_eq!(merged, 0x0000_FF0F);
/// ```
///
/// For bitfield values, omit the ordering and storage type. Pass `src` as a raw
/// storage value:
///
/// ```
/// # use chapa::{bitfield, insert_bits};
/// # #[bitfield(u32, order = msb0)]
/// # struct Reg {
/// #     #[bits(0..=7)] a: u8,
/// #     #[bits(8..=31)] b: u32,
/// # }
/// let reg = Reg::from_raw(0x1234_5678);
/// let updated = insert_bits!(reg; 0xFF00_0000u32; 0..=7);
/// assert_eq!(updated.raw(), 0xFF34_5678);
/// ```
///
/// The explicit `msb0` and `lsb0` forms remain const-evaluable with literal
/// specs.
///
/// The bitfield form uses the full storage width for `msb0`. If the bitfield has
/// `width = N`, use the explicit `msb0` form instead.
///
#[macro_export]
macro_rules! insert_bits {
    // Explicit MSB0 with type: insert_bits!(msb0 u32; dst; src; specs...)
    (msb0 $ty:ty; $dst:expr; $src:expr; $($specs:tt)*) => {{
        let mask: $ty = $crate::__const_mask!(msb0 $ty; $($specs)*);
        (($dst) & !mask) | (($src) & mask)
    }};
    // Explicit LSB0 with type: insert_bits!(lsb0 u8; dst; src; specs...)
    (lsb0 $ty:ty; $dst:expr; $src:expr; $($specs:tt)*) => {{
        let mask: $ty = $crate::__const_mask!(lsb0 $ty; $($specs)*);
        (($dst) & !mask) | (($src) & mask)
    }};
    // Bitfield form: insert_bits!(value; raw_src; specs...)
    ($dst:expr; $src:expr; $($specs:tt)*) => {
        $crate::mask::insert_bits_auto($dst, ($src as _), &$crate::__bits_pairs!($($specs)*))
    };
}

/// Shift a right-aligned value into a single bit range of a value.
///
/// The value is masked to the range width, shifted into position, and written
/// over `dst`. Bits outside the range keep their value from `dst`.
///
/// This macro accepts one bit `N`, one inclusive range `N..=M`, or one
/// half-open range `N..M`. Each may be a literal or a runtime expression.
///
/// # Syntax
///
/// ```
/// # use chapa::place_bits;
/// let reg: u32 = 0;
///
/// // LSB0: write 0xAB into bits 8..=15.
/// let reg = place_bits!(lsb0 u32; reg; 8..=15; 0xABu8);
/// assert_eq!(reg, 0x0000_AB00);
///
/// // MSB0: write the same value with bits numbered from the MSB.
/// let reg = place_bits!(msb0 u32; 0u32; 8..=15; 0xABu8);
/// assert_eq!(reg, 0x00AB_0000);
///
/// // A single bit
/// let reg = place_bits!(lsb0 u8; 0u8; 3; 1u8);
/// assert_eq!(reg, 0b0000_1000);
/// ```
///
/// For bitfield values, omit the ordering and storage type:
///
/// ```
/// # use chapa::{bitfield, place_bits};
/// # #[bitfield(u32, order = lsb0)]
/// # struct Reg {
/// #     #[bits(0..=7)] lo: u8,
/// #     #[bits(8..=15)] mid: u8,
/// #     #[bits(16..=31)] hi: u16,
/// # }
/// let reg = Reg::zeroed();
/// let reg = place_bits!(reg; 8..=15; 0xABu8);
/// assert_eq!(reg.mid(), 0xAB);
/// ```
///
/// Values wider than the range are truncated to the range width, like a field setter.
///
/// The explicit forms remain const-evaluable with literal specs. For an `msb0`
/// bitfield with `width = N`, use the explicit form.
#[macro_export]
macro_rules! place_bits {
    // Explicit MSB0, range: place_bits!(msb0 u32; dst; lo..=hi; val)
    (msb0 $ty:ty; $dst:expr; $lo:literal ..= $hi:literal; $val:expr) => {{
        const MASK: $ty = $crate::__const_mask!(msb0 $ty; $lo ..= $hi);
        const SHIFT: u32 = <$ty>::BITS - 1 - ($hi as u32);
        (($dst) & !MASK) | ((($val as $ty) << SHIFT) & MASK)
    }};
    // Explicit LSB0, range: place_bits!(lsb0 u32; dst; lo..=hi; val)
    (lsb0 $ty:ty; $dst:expr; $lo:literal ..= $hi:literal; $val:expr) => {{
        const MASK: $ty = $crate::__const_mask!(lsb0 $ty; $lo ..= $hi);
        const SHIFT: u32 = $lo as u32;
        (($dst) & !MASK) | ((($val as $ty) << SHIFT) & MASK)
    }};
    // Bitfield range: place_bits!(dst; lo..=hi; val)
    ($dst:expr; $lo:literal ..= $hi:literal; $val:expr) => {
        $crate::mask::place_bits_auto($dst, $lo, $hi, ($val as _))
    };
    // Half-open range forms: `lo..hi` is `lo..=(hi - 1)`.
    (msb0 $ty:ty; $dst:expr; $lo:literal .. $hi:literal; $val:expr) => {{
        let (lo, hi) = $crate::mask::__half_open_pair($lo as u8, $hi as u8);
        if lo > hi {
            $dst
        } else {
            let mask: $ty = $crate::mask::msb0_mask(<$ty>::BITS, &[(lo, hi)]) as $ty;
            let shift = <$ty>::BITS - 1 - hi as u32;
            (($dst) & !mask) | ((($val as $ty) << shift) & mask)
        }
    }};
    (lsb0 $ty:ty; $dst:expr; $lo:literal .. $hi:literal; $val:expr) => {{
        let (lo, hi) = $crate::mask::__half_open_pair($lo as u8, $hi as u8);
        if lo > hi {
            $dst
        } else {
            let mask: $ty = $crate::mask::lsb0_mask(&[(lo, hi)]) as $ty;
            (($dst) & !mask) | ((($val as $ty) << lo as u32) & mask)
        }
    }};
    ($dst:expr; $lo:literal .. $hi:literal; $val:expr) => {{
        let (lo, hi) = $crate::mask::__half_open_pair($lo as u8, $hi as u8);
        if lo > hi {
            $dst
        } else {
            $crate::mask::place_bits_auto($dst, lo, hi, ($val as _))
        }
    }};
    // Single bit forms normalize to `bit ..= bit` and forward to the range arms.
    (msb0 $ty:ty; $dst:expr; $bit:literal; $val:expr) => {
        $crate::place_bits!(msb0 $ty; $dst; $bit ..= $bit; $val)
    };
    (lsb0 $ty:ty; $dst:expr; $bit:literal; $val:expr) => {
        $crate::place_bits!(lsb0 $ty; $dst; $bit ..= $bit; $val)
    };
    ($dst:expr; $bit:literal; $val:expr) => {
        $crate::place_bits!($dst; $bit ..= $bit; $val)
    };
    // Runtime bit or range expressions.
    (msb0 $ty:ty; $dst:expr; $spec:expr; $val:expr) => {{
        let (lo, hi) = $crate::mask::__bit_spec_pair($spec);
        if lo > hi {
            $dst
        } else {
            let mask: $ty = $crate::mask::msb0_mask(<$ty>::BITS, &[(lo, hi)]) as $ty;
            let shift = <$ty>::BITS - 1 - hi as u32;
            (($dst) & !mask) | ((($val as $ty) << shift) & mask)
        }
    }};
    (lsb0 $ty:ty; $dst:expr; $spec:expr; $val:expr) => {{
        let (lo, hi) = $crate::mask::__bit_spec_pair($spec);
        if lo > hi {
            $dst
        } else {
            let mask: $ty = $crate::mask::lsb0_mask(&[(lo, hi)]) as $ty;
            (($dst) & !mask) | ((($val as $ty) << lo as u32) & mask)
        }
    }};
    ($dst:expr; $spec:expr; $val:expr) => {{
        let (lo, hi) = $crate::mask::__bit_spec_pair($spec);
        if lo > hi {
            $dst
        } else {
            $crate::mask::place_bits_auto($dst, lo, hi, ($val as _))
        }
    }};
}
