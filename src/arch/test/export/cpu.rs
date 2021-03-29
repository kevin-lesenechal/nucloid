use crate::driver::vga::VgaScreen;
use core::fmt;
use core::fmt::{Display, Formatter};

pub struct MachineState {}

impl MachineState {
    pub fn print(&self, _vga: &mut impl VgaScreen) -> fmt::Result {
        unimplemented!();
    }
}

impl Display for MachineState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "[test MachineState]")
    }
}

pub fn halt() {
    unimplemented!();
}

pub fn perm_halt() -> ! {
    unimplemented!();
}
