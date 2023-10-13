pub mod riscv;

pub use riscv::sv39_paging;

#[derive(Debug, Copy, Clone)]
pub struct VirtualAddr(pub usize);
#[derive(Debug, Copy, Clone)]
pub struct PhysicalAddr(pub usize);

pub trait VirtualMemoryScheme {
    type MapError: core::fmt::Debug;
    type MapProtection: Copy;

    fn new(allocator: &mut crate::heap_alloc::AndyAllocator<4096>) -> Result<Self, Self::MapError>
    where
        Self: Sized;

    unsafe fn create_mapping(
        &mut self,
        allocator: &mut crate::heap_alloc::AndyAllocator<4096>,
        virtual_page_num: usize,
        physical_page_num: usize,
        protiection: Self::MapProtection,
    ) -> Result<(), Self::MapError>;

    fn find_map(&self, from: VirtualAddr) -> Result<PhysicalAddr, Self::MapError>;

    unsafe fn activate(&self) -> Result<(), Self::MapError>;
}

pub unsafe fn identity_map_region<T: VirtualMemoryScheme>(
    allocator: &mut crate::heap_alloc::AndyAllocator<4096>,
    table: &mut T,
    start_addr: usize,
    end_addr: usize,
    protection: T::MapProtection,
) -> Result<(), T::MapError> {
    let start_page = start_addr / 4096;
    let end_page = (end_addr + 4095) / 4096;
    for page in start_page..end_page {
        table.create_mapping(allocator, page, page, protection)?;
    }

    Ok(())
}
pub fn assert_identity_map<T: VirtualMemoryScheme>(table: &T) {
    let regions: [(usize, usize); 6] = unsafe {
        [
            (crate::TEXT_START, crate::TEXT_END),
            (crate::RODATA_START, crate::RODATA_END),
            (crate::DATA_START, crate::DATA_END),
            (crate::BSS_START, crate::BSS_END),
            (crate::STACK_BOT, crate::STACK_TOP),
            (crate::HEAP_START, crate::HEAP_END),
        ]
    };
    for region in regions {
        let start_page = region.0 / 4096;
        let end_page = region.1 / 4096;
        for page in start_page..end_page {
            let virt_addr = VirtualAddr(page * 4096);
            let phys_addr = table.find_map(virt_addr).unwrap();
            assert!(phys_addr.0 == virt_addr.0);
        }
    }
}
