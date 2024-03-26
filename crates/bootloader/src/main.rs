#![no_main]
#![no_std]

mod elf_mapper;
mod frame_allocator;
mod make_stack;
mod read_file;

use uefi::prelude::*;

use x86_64::structures::paging::{PageSize, PhysFrame, Mapper};
use x86_64::VirtAddr;
use x86_64::{structures::paging::Size4KiB, PhysAddr};

static mut WRITER: AndyWriter = AndyWriter {};

const NUM_STACK_PAGES: u64 = 100;
const UEFI_PHYSICAL_OFFSET: u64 = 0; //UEFI uses identity mapping

#[entry]
fn main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    eprintln!("Reading kernel file");
    let kernel_slice = read_file::load_file_from_disk("efi\\kernel\\kernel", image, &st).unwrap();
    let kernel_addr: PhysAddr = PhysAddr::new(kernel_slice as *const [u8] as *const u8 as u64);
    assert!(kernel_addr.is_aligned(Size4KiB::SIZE));
    eprintln!("Finished reading kernel file");

    eprintln!("Parsing ELF file");
    let kernel_elf = xmas_elf::ElfFile::new(kernel_slice).unwrap();
    eprintln!("Successfully parsed ELF file");

    eprintln!("exiting boot services");
    let (system_table, mut memory_map) =
        st.exit_boot_services(uefi::table::boot::MemoryType::LOADER_DATA);

    eprintln!("enabling write protection on ring 0");
    enable_write_protect_bit();
    eprintln!("enabling no execute flag");
    enable_nxe_bit();

    eprintln!("bruh");
    memory_map.sort();
    let mut frame_allocator =
        frame_allocator::AndyFrameAllocator::new(memory_map.entries().copied());

    eprintln!("Mapping ELF file to virtual memory");
    let (mut kernel_page_table, kernel_page_table_top_frame) =
        elf_mapper::map_elf_into_memory(kernel_addr, &kernel_elf, &mut frame_allocator);
    eprintln!("Successfully mapped ELF file to virtual memory");

    let entry_point = VirtAddr::new(kernel_elf.header.pt2.entry_point());
    let stack_top = make_stack::make_stack(&mut frame_allocator, NUM_STACK_PAGES);

    eprintln!("identity mapping context switch function");
    let context_switch_function = PhysAddr::new(context_switch as *const () as u64);
    let context_switch_function_start_frame: PhysFrame =
        PhysFrame::containing_address(context_switch_function);
    for frame in PhysFrame::range_inclusive(
        context_switch_function_start_frame,
        context_switch_function_start_frame + 1,
    ) {
        unsafe {
            let tlb = kernel_page_table
                .identity_map(
                    frame,
                    x86_64::structures::paging::PageTableFlags::PRESENT,
                    &mut frame_allocator,
                )
                .unwrap();
            tlb.flush();
        };
    }
    eprintln!("DONE identity mapping context switch function");

    unsafe {
        context_switch(kernel_page_table_top_frame, stack_top, entry_point);
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::arch::asm;

    eprintln!("error: {}", info);

    loop {
        unsafe { asm!("cli; hlt") };
    }
}

pub fn print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    unsafe {
        WRITER.write_fmt(args).unwrap();
    }
}

struct AndyWriter {}

impl core::fmt::Write for AndyWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            unsafe {
                let port: u16 = 0xe9; //qemu -debugcon
                core::arch::asm!("outb %al, %dx", in("al") b, in("dx") port, options(att_syntax));
            };
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => ($crate::print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprintln {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::eprint!("{}\n", format_args!($($arg)*)));
}

unsafe fn context_switch(page_table: PhysFrame, stack_top: VirtAddr, entry_point: VirtAddr) -> ! {
    unsafe {
        core::arch::asm!(
            r#"
            xor rbp, rbp
            mov cr3, {}
            mov rsp, {}
            jmp {}
            "#,
            in(reg) page_table.start_address().as_u64(),
            in(reg) stack_top.as_u64(),
            in(reg) entry_point.as_u64(),
        );
    }

    unreachable!()
}

fn enable_nxe_bit() {
    use x86_64::registers::control::{Efer, EferFlags};
    unsafe { Efer::update(|efer| *efer |= EferFlags::NO_EXECUTE_ENABLE) }
}

fn enable_write_protect_bit() {
    use x86_64::registers::control::{Cr0, Cr0Flags};
    unsafe { Cr0::update(|cr0| *cr0 |= Cr0Flags::WRITE_PROTECT) };
}
