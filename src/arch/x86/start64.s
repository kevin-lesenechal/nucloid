.set VGA_ADDR,  0xb8000
.set VGA_SIZE,  0xfa0

.code32
.section .text

    .global start
_start:
    call    clear_screen
    mov     $0xb8000, %edi
    mov     $msg_init, %esi
    call    prints

    call    check_has_cpuid
    call    check_has_longmode

    mov     $(VGA_ADDR + 3 * 160), %edi
    mov     $msg_ok, %esi
    call    prints

    #call    arch_init

1:  hlt
    jmp     1b

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

1:  mov     $(VGA_ADDR + 2 * 160), %edi
    mov     $msg_no_longmode, %esi
    call prints
2:  hlt
    jmp 2b

.section .rodata

msg_init:       .asciz "Booted with Multiboot, going into long mode..."
msg_no_longmode:.asciz "The CPU does not support 64 bit mode"
msg_ok:         .asciz "OK!"
