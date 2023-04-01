fn main() {
    let target = &*std::env::var("TARGET")
        .expect("Expected TARGET environment variable");

    match target {
        "x86_64-nucloid" | "i686-nucloid" => build_x86(target),
        _ => (),
    }

    println!("cargo:rerun-if-changed=src/arch/x86/multiboot2.S");
    println!("cargo:rerun-if-changed=src/arch/x86/start32.S");
    println!("cargo:rerun-if-changed=src/arch/x86/start64.S");
    println!("cargo:rerun-if-changed=src/arch/x86/isr_entry32.S");
    println!("cargo:rerun-if-changed=src/arch/x86/isr_entry64.S");
    println!("cargo:rerun-if-changed=targets/i686.ld");
    println!("cargo:rerun-if-changed=targets/x86_64.ld");
}

fn build_x86(target: &str) {
    let mut build = make_c_builder();

    build.file("src/arch/x86/multiboot2.S");

    if target == "x86_64-nucloid" {
        build
            .file("src/arch/x86/start64.S")
            .file("src/arch/x86/isr_entry64.S");
    } else if target == "i686-nucloid" {
        build
            .file("src/arch/x86/start32.S")
            .file("src/arch/x86/isr_entry32.s");
    }

    build.link_lib_modifier("+whole-archive")
        .compile("nucloid_c");
}

fn make_c_builder() -> cc::Build {
    let mut build = cc::Build::new();

    set_compiler(&mut build);
    add_c_flags(&mut build);

    build
}

fn add_c_flags(build: &mut cc::Build) {
    build.pic(false);
    build.flag("-ffreestanding");
    build.flag("-nostdlib");
    build.flag("-mno-red-zone");
    build.flag("-mno-sse");
    build.flag("-mno-avx");
    build.flag("-no-pie");
}

fn set_compiler(build: &mut cc::Build) {
    let target = std::env::var("TARGET")
        .expect("Expected TARGET environment variable");

    let compiler = match target.as_ref() {
        "x86_64-nucloid" => "x86_64-elf-gcc",
        "i686-nucloid" => "i686-elf-gcc",
        other => panic!("unsupported target '{}'", other),
    };

    build.compiler(compiler);
}
