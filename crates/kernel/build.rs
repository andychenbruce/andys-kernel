fn main() {
    match std::env::var("TARGET").unwrap().as_str() {
        "riscv64gc-unknown-none-elf" => {
            println!("cargo:rustc-link-arg=-T./crates/kernel/src/arch/riscv64/linker.ld")
        }
        "x86_64-unknown-none" => {
            println!("cargo:rustc-link-arg=-T./crates/kernel/src/arch/x86_64/linker.ld")
        }
        arch => panic!("unsupported arch: {}", arch),
    }
}
