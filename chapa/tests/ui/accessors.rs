use chapa::bitfield;

#[bitfield(u8, order = lsb0)]
struct Reg {
    #[bits(0, writeonly, alias = "command")]
    write: bool,
    #[bits(1, fixed = true, alias = "constant")]
    one: bool,
}

fn main() {
    let mut r = Reg::zeroed();
    r.write();
    r.command();
    r.set_one(false);
    r.with_one(false);
    r.set_constant(false);
    r.with_constant(false);
}
