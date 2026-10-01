use chapa::bitfield;

#[bitfield(u8, order = lsb0)]
struct Child {
    #[bits(7, fixed = false)]
    fixed: bool,
}
#[bitfield(u8, order = lsb0)]
struct Parent {
    #[bits(0..=3)]
    child: Child,
}
fn main() {}
