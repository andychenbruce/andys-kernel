pub mod csr_stuff;
pub mod entry;
pub mod interrupt;
pub mod mmu;
pub mod trap;

use mmu::VirtualMemoryScheme;

use crate::kprintln;

lazy_static::lazy_static! {
    pub static ref WRITER: spin::Mutex<crate::uart::UartWriter> = spin::Mutex::new(unsafe { crate::uart::UartWriter::new(UART_ADDR) });
    pub static ref ALLOCATOR: spin::Mutex<crate::heap_alloc::AndyAllocator<4096>> = unsafe {spin::Mutex::new(crate::heap_alloc::AndyAllocator::new(HEAP_START, HEAP_END)) };
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

fn poweroff() {
    kprintln!("poweroff now");
    unsafe {
        let syscon_ptr: *mut u32 = crate::arch::special::SYSCON_ADDR as *mut u32;
        syscon_ptr.write_volatile(0x5555);
    }
}

fn reboot() {
    kprintln!("reboot now");
    unsafe {
        let syscon_ptr: *mut u32 = crate::arch::special::SYSCON_ADDR as *mut u32;
        syscon_ptr.write_volatile(0x7777);
    }
}

#[no_mangle]
pub fn kinit() {
    kprintln!("早上好");
    let trap_stack = ALLOCATOR.lock().allocate(10).unwrap();

    unsafe {
        core::arch::asm!("csrw mscratch, {}", in(reg) trap_stack);
    }

    let mut mem_table = mmu::riscv::sv39_paging::Sv39::new(&mut ALLOCATOR.lock()).unwrap();

    mmu::riscv::sv39_paging::sv39_setup_identity_mapping(&mut ALLOCATOR.lock(), &mut mem_table)
        .unwrap();

    mmu::assert_identity_map(&mem_table);

    unsafe {
        mem_table.activate().unwrap();
    }

    unsafe {
        let val = 0b111111111 << 10; //都开?
        core::arch::asm!("csrw mie, {}", in(reg) val);
        let val2 = 1 << 3;
        core::arch::asm!("csrw mstatus, {}", in(reg) val2);
    }
    interrupt::set_threshold(0);
    interrupt::enable(10);
    interrupt::set_priority(10, 1);
}

pub fn abort() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
