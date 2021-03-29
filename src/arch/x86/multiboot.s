.set F_ALIGN,    1 << 0 # Modules are 4 KiB aligned
.set F_MEMINFO,  1 << 1 # Ask for memory information
.set F_VIDINFO,  1 << 2

.set MAGIC,      0x1badb002
.set FLAGS,      F_ALIGN | F_MEMINFO | F_VIDINFO
.set CHECKSUM,   -(MAGIC + FLAGS)

.section .multiboot

    .long   MAGIC
    .long   FLAGS
    .long   CHECKSUM

    .long   0
    .long   0
    .long   0
    .long   0
    .long   0

    .long   1
    .long   80
    .long   25
    .long   0
