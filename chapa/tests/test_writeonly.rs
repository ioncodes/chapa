use chapa::bitfield;

#[bitfield(u32, order = lsb0)]
#[derive(Debug)]
pub struct MiMask {
    #[bits(0, readonly, overlay = "read")]
    sp_enabled: bool,
    #[bits(1, readonly, overlay = "read")]
    si_enabled: bool,
    #[bits(0, writeonly, overlay = "write", alias = "disable_sp")]
    clear_sp: bool,
    #[bits(1, writeonly, overlay = "write", alias = ["enable_sp", "unmask_sp"])]
    set_sp: bool,
}

#[test]
fn read_and_write_layouts_and_aliases() {
    let read = MiMask::from_raw(1);
    assert!(read.sp_enabled());
    assert!(!read.si_enabled());
    const COMMAND: MiMask = MiMask::zeroed().with_enable_sp(true);
    assert_eq!(COMMAND.raw(), 2);
    let mut write = MiMask::zeroed();
    write.set_disable_sp(true);
    write.set_unmask_sp(true);
    assert_eq!(write.raw(), 3);
    assert_eq!(MiMask::zeroed().with_clear_sp(true).raw(), 1);
    assert_eq!(MiMask::zeroed().with_unmask_sp(true).raw(), 2);
    assert_eq!(
        format!("{read:?}"),
        "MiMask { sp_enabled: true, si_enabled: false }"
    );
}

#[bitfield(u8, order = lsb0)]
#[derive(Debug)]
pub struct Command {
    #[bits(0..=7, writeonly, default = 42)]
    command: u8,
}

#[test]
fn writeonly_default_and_debug() {
    assert_eq!(Command::default().raw(), 42);
    assert_eq!(format!("{:?}", Command::default()), "Command");
}

#[cfg(feature = "reflection")]
#[test]
fn reflection_reports_access_permissions() {
    assert!(MiMask::FIELDS[0].readonly);
    assert!(!MiMask::FIELDS[0].writeonly);
    assert!(!MiMask::FIELDS[2].readonly);
    assert!(MiMask::FIELDS[2].writeonly);
    assert_eq!(MiMask::FIELDS[2].aliases, &["disable_sp"]);
}
