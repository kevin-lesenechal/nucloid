/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use alloc::boxed::Box;
use core::fmt::{Debug, Formatter};
use core::mem::size_of;
use core::ptr::addr_of;
use core::slice;
use gimli::{
    BaseAddresses, CfaRule, EhFrame, EhFrameHdr, EhHdrTable, EndianSlice,
    LittleEndian, ParsedEhFrameHdr, Register, RegisterRule, UnwindContext,
    UnwindSection,
};

use crate::arch::cpu::MachineState;
use crate::mem::VAddr;

pub struct Backtrace {
    unwinder: Unwinder,
}

pub struct CallFrame {
    pub pc: VAddr,
    pub symbol: Option<&'static str>,
    pub sym_off: Option<usize>,
    pub file_line: Option<(&'static str, u32)>,
}

impl Backtrace {
    pub fn from_machine_state(machine: &MachineState) -> Self {
        Self {
            unwinder: Unwinder::new(
                EhInfo::new(),
                RegisterSet::from_machine_state(machine),
            ),
        }
    }
}

impl Iterator for Backtrace {
    type Item = CallFrame;

    fn next(&mut self) -> Option<Self::Item> {
        let pc = self.unwinder.next().ok()??;

        if pc == 0 {
            return None;
        }

        Some(CallFrame {
            pc: VAddr(pc as usize),
            symbol: None,
            sym_off: None,
            file_line: None,
        })
    }
}

extern "C" {
    static __kernel_eh_frame_hdr: u8;
    static __kernel_eh_frame_hdr_end: u8;
    static __kernel_eh_frame: u8;
    static __kernel_eh_frame_end: u8;
}

#[derive(Debug)]
enum UnwinderError {
    UnexpectedRegister(Register),
    UnsupportedCfaRule,
    CfaRuleUnknownRegister(Register),
    UnimplementedRegisterRule,
    NoUnwindInfo,
    NoPcRegister,
    NoReturnAddr,
}

struct EhInfo {
    base_addrs: BaseAddresses,
    hdr: &'static ParsedEhFrameHdr<EndianSlice<'static, LittleEndian>>,
    hdr_table: EhHdrTable<'static, EndianSlice<'static, LittleEndian>>,
    eh_frame: EhFrame<EndianSlice<'static, LittleEndian>>,
}

impl EhInfo {
    fn new() -> Self {
        let hdr = unsafe { addr_of!(__kernel_eh_frame_hdr) };
        let hdr_len = (unsafe { addr_of!(__kernel_eh_frame_hdr_end) } as usize) - (hdr as usize);
        let eh_frame = unsafe { addr_of!(__kernel_eh_frame) };
        let eh_frame_len = (unsafe { addr_of!(__kernel_eh_frame_end) } as usize) - (eh_frame as usize);

        let mut base_addrs = BaseAddresses::default();
        base_addrs = base_addrs.set_eh_frame_hdr(hdr as u64);

        let hdr = Box::leak(Box::new(EhFrameHdr::new( // TODO: remove Box
            unsafe { slice::from_raw_parts(hdr, hdr_len) },
            LittleEndian,
        ).parse(&base_addrs, size_of::<usize>() as u8).unwrap()));

        base_addrs = base_addrs.set_eh_frame(eh_frame as u64);

        let eh_frame = EhFrame::new(
            unsafe { slice::from_raw_parts(eh_frame, eh_frame_len) },
            LittleEndian,
        );

        Self {
            base_addrs,
            hdr,
            hdr_table: hdr.table().unwrap(),
            eh_frame,
        }
    }
}

struct Unwinder {
    eh_info: EhInfo,
    unwind_ctx: UnwindContext<EndianSlice<'static, LittleEndian>>,
    regs: RegisterSet,
    cfa: u64,
    is_first: bool,
}

impl Debug for Unwinder {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Unwinder")
            .field("regs", &self.regs)
            .field("cfa", &self.cfa)
            .finish()
    }
}

impl Unwinder {
    fn new(
        eh_info: EhInfo,
        register_set: RegisterSet,
    ) -> Self {
        Self {
            eh_info,
            unwind_ctx: UnwindContext::new(), // TODO: no alloc
            regs: register_set,
            cfa: 0,
            is_first: true,
        }
    }

    fn next(&mut self) -> Result<Option<u64>, UnwinderError> {
        let pc = self.regs.get_pc().ok_or(UnwinderError::NoPcRegister)?;

        if self.is_first {
            self.is_first = false;
            return Ok(Some(pc));
        }

        let row = self.eh_info.hdr_table.unwind_info_for_address(
            &self.eh_info.eh_frame,
            &self.eh_info.base_addrs,
            &mut self.unwind_ctx,
            pc,
            |section, bases, offset| section.cie_from_offset(bases, offset),
        ).map_err(|_| UnwinderError::NoUnwindInfo)?;

        match row.cfa() {
            CfaRule::RegisterAndOffset { register, offset } => {
                let reg_val = self.regs.get(*register)
                    .ok_or(UnwinderError::CfaRuleUnknownRegister(*register))?;
                self.cfa = (reg_val as i64 + offset) as u64;
            },
            _ => return Err(UnwinderError::UnsupportedCfaRule),
        }

        for reg in RegisterSet::iter() {
            match row.register(reg) {
                RegisterRule::Undefined => {
                    self.regs.undef(reg)
                },
                RegisterRule::SameValue => (),
                RegisterRule::Offset(offset) => {
                    let ptr = (self.cfa as i64 + offset) as u64 as *const usize;
                    self.regs.set(reg, unsafe { ptr.read() } as u64)?;
                },
                _ => return Err(UnwinderError::UnimplementedRegisterRule),
            }
        }

        let ret = self.regs.get_ret().ok_or(UnwinderError::NoReturnAddr)?;
        self.regs.set_pc(ret);
        self.regs.set_stack_ptr(self.cfa);

        Ok(Some(ret))
    }
}

#[cfg(target_arch = "x86_64")]
mod arch {
    use gimli::{Register, X86_64};
    use crate::arch::cpu::MachineState;
    use crate::backtrace::UnwinderError;

    #[derive(Debug, Default)]
    pub(super) struct RegisterSet {
        rip: Option<u64>,
        rsp: Option<u64>,
        rbp: Option<u64>,
        ret: Option<u64>,
    }

    impl RegisterSet {
        pub(super) fn from_machine_state(machine: &MachineState) -> Self {
            Self {
                rip: Some(machine.rip),
                rsp: Some(machine.rsp),
                rbp: Some(machine.rbp),
                ret: None,
            }
        }

        pub(super) fn get(&self, reg: Register) -> Option<u64> {
            match reg {
                X86_64::RSP => self.rsp,
                X86_64::RBP => self.rbp,
                X86_64::RA => self.ret,
                _ => None,
            }
        }

        pub(super) fn set(&mut self, reg: Register, val: u64) -> Result<(), UnwinderError> {
            *match reg {
                X86_64::RSP => &mut self.rsp,
                X86_64::RBP => &mut self.rbp,
                X86_64::RA => &mut self.ret,
                _ => return Err(UnwinderError::UnexpectedRegister(reg)),
            } = Some(val);

            Ok(())
        }

        pub(super) fn undef(&mut self, reg: Register) {
            *match reg {
                X86_64::RSP => &mut self.rsp,
                X86_64::RBP => &mut self.rbp,
                X86_64::RA => &mut self.ret,
                _ => return,
            } = None;
        }

        pub(super) fn get_pc(&self) -> Option<u64> {
            self.rip
        }

        pub(super) fn set_pc(&mut self, val: u64) {
            self.rip = Some(val);
        }

        pub(super) fn get_ret(&self) -> Option<u64> {
            self.ret
        }

        pub(super) fn set_stack_ptr(&mut self, val: u64) {
            self.rsp = Some(val);
        }

        pub(super) fn iter() -> impl Iterator<Item=Register> {
            [X86_64::RSP, X86_64::RBP, X86_64::RA].into_iter()
        }
    }
}

use arch::RegisterSet;
