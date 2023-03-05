/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 **************************************************************************** */

.set VA_BASE,   0xffff800000000000
.set VGA_ADDR,  0xb8000
.set VGA_SIZE,  0xfa0

/*
 * PML4:
 *   |- PDPT0       Identity-maps the first 16 Mio of physical addresses.
 *   |  \- PD0
 *   |     |- PDE0  (2 Mio pages, no page-tables.)
 *   |     |- PDE1
 *   |     |- PDE2
 *   |     |- PDE3
 *   |     |- PDE4
 *   |     |- PDE5
 *   |     |- PDE6
 *   |     \- PDE7
 *   \- PDPT256     VA starting at 0xffff8000_00000000
 *      \- PD0
 *         |- PDE0
 *         |  \- 512 PTEs
 *         |- PDE1
 *         |  \- 512 PTEs
 *         |- PDE2
 *         |  \- 512 PTEs
 *         |- PDE3
 *         |  \- 512 PTEs
 *         |- PDE4
 *         |  \- 512 PTEs
 *         |- PDE5
 *         |  \- 512 PTEs
 *         |- PDE6
 *         |  \- 512 PTEs
 *         \- PDE7
 *            \- 512 PTEs
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
    call    check_has_longmode

    // Clear PML4 entries
    mov     $(boot_pml4 - VA_BASE), %edi
    xor     %eax, %eax
    mov     $1024, %ecx
    rep stosl

    // Setup PML4
    mov     $(boot_pml4 - VA_BASE), %edi
    movl    $(boot_pdpt0 - VA_BASE), 0(%edi)
    movl    $(boot_pdpt256 - VA_BASE), 256*8(%edi)
    orl     $((1 << 0) | (1 << 1)), 0(%edi)
    orl     $((1 << 0) | (1 << 1)), 256*8(%edi)

    // Clear PML4[0] PDPT
    mov     $(boot_pdpt0 - VA_BASE), %edi
    xor     %eax, %eax
    mov     $1024, %ecx
    rep stosl

    // Setup PML4[0] PDPT
    mov     $(boot_pdpt0 - VA_BASE), %edi
    movl    $(boot_pdpt0_pd0 - VA_BASE), (%edi)
    orl     $((1 << 0) | (1 << 1)), (%edi) // Present and writable

    // Clear PML4[256] PDPT
    mov     $(boot_pdpt256 - VA_BASE), %edi
    xor     %eax, %eax
    mov     $1024, %ecx
    rep stosl

    // Setup PML4[256] PDPT
    mov     $(boot_pdpt256 - VA_BASE), %edi
    movl    $(boot_pdpt256_pd0 - VA_BASE), (%edi)
    orl     $((1 << 0) | (1 << 1)), (%edi) // Present and writable

    // Clear PML4[0].PDPT[0] PD
    mov     $(boot_pdpt0_pd0 - VA_BASE), %edi
    xor     %eax, %eax
    mov     $1024, %ecx
    rep stosl

    // Clear PML4[256].PDPT[0] PD
    mov     $(boot_pdpt256_pd0 - VA_BASE), %edi
    xor     %eax, %eax
    mov     $1024, %ecx
    rep stosl

    // Setup PML4[0].PDPT[0].PD[0] PDEs (8)
    mov     $(boot_pdpt0_pd0 - VA_BASE), %edi
    xor     %eax, %eax
    mov     $8, %ecx
1:  movl    %eax, (%edi)
    orl     $((1 << 0) | (1 << 1) | (1 << 7)), (%edi) // Present, writable, PS
    add     $8, %edi
    add     $(2 << 20), %eax
    loop    1b

    // Setup PML4[256].PDPT[0].PD[0] PDEs (8)
    mov     $(boot_pdpt256_pd0 - VA_BASE), %edi
    mov     $(boot_pdpt256_pd0_pts - VA_BASE), %eax
    mov     $8, %ecx
1:  movl    %eax, (%edi)
    orl     $((1 << 0) | (1 << 1)), (%edi) // Present, writable
    add     $8, %edi
    add     $4096, %eax
    loop    1b

    // Setup PML4[256].PDPT[0].PD[0..8].PT[0..512]
    mov     $(boot_pdpt256_pd0_pts - VA_BASE), %edi
    xor     %eax, %eax
    mov     $(8 * 512), %ecx
1:  movl    %eax, (%edi)
    orl     $((1 << 0) | (1 << 1)), (%edi) // Present, writable
    add     $8, %edi
    add     $4096, %eax
    loop    1b

    // Switching to long-mode (compatibility)
    mov     $0xc0000080, %ecx // IA32_EFER
    rdmsr
    or      $(1 << 8), %eax  // LME
    or      $(1 << 11), %eax // NXE
    wrmsr

    // Enable PAE and PSE
    mov     %cr4, %eax
    or      $((1 << 4) | (1 << 5)), %eax
    mov     %eax, %cr4

    // Enable paging
    mov     $(boot_pml4 - VA_BASE), %eax
    mov     %eax, %cr3
    mov     %cr0, %eax
    or      $(1 << 31), %eax # CR0.PG
    or      $(1 << 16), %eax # CR0.WP
    mov     %eax, %cr0

    // Load a new GDT with a 64 bits code segment descriptor
    lgdt    (boot_gdt64_ptr - VA_BASE)
    mov     $0x10, %ax
    mov     %ax, %ss
    mov     %ax, %ds
    mov     %ax, %es
    mov     %ax, %fs
    mov     %ax, %gs

    // Switch to 64 bit mode
    jmp     $0x08, $(1f - VA_BASE)
.code64
1:
    // Let's use the virtual address for RIP
    movabs  $1f, %rax
    jmp     *%rax
1:
    // Update stack pointer to a virtual address
    mov     $VA_BASE, %rax
    add     %rax, %rsp

    // Unmap PDPT0, no longer needed
    //movq    $0, boot_pml4

    pop     %rdi // Multiboot pointer argument
    and     $(~0b111), %rsp // Align stack to 8 bytes

    jmp     arch_init

.code32

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

check_has_longmode:
    mov     $0x80000000, %eax
    cpuid
    cmp     $0x80000001, %eax
    jb      1f
    mov     $0x80000001, %eax
    cpuid
    test    $(1 << 29), %edx
    jz      1f
    ret

1:  mov     $(VGA_ADDR + 0 * 160), %edi
    mov     $(msg_no_longmode - VA_BASE), %esi
    call prints
2:  hlt
    jmp 2b

.section .boot_page_tables

    .global boot_pml4

.align 4096
boot_pml4:          .skip 4096

boot_pdpt0:         .skip 4096
boot_pdpt256:       .skip 4096

boot_pdpt0_pd0:     .skip 4096
boot_pdpt256_pd0:   .skip 4096

boot_pdpt256_pd0_pts: .skip 4096 * 8

.section .boot_stack

    .global boot_stack_bottom_guard
boot_stack_bottom_guard:
    .skip 4096 // Guard page

    .skip 32 * 4096

boot_stack_top:
    .skip 4096 // Guard page

.section .rodata

    .global boot_gdt64
boot_gdt64:
    // Null GDT entry
    .byte   0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00

    /*
     * GDT entry for 64 bits code segment.
     *   base  = 0x0000'0000
     *   limit = 0xf'ffff    (4 Gio)
     *   type  = 1011 (code, non-conformant, readable, accessed)
     *   S     = 1    (segment descriptor)
     *   DPL   = 00   (ring level)
     *   P     = 1    (physical memory)
     *   flags = 1010 (global, limit in pages, 64 bits instructions, (non-used))
     */
    .byte   0xff, 0xff, 0x00, 0x00, 0x00, 0b10011011, 0b10101111, 0x00

    /*
     * GDT entry for data segment.
     *   base  = 0x0000'0000
     *   limit = 0xf'ffff    (4 Gio)
     *   type  = 0011 (data, not stack, writable, accessed)
     *   S     = 1    (segment descriptor)
     *   DPL   = 00   (ring level)
     *   P     = 1    (physical memory)
     *   flags = 1100 (global, limit in pages, (non-used))
     */
    .byte   0xff, 0xff, 0x00, 0x00, 0x00, 0b10010011, 0b11001111, 0x00

boot_gdt64_ptr:
    .short  24 // octets, for 3 entries
    .long   boot_gdt64 - VA_BASE

msg_no_cpuid:   .asciz "ERROR: The CPU does not support CPUID"
msg_no_longmode:.asciz "The CPU does not support 64 bit mode"
msg_ok:         .asciz "OK!"
