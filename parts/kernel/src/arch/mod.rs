#[cfg(target_arch = "riscv64")]
#[path = "riscv64/mod.rs"]
pub mod special;

#[cfg(target_arch = "x86_64")]
#[path = "x86_64/mod.rs"]
pub mod special;
