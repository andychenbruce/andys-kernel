#![no_main]
#![no_std]

mod elf_mapper;
mod frame_allocator;

use core::ops::Deref;
use core::ops::DerefMut;
use uefi::prelude::*;
use uefi::proto::media::file::File;

use x86_64::structures::paging::PageSize;
use x86_64::{structures::paging::Size4KiB, PhysAddr};

static mut WRITER: AndyWriter = AndyWriter {};

const UEFI_PHYSICAL_OFFSET: u64 = 0; //UEFI uses identity mapping

#[entry]
fn main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    eprintln!("Reading kernel file");
    let kernel_slice = load_file_from_disk("efi\\kernel\\kernel", image, &st).unwrap();
    let kernel_addr: PhysAddr = PhysAddr::new(kernel_slice as *const [u8] as *const u8 as u64);
    assert!(kernel_addr.is_aligned(Size4KiB::SIZE));
    eprintln!("Finished reading kernel file");

    eprintln!("Parsing ELF file");
    let kernel_elf = xmas_elf::ElfFile::new(kernel_slice).unwrap();
    eprintln!("Successfully parsed ELF file");

    eprintln!("exiting boot services");
    let (system_table, mut memory_map) =
        st.exit_boot_services(uefi::table::boot::MemoryType::LOADER_DATA);

    memory_map.sort();
    let mut frame_allocator =
        frame_allocator::AndyFrameAllocator::new(memory_map.entries().copied());

    let (mut kernel_page_table, kernel_page_table_top_frame) =
        elf_mapper::map_elf_into_memory(kernel_addr, &kernel_elf, &mut frame_allocator);

    eprintln!("Done");
    Status::SUCCESS
}

fn load_file_from_disk(
    name: &str,
    image: Handle,
    st: &SystemTable<Boot>,
) -> Result<&'static mut [u8], uefi::Error> {
    let mut file_system_raw =
        locate_and_open_protocol::<uefi::proto::media::fs::SimpleFileSystem>(image, st)?;
    let file_system = file_system_raw.deref_mut();

    let mut root = file_system.open_volume()?;
    let mut buf = [0u16; 256];

    let filename =
        uefi::CStr16::from_str_with_buf(name, &mut buf).expect("Failed to convert string to utf16");

    let file_handle = root.open(
        filename,
        uefi::proto::media::file::FileMode::Read,
        uefi::proto::media::file::FileAttribute::empty(),
    )?;

    let mut file = match file_handle.into_type()? {
        uefi::proto::media::file::FileType::Regular(f) => f,
        uefi::proto::media::file::FileType::Dir(_) => panic!(),
    };

    let file_info = file
        .get_boxed_info::<uefi::proto::media::file::FileInfo>()
        .unwrap();
    let file_size = usize::try_from(file_info.file_size()).unwrap();

    let file_ptr = st.boot_services().allocate_pages(
        uefi::table::boot::AllocateType::AnyPages,
        uefi::table::boot::MemoryType::LOADER_DATA,
        ((file_size - 1) / 4096) + 1,
    )? as *mut u8;

    unsafe { core::ptr::write_bytes(file_ptr, 0, file_size) };
    let file_slice = unsafe { core::slice::from_raw_parts_mut(file_ptr, file_size) };
    file.read(file_slice).unwrap();

    Ok(file_slice)
}

fn locate_and_open_protocol<P: uefi::proto::ProtocolPointer>(
    image: Handle,
    st: &SystemTable<Boot>,
) -> Result<uefi::table::boot::ScopedProtocol<P>, uefi::Error> {
    let this = st.boot_services();
    let device_path = open_device_path_protocol(image, st)?;
    let mut device_path = device_path.deref();

    let fs_handle = this.locate_device_path::<P>(&mut device_path)?;

    let opened_handle = unsafe {
        this.open_protocol::<P>(
            uefi::table::boot::OpenProtocolParams {
                handle: fs_handle,
                agent: image,
                controller: None,
            },
            uefi::table::boot::OpenProtocolAttributes::Exclusive,
        )
    }?;

    Ok(opened_handle)
}

fn open_device_path_protocol(
    image: Handle,
    st: &SystemTable<Boot>,
) -> Result<uefi::table::boot::ScopedProtocol<uefi::proto::device_path::DevicePath>, uefi::Error> {
    let this = st.boot_services();
    let device_handle = unsafe {
        this.open_protocol::<uefi::proto::loaded_image::LoadedImage>(
            uefi::table::boot::OpenProtocolParams {
                handle: image,
                agent: image,
                controller: None,
            },
            uefi::table::boot::OpenProtocolAttributes::Exclusive,
        )
    }?
    .deref()
    .device()
    .unwrap();

    let device_path = unsafe {
        this.open_protocol::<uefi::proto::device_path::DevicePath>(
            uefi::table::boot::OpenProtocolParams {
                handle: device_handle,
                agent: image,
                controller: None,
            },
            uefi::table::boot::OpenProtocolAttributes::Exclusive,
        )
    }?;

    Ok(device_path)
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
