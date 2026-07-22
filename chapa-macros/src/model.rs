use proc_macro2::Span;

/// Which end bit 0 refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOrder {
    /// Bit 0 is the most-significant bit.
    Msb0,
    /// Bit 0 is the least-significant bit.
    Lsb0,
}

/// The width of a primitive integer, named by bit count.
///
/// Used both for the backing storage of a bitfield (always unsigned) and to
/// size primitive field types, which may be signed or unsigned; the
/// `unsigned_ident`/`signed_ident` pair maps a width to the concrete Rust
/// keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    W8,
    W16,
    W32,
    W64,
    W128,
}

impl StorageKind {
    /// Returns the number of bits in this width.
    pub fn bit_width(self) -> u32 {
        match self {
            StorageKind::W8 => 8,
            StorageKind::W16 => 16,
            StorageKind::W32 => 32,
            StorageKind::W64 => 64,
            StorageKind::W128 => 128,
        }
    }

    /// Returns the unsigned Rust keyword for this width (e.g. `"u32"`).
    pub fn unsigned_ident(self) -> &'static str {
        match self {
            StorageKind::W8 => "u8",
            StorageKind::W16 => "u16",
            StorageKind::W32 => "u32",
            StorageKind::W64 => "u64",
            StorageKind::W128 => "u128",
        }
    }

    /// Returns the signed Rust keyword for this width (e.g. `"i32"`).
    pub fn signed_ident(self) -> &'static str {
        match self {
            StorageKind::W8 => "i8",
            StorageKind::W16 => "i16",
            StorageKind::W32 => "i32",
            StorageKind::W64 => "i64",
            StorageKind::W128 => "i128",
        }
    }

    /// Parses an unsigned integer keyword (`u8`...`u128`) into its
    /// `StorageKind`, or `None` if unrecognised.
    pub fn from_unsigned_str(s: &str) -> Option<Self> {
        match s {
            "u8" => Some(StorageKind::W8),
            "u16" => Some(StorageKind::W16),
            "u32" => Some(StorageKind::W32),
            "u64" => Some(StorageKind::W64),
            "u128" => Some(StorageKind::W128),
            _ => None,
        }
    }

    /// Parses a signed integer keyword (`i8`...`i128`) into the `StorageKind`
    /// of the same width, or `None` if unrecognised.
    pub fn from_signed_str(s: &str) -> Option<Self> {
        match s {
            "i8" => Some(StorageKind::W8),
            "i16" => Some(StorageKind::W16),
            "i32" => Some(StorageKind::W32),
            "i64" => Some(StorageKind::W64),
            "i128" => Some(StorageKind::W128),
            _ => None,
        }
    }

    /// Pick the smallest storage kind that fits `bits` bits.
    pub fn smallest_fitting(bits: u32) -> Option<Self> {
        if bits <= 8 {
            Some(StorageKind::W8)
        } else if bits <= 16 {
            Some(StorageKind::W16)
        } else if bits <= 32 {
            Some(StorageKind::W32)
        } else if bits <= 64 {
            Some(StorageKind::W64)
        } else if bits <= 128 {
            Some(StorageKind::W128)
        } else {
            None
        }
    }
}

/// Parsed arguments from `#[bitfield(u32, order = msb0, width = 16)]`.
#[derive(Debug, Clone)]
pub struct BitfieldArgs {
    /// The declared backing storage type.
    pub storage: StorageKind,
    /// Span of the storage token (for error reporting).
    pub storage_span: Span,
    /// Logical bit ordering.
    pub order: BitOrder,
    #[allow(dead_code)]
    /// Span of the `order` key (for error reporting).
    pub order_span: Span,
    /// Optional effective width, narrower than the storage type.
    pub width: Option<u32>,
    /// Span of the `width` value (for error reporting).
    pub width_span: Option<Span>,
}

/// An inclusive bit range, e.g. `0..=3`.
#[derive(Debug, Clone, Copy)]
pub struct BitRange {
    /// Start bit (inclusive).
    pub start: u32,
    /// End bit (inclusive).
    pub end: u32,
    /// Source span of the range literal (for error reporting).
    pub span: Span,
}

impl BitRange {
    /// Returns the number of bits covered by this range.
    pub fn width(&self) -> u32 {
        self.end - self.start + 1
    }

    /// Returns `true` if this range shares any bits with `other`.
    pub fn overlaps(&self, other: &BitRange) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

/// The Rust type of a bitfield field, as understood by the macro.
#[derive(Clone)]
pub enum FieldType {
    /// A single `bool` bit.
    Bool,
    /// A primitive unsigned integer (`u8`...`u128`) of the given width.
    PrimitiveUnsigned(StorageKind),
    /// A primitive signed integer (`i8`...`i128`) of the given width. Getters
    /// sign-extend the field's most significant bit, setters truncate
    /// two's-complement values to the field width.
    PrimitiveSigned(StorageKind),
    /// A nested bitfield struct implementing [`chapa::BitField`].
    Nested(syn::Type),
}

/// All information about one field in a bitfield struct.
#[derive(Clone)]
pub struct FieldDef {
    /// The original struct field identifier (may start with `_`).
    pub name: syn::Ident,
    /// The public accessor name (leading `_` stripped if present).
    pub accessor_name: String,
    /// Resolved field type.
    pub ty: FieldType,
    /// Original type.
    pub raw_ty: syn::Type,
    /// Logical bit range declared in `#[bits(...)]`.
    pub range: BitRange,
    /// Whether setters should be suppressed.
    pub readonly: bool,
    /// Extra accessor names declared with `alias = ...`.
    pub aliases: Vec<String>,
    /// Overlay group name declared with `overlay = "..."`, if any.
    pub overlay: Option<String>,
    /// Default value expression declared with `default = ...`, if any.
    ///
    /// Applied by `Default::default()`. Setting a field default automatically
    /// implements `Default` for the struct.
    pub default: Option<syn::Expr>,
    /// Source span of the field identifier (for error reporting).
    pub span: Span,
}

/// The fully parsed and resolved definition of a bitfield struct.
#[derive(Clone)]
pub struct BitfieldDef {
    /// Macro arguments (`storage`, `order`, `width`).
    pub args: BitfieldArgs,
    /// Effective logical width: `width` if specified, else `storage.bit_width()`.
    pub effective_width: u32,
    /// All fields in declaration order.
    pub fields: Vec<FieldDef>,
    /// Visibility of the original struct.
    pub vis: syn::Visibility,
    /// Name of the original struct.
    pub name: syn::Ident,
    /// Attributes forwarded to the generated struct. Derives implemented by the
    /// macro have already been removed.
    pub user_attrs: Vec<proc_macro2::TokenStream>,
    /// Location of a user-written `Debug` derive.
    pub debug_span: Option<proc_macro2::Span>,
    /// Location of a user-written `Default` derive.
    pub default_span: Option<proc_macro2::Span>,
    /// Location of a user-written `Copy` derive.
    pub copy_span: Option<proc_macro2::Span>,
    /// Location of a user-written `Clone` derive.
    pub clone_span: Option<proc_macro2::Span>,
}
