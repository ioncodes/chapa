use chapa::bitfield;

#[bitfield(u8, order = lsb0)]
struct Child {
    #[bits(0, fixed = true)]
    one: bool,
}
#[bitfield(u8, order = lsb0)]
struct Parent {
    #[bits(0..=7)]
    child: Child,
    #[bits(0, fixed = false)]
    zero: bool,
}
fn main() {}
