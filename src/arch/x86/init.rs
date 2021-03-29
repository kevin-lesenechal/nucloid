/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use multiboot::information::{Multiboot, MemoryManagement, FramebufferTable,
                             ColorInfoType, SymbolType};

use crate::arch::x86::driver::vga::Vga;
use crate::arch::x86::{gdt, irq};
use crate::driver::vga::VgaScreen;
use crate::main;
use crate::mem::{PAddr, PHYS_MEM_SIZE, VA_SIZE};
use crate::{print, println};
use crate::arch::sync::{push_critical_region, pop_critical_region};
use crate::arch::mem::{VA_BASE, LOWMEM_SIZE};
use crate::arch;
use crate::misc::BinSize;
use core::cmp::min;
use crate::mem::frame::FRAME_ALLOCATOR;

struct MultibootMM;

impl MemoryManagement for MultibootMM {
    unsafe fn paddr_to_slice(&self,
                             addr: u64,
                             length: usize) -> Option<&'static [u8]> {
        Some(core::slice::from_raw_parts((addr + VA_BASE as u64) as _, length))
    }

    unsafe fn allocate(&mut self, _length: usize) -> Option<(u64, &mut [u8])> {
        unimplemented!()
    }

    unsafe fn deallocate(&mut self, _addr: u64) {
        unimplemented!()
    }
}

static mut MULTIBOOT_MM: MultibootMM = MultibootMM;
static mut MULTIBOOT: Option<Multiboot<'static, 'static>> = None;

#[no_mangle]
pub unsafe extern "C" fn arch_init(multiboot_info_pa: PAddr) -> ! {
    push_critical_region();

    MULTIBOOT = Some(
        Multiboot::from_ptr(multiboot_info_pa.0 as _, &mut MULTIBOOT_MM).unwrap()
    );
    let multiboot = MULTIBOOT.as_ref().unwrap();

    let mut vga = make_vga(multiboot.framebuffer_table());
    vga.move_cursor(0, 1);
    *crate::screen::VGA_SCREEN.lock() = Some(vga);

    print!(" -> Setting up GDT... ");
    gdt::setup_table();
    gdt::load_kernel_selectors();
    println!("OK");

    print!(" -> Setting up interrupts... ");
    irq::setup();
    println!("OK");

    let phys_mem_size = multiboot.upper_memory_bound()
        .expect("No memory bounds were provided by the bootloader") as u64;
    unsafe {
        PHYS_MEM_SIZE = (phys_mem_size << 10) + (1 << 20);
        VA_SIZE = VA_BASE + min(PHYS_MEM_SIZE, LOWMEM_SIZE as u64) as usize;
    }
    if PHYS_MEM_SIZE < 32 << 20 {
        panic!("Nucloid requires at least 32 Mio of RAM");
    }

    print!(" -> Setting up memory management... ");
    arch::mem::boot_setup();
    println!("OK");

    /*if let Some(bootloader) = multiboot.boot_loader_name() {
        println!("Booted by {}", bootloader);
    }

    if let Some(modules) = multiboot.modules() {
        for module in modules {
            println!("module = {:?}", module);
        }
    }

    if let Some(SymbolType::Elf(syms)) = multiboot.symbols() {
        println!("syms = {:?}", syms);
    }*/

    /*if let Some(mm_regions) = multiboot.memory_regions() {
        for mm_region in mm_regions {
            println!("  [{}] PA {:#010x} -> {:#010x}, size={:>10.2} Kio",
                     mm_region.memory_type() as u32,
                     mm_region.base_address(),
                     mm_region.base_address() + mm_region.length(),
                     mm_region.length() as f64 / 1024.0);
        }
    }*/

    /*println!("cmd_line = {:?}",
             multiboot.command_line());
    if let Some(fb) = multiboot.framebuffer_table() {
        println!("[framebuffer] type={:?}, w={}, h={}, p={}, bpp={}, addr={:#x}",
                 fb.color_info(), fb.width, fb.height, fb.pitch, fb.bpp, fb.addr);
    } else {
        println!("No framebuffer info from Multiboot");
    }*/

    // We can now activate and handle safely interruptions.
    pop_critical_region();

    let mut alloc_lock = FRAME_ALLOCATOR.lock();
    let alloc = alloc_lock.as_mut().unwrap();
    println!("alloc = {:?}", alloc.allocate(false));
    println!("alloc = {:?}", alloc.allocate(false));
    println!("alloc = {:?}", alloc.allocate(false));
    println!("alloc = {:?}", alloc.allocate(false));

    main();
}

fn make_vga(fb_info: Option<&FramebufferTable>) -> Vga {
    let addr;
    let size;
    let width;
    let height;

    if let Some(fb_info) = fb_info {
        match fb_info.color_info().unwrap() {
            ColorInfoType::Text => (),
            _ => panic!("Nucloid must be booted with a text-mode framebuffer"),
        };
        assert_eq!(fb_info.bpp & 0b111, 0);
        let bytes_per_ch = fb_info.bpp as usize >> 3;

        width = fb_info.width;
        height = fb_info.height;
        addr = VA_BASE + fb_info.addr as usize;
        size = width as usize * height as usize * bytes_per_ch;
    } else {
        width = 80;
        height = 25;
        addr = VA_BASE + 0xb8000;
        size = 0xfa0;
    }

    assert!(width <= 255 && height <= 255);

    unsafe { Vga::new(addr as _, size, width as u8, height as u8) }
}
