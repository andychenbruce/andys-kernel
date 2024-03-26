use x86_64::{
    structures::paging::{
        FrameAllocator, PhysFrame, Size4KiB,
    },
    PhysAddr
};


pub struct AndyFrameAllocator<'a> {
    next_frame: PhysFrame,
    memory_map: core::iter::Copied<uefi::table::boot::MemoryMapIter<'a>>,
    curr_descriptor: Option<uefi::table::boot::MemoryDescriptor>,
}

impl<'a> AndyFrameAllocator<'a> {
    pub fn new(memory_map: core::iter::Copied<uefi::table::boot::MemoryMapIter<'a>>) -> Self {
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
        let mem_start = PhysAddr::new(descriptor.phys_start);
        let mem_len = descriptor.page_count * (uefi::table::boot::PAGE_SIZE as u64);
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

