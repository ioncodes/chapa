use chapa::bitfield;

#[bitfield(u8, order = lsb0)]
struct Invalid {
    #[bits(0..=2, fixed = 0)]
    zeros: u8,
    #[bits(2, fixed = true)]
    one: bool,
}
fn main() {}
