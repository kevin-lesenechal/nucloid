/*struct MultibootInfo
{
    // Multiboot info version number
    flags: u32,

    // Available memory from BIOS
    mem_lower: u32,
    mem_upper: u32,

    // "root" partition
    boot_device: u32,

    // Kernel command line
    cmdline: u32,

    // Boot-Module list
    mods_count: u32,
    mods_addr: u32,

    union
    {
        multiboot_aout_symbol_table_t aout_sym,
        multiboot_elf_section_header_table_t elf_sec,
    } u,

    // Memory Mapping buffer
    mmap_length: u32,
    mmap_addr: u32,

    // Drive Info buffer
    drives_length: u32,
    drives_addr: u32,

    // ROM configuration table
    config_table: u32,

    // Boot Loader Name
    boot_loader_name: u32,

    // APM table
    apm_table: u32,

    // Video
    vbe_control_info: u32,
    vbe_mode_info: u32,
    vbe_mode: u16,
    vbe_interface_seg: u16,
    vbe_interface_off: u16,
    vbe_interface_len: u16,

    framebuffer_addr: u64,
    framebuffer_pitch: u32,
    framebuffer_width: u32,
    framebuffer_height: u32,
    framebuffer_bpp: u8,
}*/
