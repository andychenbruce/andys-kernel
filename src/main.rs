#![no_std]
#![no_main]

mod arch;
mod heap_alloc;
mod uart;

use arch::special::WRITER;

pub fn print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => ($crate::print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! kprintln {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::kprint!("{}\n", format_args!($($arg)*)));
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    kprintln!("PANIC: {:?}", info);
    arch::special::abort()
}

#[no_mangle]
fn kmain() -> ! {
    loop {}
}
