use crate::eprintln;
use crate::frame_allocator::AndyFrameAllocator;
use crate::UEFI_PHYSICAL_OFFSET;
use x86_64::structures::paging::Mapper;
use x86_64::structures::paging::PageSize;
use x86_64::structures::paging::Translate;
use x86_64::{
    structures::paging::{
        FrameAllocator, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

pub fn map_elf_into_memory(
    kernel_offset: PhysAddr,
    kernel_file: &xmas_elf::ElfFile,
    frame_allocator: &mut AndyFrameAllocator,
) -> (OffsetPageTable<'static>, PhysFrame) {
    assert!(PhysAddr::new(kernel_file.input as *const [u8] as *const u8 as u64) == kernel_offset);

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

    for program_header in kernel_file.program_iter() {
        if !matches!(program_header, xmas_elf::program::ProgramHeader::Ph64(_)) {
            panic!("only supports 64 bit elfs");
        }

        if program_header.align() != 0 {
            assert!(
                (program_header.virtual_addr() % program_header.align())
                    == (program_header.offset() % program_header.align())
            );
        }

        match program_header.get_type().unwrap() {
            xmas_elf::program::Type::Load => {
                handle_load_segment(
                    kernel_offset,
                    kernel_file,
                    &mut kernel_page_table,
                    frame_allocator,
                    program_header,
                );
            }
            xmas_elf::program::Type::GnuRelro => {
                let make_read_only_start = program_header.virtual_addr();
                let make_read_only_end = make_read_only_start + program_header.mem_size();
                eprintln!("todo handle GNU_RELRO");
                eprintln!(
                    "should set memory {:x} to {:x} as read only memory",
                    make_read_only_start, make_read_only_end
                );
            }
            xmas_elf::program::Type::OsSpecific(num) => {
                if num == 1685382481 {
                    eprintln!("todo handle GNU_STACK");
                } else {
                    todo!("wierd os specific type: {:?}", num)
                }
            }
            other => {
                todo!("wierd type: {:?}", other)
            }
        }
    }

    return (kernel_page_table, kernel_page_table_top_frame);
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

    //If the segment's memory size p_memsz is larger than the file size p_filesz, the "extra" bytes are defined to hold the value 0 and to follow the segment's initialized area. The file size may not be larger than the memory size.
    let segment_filesize = segment.file_size();
    let segment_mem_size = segment.mem_size();
    assert!(segment_filesize <= segment_mem_size);
    if segment_filesize < segment_mem_size {
        elf_fill_zeros(
            target_start + segment_filesize,
            target_start + segment_mem_size,
            segment_flags,
            kernel_page_table,
            frame_allocator,
        );
    }
}

fn elf_fill_zeros(
    start: VirtAddr,
    end: VirtAddr,
    segment_flags: PageTableFlags,
    kernel_page_table: &mut OffsetPageTable,
    frame_allocator: &mut AndyFrameAllocator,
) {
    if !start.is_aligned(Size4KiB::SIZE) {
        //duplicate last page and fill remaining bytes with 0
        let last_page: Page<Size4KiB> = Page::containing_address(start - 1u64);
        let (frame, flags) = match kernel_page_table.translate(last_page.start_address()) {
            x86_64::structures::paging::mapper::TranslateResult::Mapped {
                frame,
                offset: _,
                flags,
            } => (frame, flags),
            err => panic!("err: {:?}", err),
        };
        assert!(flags == segment_flags);

        let frame = if let x86_64::structures::paging::mapper::MappedFrame::Size4KiB(frame) = frame
        {
            frame
        } else {
            unreachable!()
        };
        let new_frame = frame_allocator.allocate_frame().unwrap();
        let frame_ptr = frame.start_address().as_u64() as *const u8;
        let new_frame_ptr = new_frame.start_address().as_u64() as *mut u8;
        unsafe {
            core::ptr::copy_nonoverlapping(frame_ptr, new_frame_ptr, Size4KiB::SIZE as usize);
        }
        kernel_page_table.unmap(last_page).unwrap().1.ignore();
        unsafe {
            kernel_page_table
                .map_to(last_page, new_frame, flags, frame_allocator)
                .unwrap()
                .ignore();
        }

        let bytes_before_zeros = start.as_u64() % Size4KiB::SIZE;
        unsafe {
            core::ptr::write_bytes(
                new_frame_ptr.add(bytes_before_zeros as usize),
                0,
                (Size4KiB::SIZE - bytes_before_zeros) as usize,
            );
        }
    }
    let start_page: Page = Page::containing_address(VirtAddr::new(x86_64::align_up(
        start.as_u64(),
        Size4KiB::SIZE,
    )));
    let end_page = Page::containing_address(end - 1u64);

    for page in Page::range_inclusive(start_page, end_page) {
        let frame = frame_allocator.allocate_frame().unwrap();

        let frame_ptr = frame.start_address().as_u64() as *mut [u8; Size4KiB::SIZE as usize];
        unsafe { frame_ptr.write([0; Size4KiB::SIZE as usize]) };

        let flusher = unsafe {
            kernel_page_table
                .map_to(page, frame, segment_flags, frame_allocator)
                .unwrap()
        };
        flusher.ignore();
    }
}

fn uefi_get_addr_of_frame(frame: PhysFrame) -> VirtAddr {
    uefi_get_addr(frame.start_address())
}

fn uefi_get_addr(physical_addr: PhysAddr) -> VirtAddr {
    VirtAddr::new(physical_addr.as_u64() - UEFI_PHYSICAL_OFFSET)
}
