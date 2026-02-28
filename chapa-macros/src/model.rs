use proc_macro2::Span;

/// Which end bit 0 refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOrder {
    /// Bit 0 is the most-significant bit.
    Msb0,
    /// Bit 0 is the least-significant bit.
    Lsb0,
}

/// The primitive integer type chosen as backing storage for a bitfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl StorageKind {
    /// Returns the number of bits in this storage type.
    pub fn bit_width(self) -> u32 {
        match self {
            StorageKind::U8 => 8,
            StorageKind::U16 => 16,
            StorageKind::U32 => 32,
            StorageKind::U64 => 64,
            StorageKind::U128 => 128,
        }
    }

    /// Returns the Rust keyword for this type (e.g. `"u32"`).
    pub fn ident(self) -> &'static str {
        match self {
            StorageKind::U8 => "u8",
            StorageKind::U16 => "u16",
            StorageKind::U32 => "u32",
            StorageKind::U64 => "u64",
            StorageKind::U128 => "u128",
        }
    }

    /// Parses a Rust integer keyword into a `StorageKind`, or `None` if unrecognised.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "u8" => Some(StorageKind::U8),
            "u16" => Some(StorageKind::U16),
            "u32" => Some(StorageKind::U32),
            "u64" => Some(StorageKind::U64),
            "u128" => Some(StorageKind::U128),
            _ => None,
        }
    }

    /// Pick the smallest storage kind that fits `bits` bits.
    pub fn smallest_fitting(bits: u32) -> Option<Self> {
        if bits <= 8 {
            Some(StorageKind::U8)
        } else if bits <= 16 {
            Some(StorageKind::U16)
        } else if bits <= 32 {
            Some(StorageKind::U32)
        } else if bits <= 64 {
            Some(StorageKind::U64)
        } else if bits <= 128 {
            Some(StorageKind::U128)
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
    /// A primitive unsigned integer (`u8`...`u128`).
    Primitive(StorageKind),
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
    #[allow(dead_code)]
    /// Original syntactic type (kept for diagnostics).
    pub raw_ty: syn::Type,
    /// Logical bit range declared in `#[bits(...)]`.
    pub range: BitRange,
    /// Whether setters should be suppressed.
    pub readonly: bool,
    /// Extra accessor names declared with `alias = ...`.
    pub aliases: Vec<String>,
    /// Overlay group name declared with `overlay = "..."`, if any.
    pub overlay: Option<String>,
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
    /// Non-`#[bitfield]` attributes forwarded to the generated struct.
    pub user_attrs: Vec<syn::Attribute>,
}
