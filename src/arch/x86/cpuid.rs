use x86::cpuid::CpuId;

static mut CPUID: Option<CpuId> = None;

pub unsafe fn init() {
    CPUID = Some(CpuId::new());
}

pub fn get() -> &'static CpuId {
    unsafe { CPUID.as_ref().expect("CPUID not initialized") }
}
