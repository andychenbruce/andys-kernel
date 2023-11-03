pub mod entry;

lazy_static::lazy_static! {
    pub static ref WRITER: spin::Mutex<crate::uart::UartWriter> = todo!();
    pub static ref ALLOCATOR: spin::Mutex<crate::heap_alloc::AndyAllocator<4096>> = todo!();
}

pub fn abort() -> ! {
    unsafe {
        core::arch::asm!("cli");
        loop {
            core::arch::asm!("hlt");
        }
    }
}

#[no_mangle]
pub fn kinit() {
    todo!()
}
