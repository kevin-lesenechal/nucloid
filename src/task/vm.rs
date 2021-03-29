//use alloc::vec::Vec;

#[allow(unused)]
pub struct VirtualMemory {
    //areas: Vec<VMArea>,
}

#[allow(unused)]
pub struct VMArea {
    addr: usize,
    size: usize,

    enabled: bool,
    writable: bool,
    executable: bool,
}
