/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::arch::x86::driver::pic8259::Pic8259;
use crate::arch::x86::gdt::KERNEL_CODE_SELECTOR;
use crate::println;

use x86::segmentation::{DescriptorBuilder, GateDescriptorBuilder,
                        BuildDescriptor};
use x86::dtables::{lidt, DescriptorTablePointer};
use x86::Ring::Ring0;
use x86::irq::InterruptDescription;
use crate::arch::cpu::MachineState;
use crate::panic::panic_at_state;
use crate::arch::sync::{push_critical_region, pop_critical_region};
use crate::mem::{handle_pagefault, AccessAttempt, VAddr};

#[cfg(target_arch = "x86_64")]
#[repr(C, packed)]
struct IsrRegisters {
    rip:    u64,
    cs:     u64,
    rflags: u64,
    rsp:    u64,
    ss:     u64,
}

#[cfg(target_arch = "x86")]
#[repr(C, packed)]
struct IsrRegisters {
    eip:    u32,
    cs:     u32,
    eflags: u32,
}

#[cfg(target_arch = "x86_64")]
#[repr(C, packed)]
struct GPRegisters {
    rdi:    u64,
    rsi:    u64,
    rbp:    u64,
    rsp:    u64,
    rbx:    u64,
    rdx:    u64,
    rcx:    u64,
    rax:    u64,
    r8:     u64,
    r9:     u64,
    r10:    u64,
    r11:    u64,
    r12:    u64,
    r13:    u64,
    r14:    u64,
    r15:    u64,
}

#[cfg(target_arch = "x86")]
#[repr(C, packed)]
struct GPRegisters {
    edi:    u32,
    esi:    u32,
    ebp:    u32,
    esp:    u32,
    ebx:    u32,
    edx:    u32,
    ecx:    u32,
    eax:    u32,
}

extern {
    fn isr_entry_exception_0();
    fn isr_entry_exception_1();
    fn isr_entry_exception_2();
    fn isr_entry_exception_3();
    fn isr_entry_exception_4();
    fn isr_entry_exception_5();
    fn isr_entry_exception_6();
    fn isr_entry_exception_7();
    fn isr_entry_exception_8();
    fn isr_entry_exception_9();
    fn isr_entry_exception_10();
    fn isr_entry_exception_11();
    fn isr_entry_exception_12();
    fn isr_entry_exception_13();
    fn isr_entry_exception_14();
    fn isr_entry_exception_15();
    fn isr_entry_exception_16();
    fn isr_entry_exception_17();
    fn isr_entry_exception_18();
    fn isr_entry_exception_19();
    fn isr_entry_exception_20();
    fn isr_entry_exception_21();
    fn isr_entry_exception_22();
    fn isr_entry_exception_23();
    fn isr_entry_exception_24();
    fn isr_entry_exception_25();
    fn isr_entry_exception_26();
    fn isr_entry_exception_27();
    fn isr_entry_exception_28();
    fn isr_entry_exception_29();
    fn isr_entry_exception_30();
    fn isr_entry_exception_31();
    fn isr_entry_irq_0();
    fn isr_entry_irq_1();
    fn isr_entry_irq_2();
    fn isr_entry_irq_3();
    fn isr_entry_irq_4();
    fn isr_entry_irq_5();
    fn isr_entry_irq_6();
    fn isr_entry_irq_7();
    fn isr_entry_irq_8();
    fn isr_entry_irq_9();
    fn isr_entry_irq_10();
    fn isr_entry_irq_11();
    fn isr_entry_irq_12();
    fn isr_entry_irq_13();
    fn isr_entry_irq_14();
    fn isr_entry_irq_15();
}

static VECTORS: [unsafe extern fn(); 48] = [
    isr_entry_exception_0,
    isr_entry_exception_1,
    isr_entry_exception_2,
    isr_entry_exception_3,
    isr_entry_exception_4,
    isr_entry_exception_5,
    isr_entry_exception_6,
    isr_entry_exception_7,
    isr_entry_exception_8,
    isr_entry_exception_9,
    isr_entry_exception_10,
    isr_entry_exception_11,
    isr_entry_exception_12,
    isr_entry_exception_13,
    isr_entry_exception_14,
    isr_entry_exception_15,
    isr_entry_exception_16,
    isr_entry_exception_17,
    isr_entry_exception_18,
    isr_entry_exception_19,
    isr_entry_exception_20,
    isr_entry_exception_21,
    isr_entry_exception_22,
    isr_entry_exception_23,
    isr_entry_exception_24,
    isr_entry_exception_25,
    isr_entry_exception_26,
    isr_entry_exception_27,
    isr_entry_exception_28,
    isr_entry_exception_29,
    isr_entry_exception_30,
    isr_entry_exception_31,
    isr_entry_irq_0,
    isr_entry_irq_1,
    isr_entry_irq_2,
    isr_entry_irq_3,
    isr_entry_irq_4,
    isr_entry_irq_5,
    isr_entry_irq_6,
    isr_entry_irq_7,
    isr_entry_irq_8,
    isr_entry_irq_9,
    isr_entry_irq_10,
    isr_entry_irq_11,
    isr_entry_irq_12,
    isr_entry_irq_13,
    isr_entry_irq_14,
    isr_entry_irq_15,
];

static mut PIC8259: Option<Pic8259> = None;

#[cfg(target_arch = "x86_64")]
type DescriptorType = x86::bits64::segmentation::Descriptor64;
#[cfg(target_arch = "x86")]
type DescriptorType = x86::segmentation::Descriptor;

static mut IDT: [DescriptorType; 64] = [DescriptorType::NULL; 64];

unsafe fn get_pic() -> &'static mut Pic8259 {
    PIC8259.as_mut().unwrap()
}

pub unsafe fn setup() {
    #[cfg(target_arch = "x86_64")]
    type IdtType = u64;
    #[cfg(target_arch = "x86")]
    type IdtType = u32;

    let mut pic = Pic8259::new(0x20, 0xa0);
    pic.init(32, 40);
    PIC8259 = Some(pic);

    let mut vec = 0;

    for isr in VECTORS.iter() {
        let offset = core::mem::transmute::<_, usize>(*isr);

        IDT[vec] = <DescriptorBuilder as GateDescriptorBuilder<IdtType>>
                   ::interrupt_descriptor(
            KERNEL_CODE_SELECTOR,
            offset as IdtType
        ).present()
            .dpl(Ring0)
            .finish();
        vec += 1;
    }

    let ptr = DescriptorTablePointer::new(&IDT);
    lidt(&ptr);
}

#[cfg(target_arch = "x86_64")]
#[no_mangle]
unsafe extern "C" fn isr_exception(vec_i: usize, errc: usize,
                                   isr_regs: &IsrRegisters,
                                   regs: &GPRegisters) {
    let machine_state = MachineState {
        rax: regs.rax, rbx: regs.rbx, rcx: regs.rcx, rdx: regs.rdx,
        r8: regs.r8, r9: regs.r9, r10: regs.r10, r11: regs.r11,
        r12: regs.r12, r13: regs.r13, r14: regs.r14, r15: regs.r15,
        rdi: regs.rdi, rsi: regs.rsi, rsp: regs.rsp, rbp: regs.rbp,
        rip: isr_regs.rip, rflags: isr_regs.rflags,
        cs: isr_regs.cs as u16, ss: isr_regs.ss as u16,
        ds: 0, es: 0, fs: 0, gs: 0, // TODO: seg regs
    };

    handle_exception(vec_i, Some(errc), &machine_state);
}

#[cfg(target_arch = "x86")]
#[no_mangle]
unsafe extern "C" fn isr_exception(vec_i: usize,
                                   regs: GPRegisters,
                                   errc: usize,
                                   isr_regs: IsrRegisters) {
    let machine_state = MachineState {
        eax: regs.eax, ebx: regs.ebx, ecx: regs.ecx, edx: regs.edx,
        edi: regs.edi, esi: regs.esi, esp: regs.esp, ebp: regs.ebp,
        eip: isr_regs.eip, eflags: isr_regs.eflags,
        cs: isr_regs.cs as u16, ss: 0, ds: 0, es: 0, fs: 0, gs: 0, // TODO: seg regs
    };

    handle_exception(vec_i, Some(errc), &machine_state);
}

unsafe fn handle_exception(vec_i: usize,
                           errc: Option<usize>,
                           machine_state: &MachineState) {
    push_critical_region();

    let ex = x86::irq::EXCEPTIONS.get(vec_i as usize)
        .unwrap_or(&InterruptDescription {
        vector: 0,
        mnemonic: "#??",
        description: "(unknown)",
        irqtype: "???",
        source: "???",
    });

    if vec_i == x86::irq::PAGE_FAULT_VECTOR as usize {
        let errc = errc.expect("Page fault must provide an error code");
        let is_write = errc & (1 << 1) > 0;
        let is_exec = errc & (1 << 4) > 0;

        let addr = VAddr(unsafe { x86::controlregs::cr2() });

        let access = if is_exec {
            AccessAttempt::Execute
        } else if is_write {
            AccessAttempt::Write
        } else {
            AccessAttempt::Read
        };

        handle_pagefault(addr, access, machine_state);
        return;
    }

    if let Some(errc) = errc {
        panic_at_state(
            format_args!("Exception ({}; errc={}) {} {}",
                         vec_i, errc, ex.mnemonic, ex.description),
            Some(machine_state)
        );
    } else {
        panic_at_state(
            format_args!("Exception ({}) {} {}",
                         vec_i, ex.mnemonic, ex.description),
            Some(machine_state)
        );
    }
}

#[no_mangle]
unsafe extern "C" fn isr_irq(irq: usize) {
    push_critical_region();

    if irq == 0 {
        //print!(".");
    } else {
        println!("IRQ={}", irq);
    }

    get_pic().ack_irq(irq as u32);

    pop_critical_region();
}
