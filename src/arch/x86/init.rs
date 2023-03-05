/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::arch::x86::driver::serial::{SerialDevice, COM1_IOPORT, ParityMode,
                                       StopBits};
use crate::arch::x86::{gdt, irq};
use crate::{debug, info, main, notice};
use crate::mem::{PAddr, PHYS_MEM_SIZE, LOWMEM_VA_END};
use crate::arch::sync::{push_critical_region, pop_critical_region};
use crate::arch::mem::LOWMEM_VA_START;
use crate::arch;

use crate::screen::R;
use crate::arch::x86::mem::{lowmem_va_size, physical_memory_size};
use crate::arch::x86::driver::vesa::VesaFramebuffer;
use crate::arch::x86::export::logging::LOGGER_SERIAL;
use crate::logging::DEFAULT_LOGGER;
use crate::mem::load::{kernel_image, kernel_rodata_segment, kernel_text_segment};
use crate::ui::kterm::KERNEL_TERMINAL;
use crate::ui::term::Terminal;

/// Welcome in Rust land! This is the very first Rust code to run on the CPU
/// once the previous `_start` routine in assembly ran. We did the bare
/// minimum in this routine to run Rust (mostly setting up paging) since
/// assembly is not my preffered programming language.
///
/// It is time to perform the most vital initializations; the order in which to
/// perform them is critical: we want to be able to easely debug the kernel.
///
/// First, we need to setup a USART so that crashes and debug information are
/// able to be exfiltrated since a lot can go wrong before we can print anything
/// onto the screen. This is convenient on both QEMU and real hardware.
///
/// Then, some basic global variables are set from the Multiboot information
/// structure: `PHYS_MEM_SIZE` and `LOWMEM_VA_END`.
///
/// We then set up the kernel's GDT, since the GDT created by `_start` is not
/// enough to run ring 3 code or 32 bits code.
///
/// After that, the IDT is initialized: from there, we can handle CPU exceptions
/// and print useful crash report.
///
/// Then most important part: we set up the memory management, composed of:
///     * mapping all low-memory in the virtual address space;
///     * setting up proper page protections for read/write/execute;
///     * creating and configuring the physical frames allocator;
///     * (i386) constructing the high-memory allocator.
///
/// Interrupts can now be enabled.
///
/// Finally, we call the kernel's `main` function to start the architecture-
/// agnostic code.
#[no_mangle]
pub unsafe extern "C" fn arch_init(multiboot_info_pa: PAddr) -> ! {
    // We are not yet ready to handle interruptions: we don't even have an IDT!
    push_critical_region();

    LOGGER_SERIAL = Some(unsafe { SerialDevice::new(
        COM1_IOPORT, 115200, ParityMode::None, 8, StopBits::One
    ).expect("Couldn't initialize serial device") });
    *DEFAULT_LOGGER.lock() = LOGGER_SERIAL.as_mut().unwrap();

    let mbi = multiboot2::load(
        multiboot_info_pa
            .into_lowmem_vaddr()
            .unwrap()
            .0
    ).unwrap();

    notice!("Nucloid v{}", env!("CARGO_PKG_VERSION"));

    let mem_map = mbi.memory_map_tag()
        .expect("No memory map provided by the bootloader");

    unsafe {
        PHYS_MEM_SIZE = physical_memory_size(&mem_map);
        LOWMEM_VA_END = LOWMEM_VA_START + lowmem_va_size(&mem_map);
    }

    debug!("phys_mem_size = 0x{:x}, va_size = 0x{:x}",
           R(PHYS_MEM_SIZE), R(LOWMEM_VA_END));
    debug!("Kernel image:   {:#?}", kernel_image());
    debug!("Text segment:   {:#?}", kernel_text_segment());
    debug!("Rodata segment: {:#?}", kernel_rodata_segment());

    info!("Setting up GDT...");
    gdt::setup_table();
    gdt::load_kernel_selectors();

    info!("Setting up interrupts...");
    irq::setup();

    let fb_info = mbi.framebuffer_tag().expect("No framebuffer");
    let fb_addr = PAddr(fb_info.address);
    let fb_width = fb_info.width;
    let fb_height = fb_info.height;
    let fb_pitch = fb_info.pitch;
    let fb_bpp = fb_info.bpp;

    info!("Setting up memory management...");
    arch::x86::mem::boot_setup(&mem_map);
    core::mem::forget(mbi); // FIXME: Multiboot info is invalidated

    // We can now activate and handle interruptions safely.
    pop_critical_region();

    let fb_bsize = fb_pitch as usize * fb_height as usize;
    let fb_vaddr = fb_addr.into_vaddr(fb_bsize >> 12).unwrap();

    let fb = VesaFramebuffer::new(
        fb_vaddr.0 as _,
        fb_width as usize,
        fb_height as usize,
        fb_pitch as usize,
        fb_bpp
    );

    debug!("fb ({fb_width}×{fb_height}) paddr = {:?}, vaddr = {:?}, size = {}",
           fb_addr, *fb_vaddr, fb_bsize);
    *KERNEL_TERMINAL.lock() = Some(Terminal::create(fb));

    main();
}
