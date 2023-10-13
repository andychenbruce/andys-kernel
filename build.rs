fn main() {
    println!("cargo:rustc-link-arg=-T./src/arch/riscv/linker.ld");
}
