/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 **************************************************************************** */

.macro ISR_EXCEPTION      vec_n
    .global isr_entry_exception_\vec_n
    isr_entry_exception_\vec_n:
        push  $0
        pusha
        push  $\vec_n
        call  isr_exception
        add   $4, %esp
        popa
        add   $4, %esp
        iret
.endm

.macro ISR_EXCEPTION_ERRC vec_n
    .global isr_entry_exception_\vec_n
    isr_entry_exception_\vec_n:
        pusha
        push  $\vec_n
        call  isr_exception
        add   $4, %esp
        popa
        add   $4, %esp
        iret
.endm

.macro ISR_IRQ irq_n
    .global isr_entry_irq_\irq_n
    isr_entry_irq_\irq_n:
        pusha
        push  $\irq_n
        call  isr_irq
        add   $4, %esp
        popa
        iret
.endm

.text

.global isr_default_vec
isr_default_vec:
    iret

ISR_EXCEPTION      0  # DE  Divide-by-zero Error
ISR_EXCEPTION      1  # BD  Debug
ISR_EXCEPTION      2  #     Non-maskable Interrupt
ISR_EXCEPTION      3  # BP  Breakpoint
ISR_EXCEPTION      4  # OF  Overflow
ISR_EXCEPTION      5  # BR  Bound Range Exceeded
ISR_EXCEPTION      6  # UD  Invalid Opcode
ISR_EXCEPTION      7  # NM  Device Not Available
ISR_EXCEPTION_ERRC 8  # DF  Double Fault
ISR_EXCEPTION      9  #     Coprocessor Segment Overrun
ISR_EXCEPTION_ERRC 10 # TS  Invalid TSS
ISR_EXCEPTION_ERRC 11 # NP  Segment Not Present
ISR_EXCEPTION_ERRC 12 # SS  Stack-Segment Fault
ISR_EXCEPTION_ERRC 13 # GP  General Protection Fault
ISR_EXCEPTION_ERRC 14 # PF  Page Fault
ISR_EXCEPTION      15 #     (reserved)
ISR_EXCEPTION      16 # MF  x87 Floating-Point Exception
ISR_EXCEPTION      17 # AC  Alignment Check
ISR_EXCEPTION      18 # MC  Machine Check
ISR_EXCEPTION      19 # XM/XF  SIMD Floating-Point Exception
ISR_EXCEPTION      20 # VE  Virtualization Exception
ISR_EXCEPTION      21 #     (reserved)
ISR_EXCEPTION      22 #     (reserved)
ISR_EXCEPTION      23 #     (reserved)
ISR_EXCEPTION      24 #     (reserved)
ISR_EXCEPTION      25 #     (reserved)
ISR_EXCEPTION      26 #     (reserved)
ISR_EXCEPTION      27 #     (reserved)
ISR_EXCEPTION      28 #     (reserved)
ISR_EXCEPTION      29 #     (reserved)
ISR_EXCEPTION      30 # SX  Security Exception
ISR_EXCEPTION      31 #     (reserved)

ISR_IRQ 0
ISR_IRQ 1
ISR_IRQ 2
ISR_IRQ 3
ISR_IRQ 4
ISR_IRQ 5
ISR_IRQ 6
ISR_IRQ 7
ISR_IRQ 8
ISR_IRQ 9
ISR_IRQ 10
ISR_IRQ 11
ISR_IRQ 12
ISR_IRQ 13
ISR_IRQ 14
ISR_IRQ 15
