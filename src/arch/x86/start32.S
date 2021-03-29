/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 **************************************************************************** */

.set VA_BASE,   0xc0000000
.set VGA_ADDR,  0xb8000
.set VGA_SIZE,  0xfa0

/*
 * These are the very first instructions our kernel is executing right after the
 * Multiboot-compliant bootloader handed control. The CPU is in 32 bits
 * protected mode, interrupts and paging are disabled.
 *
 * Our task is to hand execution to Rust as quickly as possible. For this, we
 * have to enable paging and setup minimap virtual memory mappings; the Rust
 * memory manager will handle the rest. In 32 bits, the kernel lives in the
 * "upper half" of virtual memory (3 Gio for userspace and 1 Gio for the kernel)
 * starting at address VA 0xc000_0000. The Rust binary uses absolute addressing
 * and the linker script places the sections at VA 0xc010_0000; since paging is
 * not enabled, current memory accesses are in linear address space, which, with
 * the flat memory model the bootloader has set up for us, identity maps
 * physical memory.
 *
 * All symbols defined or accessed in this file refer to virtual memory at
 * address VA 0xc000_0000 and beyond; we cannot use those addresses directly
 * until the proper pagination mappings are put in place. The linker script is
 * written so that any data in VA 0xc000_0000 + x is loaded at PA x. More
 * precisely, our kernel is loaded at address PA 0x0010_0000 (1 Mio) into the
 * second RAM area (14 Mio), please mind the ISA memory hole at PA 0x0f00_0000.
 * So, to retrieve the physical memory address at which data has been loaded by
 * the bootloader from some symbol's virtual address, we need to substract the
 * value VA_BASE (0xc000_0000): PA = VA - VA_BASE.
 *
 * Our paging organisation first enables PAE and use the fourth and last entry
 * of the PDPT (managing VA 0xc000_0000..=0xffff_ffff) for use for kernel space;
 * into which, 8 PDs of 512 PDEs are created to map the first 16 Mio of kernel
 * virtual space (VA_BASE) to PA 0x0010_0000 where the kernel is loaded.
 *
 * The first entry of the PDPT (managing VA 0x0000_0000..0x4000_0000) is also
 * used to temporarily identity map the VA/PA 0x0010_0000 to be able to continue
 * execution after setting CR0.PG bit (%eip still refers to physical addresses).
 * Once we jumped execution in the kernel upper half, we can unmap the PD0 and
 * finally jump into Rust land.
 *
 * PDPT:
 *   |- PD0           VA 0x0000_0000..0x0080_0000 -> PA 0x0000_0000..0x0080_0000
 *   |  |- PT0
 *   |  |  \- 512 PTE
 *   |  |- PT1
 *   |  |  \- 512 PTE
 *   |  |- PT2
 *   |  |  \- 512 PTE
 *   |  \- PT3
 *   |     \- 512 PTE
 *   |- PD1           (unmapped)
 *   |- PD2           (unmapped)
 *   \- PD3           VA 0xc000_0000..0xc100_0000 -> PA 0x0000_0000..0x0100_0000
 *      |- PT0
 *      |  \- 512 PTE
 *      |- PT1
 *      |  \- 512 PTE
 *      |- PT3
 *      |  \- 512 PTE
 *      |- PT4
 *      |  \- 512 PTE
 *      |- PT5
 *      |  \- 512 PTE
 *      |- PT6
 *      |  \- 512 PTE
 *      \- PT7
 *         \- 512 PTE
 *
 * We also provide the kernel with a temporary 32 Kio stack for the early boot
 * process before we setup a stack for each task and per CPU for IRQ handling.
 * The %esp is first setup to point to physical memory before PG is set, and
 * after set to its respective virtual address.
 */

.code32
.section .text

    .global _start
_start:
    // Multiboot specification says EFLAGS.DF is undefined, let's clear it.
    cld

    // Setup a temporary 32 Kio stack for the boot process.
    mov     $(boot_stack_top - VA_BASE), %esp
    movl    $0xdeadbeef, (boot_stack_bottom_guard - VA_BASE)

    // Immediately save the Multiboot info pointer passed by the bootloader as
    // the first argument of arch_init(), before we trash it with CPUID calls.
    push    %ebx

    call    clear_screen
    call    check_has_cpuid
    call    check_pae_pse

    // Clear 4 entries of PDPT
    mov     $(boot_pdpt - VA_BASE), %edi
    xor     %eax, %eax
    mov     $8, %ecx
    rep stosl

    // Setup PDPT
    mov     $(boot_pdpt - VA_BASE), %edi
    movl    $(boot_pd0 - VA_BASE), 0(%edi)
    movl    $(boot_pd3 - VA_BASE), 24(%edi)
    orl     $(1 << 0), 0(%edi)
    orl     $(1 << 0), 24(%edi)

    // Clear PD0 entries
    mov     $(boot_pd0 - VA_BASE), %edi
    xor     %eax, %eax
    mov     $1024, %ecx
    rep stosl

    // Setup PD0 entries (4)
    mov     $(boot_pd0 - VA_BASE), %edi
    mov     $(boot_pd0_pt0 - VA_BASE), %eax
    mov     $4, %ecx
1:  mov     %eax, (%edi)
    orl     $((1 << 0) | (1 << 1)), (%edi) // Present and writable
    add     $8, %edi
    add     $4096, %eax
    loop    1b

    // Setup PD0 PTs entries (512 entries for 4 PTs)
    mov     $(boot_pd0_pt0 - VA_BASE), %edi
    xor     %eax, %eax
    mov     $(4 * 512), %ecx
1:  mov     %eax, (%edi)
    orl     $((1 << 0) | (1 << 1)), (%edi) // Present and writable
    add     $8, %edi
    add     $4096, %eax
    loop    1b

    // Clear PD3 entries
    mov     $(boot_pd3 - VA_BASE), %edi
    xor     %eax, %eax
    mov     $1024, %ecx
    rep stosl

    // Setup PD3 entries (8)
    mov     $(boot_pd3 - VA_BASE), %edi
    mov     $(boot_pd3_pt0 - VA_BASE), %eax
    mov     $8, %ecx
1:  mov     %eax, (%edi)
    orl     $((1 << 0) | (1 << 1)), (%edi) // Present and writable
    add     $8, %edi
    add     $4096, %eax
    loop    1b

    // Setup PD3 PTs entries (512 entries for 8 PTs)
    mov     $(boot_pd3_pt0 - VA_BASE), %edi
    xor     %eax, %eax
    mov     $(8 * 512), %ecx
1:  mov     %eax, (%edi)
    orl     $((1 << 0) | (1 << 1)), (%edi) // Present, writable
    add     $8, %edi
    add     $4096, %eax
    loop    1b

    // Enable PAE and PSE
    mov     %cr4, %eax
    or      $((1 << 4) | (1 << 5)), %eax
    mov     %eax, %cr4

    // Enable NX
    mov     $0xc0000080, %ecx // EFER
    rdmsr
    or      $(1 << 11), %eax // NXE
    wrmsr

    // Enable paging
    mov     $(boot_pdpt - VA_BASE), %eax
    mov     %eax, %cr3
    mov     %cr0, %eax
    or      $(1 << 31), %eax # CR0.PG
    or      $(1 << 16), %eax # CR0.WP
    mov     %eax, %cr0

    // Update stack pointer to a virtual address
    add     $VA_BASE, %esp

    mov     $1f, %eax
    jmp     *%eax
1:  // Reload page tables
    mov     %cr3, %eax
    mov     %eax, %cr3

    // Unmap PD0, no longer needed
    movl    $0, boot_pdpt

    // Let's continue execution in Rust land, this function must never return.
    call    arch_init

clear_screen:
    mov     $' ', %al
    mov     $0b00000111, %ah
    mov     $VGA_ADDR, %edi
    mov     $(VGA_SIZE >> 1), %ecx
    rep stosw
    ret

prints:
    lodsb
    cmp     $0, %al
    je      1f
    stosb
    mov     $0b00000111, %al
    stosb
    jmp     prints
1:  ret

check_has_cpuid:
    pushfl
    pop     %eax
    or      $(1 << 21), %eax
    push    %eax
    popfl

    pushfl
    popfl // pop %eax ?
    test    $(1 << 21), %eax
    jz      1f
    ret
1:  mov     $(VGA_ADDR + 0 * 160), %edi
    mov     $(msg_no_cpuid - VA_BASE), %esi
    call    prints
2:  hlt
    jmp 2b

check_pae_pse:
    mov     $0x01, %eax
    cpuid
    test    $(1 << 6), %edx
    jz      1f
    test    $(1 << 3), %edx
    jz      2f

    // Test for PAE NX
    mov     $0x80000000, %eax
    cpuid
    cmp     $0x80000001, %eax
    jb      3f
    mov     $0x80000001, %eax
    cpuid
    test    $(1 << 20), %edx
    jz      3f

    ret

1:  mov     $(VGA_ADDR + 0 * 160), %edi
    mov     $(msg_no_pae - VA_BASE), %esi
    call    prints
    jmp     4f
2:  mov     $(VGA_ADDR + 0 * 160), %edi
    mov     $(msg_no_pse - VA_BASE), %esi
    call    prints
    jmp     4f
3:  mov     $(VGA_ADDR + 0 * 160), %edi
    mov     $(msg_no_nx - VA_BASE), %esi
    call    prints
4:  hlt
    jmp 4b

.section .boot_page_tables

    .global boot_pdpt
    .global boot_pd0
    .global boot_pd3
    .global boot_pd0_pt0
    .global boot_pd3_pt0

boot_pdpt:      .skip 4096

boot_pd0:       .skip 4096
boot_pd3:       .skip 4096

boot_pd0_pt0:   .skip 4096 * 4

boot_pd3_pt0:   .skip 4096 * 8

.section .boot_stack

    .global boot_stack_bottom_guard
boot_stack_bottom_guard:
    .skip 4096 // Guard page

    .skip 16 * 4096

boot_stack_top:
    .skip 4096 # Guard page

.section .data

    .global kernel_cmdline
kernel_cmdline: .skip 4096

.section .rodata

msg_no_cpuid:   .asciz "ERROR: The CPU does not support CPUID"
msg_no_pae:     .asciz "ERROR: The CPU does not support PAE"
msg_no_nx:      .asciz "ERROR: The CPU does not support PAE NX"
msg_no_pse:     .asciz "ERROR: The CPU does not support PSE"
