#![no_std]
#![no_main]

mod arch;
mod heap_alloc;
mod uart;
use arch::special::mmu::VirtualMemoryScheme;

lazy_static::lazy_static! {
    static ref WRITER: spin::Mutex<uart::UartWriter> = spin::Mutex::new(unsafe { uart::UartWriter::new(UART_ADDR) });
    static ref ALLOCATOR: spin::Mutex<heap_alloc::AndyAllocator<4096>> = unsafe {spin::Mutex::new(heap_alloc::AndyAllocator::new(HEAP_START, HEAP_END)) };
}
extern "C" {
    //.text
    static TEXT_START: usize;
    static TEXT_END: usize;
    //.rodata
    static RODATA_START: usize;
    static RODATA_END: usize;

    //.data
    static DATA_START: usize;
    static DATA_END: usize;

    //.bss
    static BSS_START: usize;
    static BSS_END: usize;

    static STACK_TOP: usize;
    static STACK_BOT: usize;
    static HEAP_START: usize;
    static HEAP_END: usize;

    //syscon mmio
    static SYSCON_ADDR: usize;

    static UART_ADDR: usize;
}

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

fn poweroff() {
    kprintln!("poweroff now");
    unsafe {
        let syscon_ptr: *mut u32 = SYSCON_ADDR as *mut u32;
        syscon_ptr.write_volatile(0x5555);
    }
}

fn reboot() {
    kprintln!("reboot now");
    unsafe {
        let syscon_ptr: *mut u32 = SYSCON_ADDR as *mut u32;
        syscon_ptr.write_volatile(0x7777);
    }
}

#[no_mangle]
fn abort() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    kprintln!("PANIC: {:?}", info);
    abort();
}

#[no_mangle]
fn kinit() {
    kprintln!("早上好");

    let trap_stack = ALLOCATOR.lock().allocate(10).unwrap();

    unsafe {
        core::arch::asm!("csrw mscratch, {}", in(reg) trap_stack);
    }

    let mut mem_table =
        arch::special::mmu::riscv::sv39_paging::Sv39::new(&mut ALLOCATOR.lock()).unwrap();

    arch::special::mmu::riscv::sv39_paging::sv39_setup_identity_mapping(
        &mut ALLOCATOR.lock(),
        &mut mem_table,
    )
    .unwrap();

    arch::special::mmu::assert_identity_map(&mem_table);

    unsafe {
        mem_table.activate().unwrap();
    }

    unsafe {
        let val = 0b111111111 << 10; //都开?
        core::arch::asm!("csrw mie, {}", in(reg) val);
        let val2 = 1 << 3;
        core::arch::asm!("csrw mstatus, {}", in(reg) val2);
    }
    arch::special::interrupt::set_threshold(0);
    arch::special::interrupt::enable(10);
    arch::special::interrupt::set_priority(10, 1);
}

#[no_mangle]
fn kmain() -> ! {
    loop {}
}
