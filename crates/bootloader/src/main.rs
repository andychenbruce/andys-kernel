#![no_main]
#![no_std]

use core::ops::Deref;
use core::ops::DerefMut;
use uefi::prelude::*;
use uefi::proto::media::file::File;

use x86_64::structures::paging::Mapper;
use x86_64::structures::paging::PageSize;
use x86_64::{
    structures::paging::{
        FrameAllocator, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

static mut WRITER: AndyWriter = AndyWriter {};

const UEFI_PHYSICAL_OFFSET: u64 = 0; //UEFI uses identity mapping

#[entry]
fn main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    eprintln!("sup");

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
    let mut frame_allocator = AndyFrameAllocator::new(memory_map.entries().copied());
    let (mut kernel_page_table, kernel_page_table_top_frame) = {
        let table_top_frame: PhysFrame =
            frame_allocator.allocate_frame().expect("no unused frames");
        eprintln!("New page table at: {:#?}", &table_top_frame);
        let frame_addr = uefi_get_addr_of_frame(table_top_frame);

        let ptr = frame_addr.as_mut_ptr();
        unsafe { *ptr = PageTable::new() };
        let top_table = unsafe { &mut *ptr };
        (
            unsafe { OffsetPageTable::new(top_table, VirtAddr::new(UEFI_PHYSICAL_OFFSET)) },
            table_top_frame,
        )
    };

    load_elf_to_memory(
        kernel_addr,
        &kernel_elf,
        &mut kernel_page_table,
        &mut frame_allocator,
    );

    loop {}
    //Status::SUCCESS
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

fn load_elf_to_memory(
    kernel_offset: PhysAddr,
    kernel_file: &xmas_elf::ElfFile,
    kernel_page_table: &mut OffsetPageTable,
    frame_allocator: &mut AndyFrameAllocator,
) {
    assert!(PhysAddr::new(kernel_file.input as *const [u8] as *const u8 as u64) == kernel_offset);
    for program_header in kernel_file.program_iter() {
        if !matches!(program_header, xmas_elf::program::ProgramHeader::Ph64(_)) {
            panic!("only supports 64 bit elfs");
        }

        assert!(
            (program_header.virtual_addr() % program_header.align())
                == (program_header.offset() % program_header.align())
        );

        match program_header.get_type().unwrap() {
            xmas_elf::program::Type::Load => {
                handle_load_segment(
                    kernel_offset,
                    kernel_file,
                    kernel_page_table,
                    frame_allocator,
                    program_header,
                );
            }
            _ => {
                todo!()
            }
        }
    }
}

fn handle_load_segment(
    kernel_addr: PhysAddr,
    kernel_file: &xmas_elf::ElfFile,
    kernel_page_table: &mut OffsetPageTable,
    frame_allocator: &mut AndyFrameAllocator,
    segment: xmas_elf::program::ProgramHeader,
) {
    assert!(matches!(
        segment.get_type().unwrap(),
        xmas_elf::program::Type::Load
    ));

    let data = match segment.get_data(kernel_file).unwrap() {
        xmas_elf::program::SegmentData::Undefined(slice) => slice,
        _ => todo!(),
    };

    {
        let elf_offset = PhysAddr::new(kernel_file.input as *const [u8] as *const u8 as u64);
        let segment_data_start = PhysAddr::new(data as *const [u8] as *const u8 as u64);
        let segment_offset = segment.offset();
        assert!(kernel_addr == elf_offset);
        assert!(kernel_addr + segment_offset == segment_data_start);
    }

    let segment_start = kernel_addr + segment.offset();
    let segment_end = segment_start + segment.file_size();

    let segment_start_frame: PhysFrame = PhysFrame::containing_address(segment_start);
    let segment_end_frame: PhysFrame = PhysFrame::containing_address(segment_end - 1);

    let target_start = VirtAddr::new(segment.virtual_addr());
    let target_start_page: Page = Page::containing_address(target_start);

    eprintln!(
        "loading segment in real memory in memory [{:?}..{:?}] to starting at page {:?}",
        segment_start, segment_end, target_start_page
    );

    let mut segment_flags = PageTableFlags::PRESENT;
    if !segment.flags().is_execute() {
        segment_flags |= PageTableFlags::NO_EXECUTE;
    }
    if segment.flags().is_write() {
        segment_flags |= PageTableFlags::WRITABLE;
    }

    for from_frame in PhysFrame::range_inclusive(segment_start_frame, segment_end_frame) {
        let offset = from_frame - segment_start_frame;
        let target_page = target_start_page + offset;
        eprintln!("mapping frame {:?} to page {:?}", from_frame, target_page);

        match unsafe {
            kernel_page_table.map_to(target_page, from_frame, segment_flags, frame_allocator)
        } {
            Ok(flusher) => {
                flusher.ignore();
            }
            Err(mapping_error) => {
                if let x86_64::structures::paging::mapper::MapToError::PageAlreadyMapped(
                    already_there_frame,
                ) = mapping_error
                {
                    assert!(already_there_frame == from_frame);
                } else {
                    panic!("error mapping thing: {:?}", mapping_error);
                }
            }
        }
    }
}

struct AndyFrameAllocator<'a> {
    next_frame: PhysFrame,
    memory_map: core::iter::Copied<uefi::table::boot::MemoryMapIter<'a>>,
    curr_descriptor: Option<uefi::table::boot::MemoryDescriptor>,
}

impl<'a> AndyFrameAllocator<'a> {
    fn new(memory_map: core::iter::Copied<uefi::table::boot::MemoryMapIter<'a>>) -> Self {
        AndyFrameAllocator {
            //must skip frame 0 for null pointers
            next_frame: PhysFrame::from_start_address(PhysAddr::new(0x1000)).unwrap(),
            memory_map,
            curr_descriptor: None,
        }
    }
    fn allocate_frame_from_descriptor(
        &mut self,
        descriptor: uefi::table::boot::MemoryDescriptor,
    ) -> Option<PhysFrame<Size4KiB>> {
        let mem_len = descriptor.page_count * (uefi::table::boot::PAGE_SIZE as u64);
        let mem_start = PhysAddr::new(descriptor.phys_start);
        let mem_end = mem_start + mem_len;

        let start_frame = PhysFrame::from_start_address(mem_start).unwrap();
        let end_frame = PhysFrame::containing_address(mem_end - 1);

        assert!(self.next_frame <= end_frame);
        if self.next_frame <= start_frame {
            self.next_frame = start_frame;
        }

        if self.next_frame <= end_frame {
            let out = self.next_frame;
            self.next_frame += 1;
            return Some(out);
        } else {
            return None;
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for AndyFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if let Some(descriptor) = self.curr_descriptor {
            if let Some(success) = self.allocate_frame_from_descriptor(descriptor) {
                return Some(success);
            } else {
                self.curr_descriptor = None;
            }
        }

        while let Some(descriptor) = self.memory_map.next() {
            let is_usable = descriptor.ty == uefi::table::boot::MemoryType::CONVENTIONAL;
            if !is_usable {
                continue;
            }
            if let Some(frame) = self.allocate_frame_from_descriptor(descriptor) {
                self.curr_descriptor = Some(descriptor);
                return Some(frame);
            }
        }
        None
    }
}

fn uefi_get_addr_of_frame(frame: PhysFrame) -> VirtAddr {
    uefi_get_addr(frame.start_address())
}

fn uefi_get_addr(physical_addr: PhysAddr) -> VirtAddr {
    VirtAddr::new(physical_addr.as_u64() - UEFI_PHYSICAL_OFFSET)
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
