use x86_64::VirtAddr;
use crate::frame_allocator::AndyFrameAllocator;



pub fn make_stack(frame_allocator: &mut AndyFrameAllocator, num_pages: u64) -> VirtAddr{
    //todo
    return VirtAddr::new(1234);
}


