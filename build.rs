fn main() {
    let target = std::env::var("TARGET")
        .expect("Expected TARGET environment variable");

    let mut build = cc::Build::new();

    if target == "x86_64-nucloid" {
        build
            .file("src/arch/x86/multiboot.s")
            .file("src/arch/x86/start64.s");
    } else if target == "i686-nucloid" {
        build
            .file("src/arch/x86/multiboot.s")
            .file("src/arch/x86/start32.S")
            .file("src/arch/x86/isr_entry32.s");
    } else {
        panic!("Unsupported target '{}'", target);
    }

    build.compile("nucloid_c");
}
