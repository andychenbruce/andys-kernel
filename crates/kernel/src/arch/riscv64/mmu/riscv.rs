use super::{PhysicalAddr, VirtualAddr, VirtualMemoryScheme};
#[derive(Debug)]
pub enum RiscvPagingError {
    AlreadyMapped {
        attempted_ppn: usize,
        already_there_ppn: usize,
        vpn: usize,
    },
    ReadInvalidPage,
    ReadReservedProtection(usize),
    ReadNextTableFromLeaf,
    WalkingHitInvalidPage,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ProtectionBits {
    //0b0000
    TablePtr,
    //0b0010
    Read,
    //0b0100
    //W, //reserved
    //0b0110
    ReadWrite,
    //0b1000
    Execute,
    //0b1010
    ReadExecute,
    //0b1100
    //XW //reserved
    //0b1110
    ReadWriteExecute,
}

pub mod sv39_paging {
    const PAGE_OFFSET: usize = 12;
    const PAGE_SIZE_BYTES: usize = 1 << PAGE_OFFSET;
    const LEVELS: usize = 3;
    //const SATP_MODE: usize = 8;
    const PPN_MASK: usize = 0b0000000000111111111111111111111111111111111111111111110000000000;

    //use crate::kprintln;
    use super::{PhysicalAddr, VirtualAddr, VirtualMemoryScheme};
    use core::mem::size_of;
    use static_assertions::const_assert;

    use super::RiscvPagingError;

    const_assert!(size_of::<PageTableEntry>() == size_of::<usize>());
    const_assert!(size_of::<PageTable>() == PAGE_SIZE_BYTES);

    impl super::ProtectionBits {
        fn bits(&self) -> usize {
            match self {
                Self::TablePtr => 0b0000,
                Self::Read => 0b0010,
                Self::ReadWrite => 0b0110,
                Self::Execute => 0b1000,
                Self::ReadExecute => 0b1010,
                Self::ReadWriteExecute => 0b1110,
            }
        }

        fn from_entry(entry: PageTableEntry) -> Result<Self, RiscvPagingError> {
            if !entry.is_valid() {
                return Err(RiscvPagingError::ReadInvalidPage);
            }
            let mask: usize = 0b1110;
            let bits = entry.bits & mask;
            match bits {
                0b0000 => Ok(Self::TablePtr),
                0b0010 => Ok(Self::Read),
                0b0110 => Ok(Self::ReadWrite),
                0b1000 => Ok(Self::Execute),
                0b1010 => Ok(Self::ReadExecute),
                0b1110 => Ok(Self::ReadWriteExecute),
                bad_bits => Err(RiscvPagingError::ReadReservedProtection(bad_bits)),
            }
        }
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct PageTableEntry {
        bits: usize,
    }

    impl PageTableEntry {
        fn get_ppn(&self) -> Result<usize, RiscvPagingError> {
            if !self.is_valid() {
                return Err(RiscvPagingError::ReadInvalidPage);
            }
            Ok((self.bits & PPN_MASK) >> 10)
        }
        fn set_ppn(&mut self, ppn: usize) {
            let ppn_bits = ppn << 10;
            assert!(ppn_bits & PPN_MASK == ppn_bits);
            self.bits = (self.bits & !PPN_MASK) | ppn_bits;
        }
        fn set_is_valid(&mut self, val: bool) {
            if val {
                self.bits |= 1;
            } else {
                self.bits &= !1;
            }
        }
        fn is_accessed(&self) -> bool {
            self.bits & (1 << 6) != 0
        }
        fn is_dirty(&self) -> bool {
            self.bits & (1 << 7) != 0
        }
        fn get_table(&self) -> Result<*mut PageTable, RiscvPagingError> {
            if self.is_leaf()? {
                return Err(RiscvPagingError::ReadNextTableFromLeaf);
            }
            Ok(self.get_page_addr()? as *mut PageTable)
        }
        fn get_page_addr(&self) -> Result<usize, RiscvPagingError> {
            if !self.is_valid() {
                return Err(RiscvPagingError::ReadInvalidPage);
            }
            Ok(self.get_ppn()? * PAGE_SIZE_BYTES)
        }
        fn is_leaf(&self) -> Result<bool, RiscvPagingError> {
            assert!(self.is_valid());
            Ok(self.get_protection()? != super::ProtectionBits::TablePtr)
        }
        fn get_protection(&self) -> Result<super::ProtectionBits, RiscvPagingError> {
            super::ProtectionBits::from_entry(*self)
        }
        fn set_accessed_and_dirty(&mut self) {
            self.bits |= (1 << 6) | (1 << 7);
        }
        fn set_protection(&mut self, protection: super::ProtectionBits) {
            let mask = 0b1110;
            let bits = protection.bits();
            assert!((bits & mask) == bits);
            self.bits = (self.bits & !mask) | bits;
        }
        fn is_valid(&self) -> bool {
            self.bits & 1 == 1
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct PageTable {
        entries: [PageTableEntry; 512],
    }

    impl PageTable {
        fn new_empty() -> Self {
            PageTable {
                entries: [PageTableEntry { bits: 0 }; 512],
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Sv39 {
        root: *mut PageTable,
    }

    fn get_vpn_index(vpn: usize, level: usize) -> usize {
        assert!(level < LEVELS);
        let out = (vpn >> (9 * level)) & ((1 << 9) - 1);
        assert!(out < 512);
        out
    }

    impl super::VirtualMemoryScheme for Sv39 {
        type MapError = super::RiscvPagingError;
        type MapProtection = super::ProtectionBits;
        fn new(
            allocator: &mut crate::heap_alloc::AndyAllocator<PAGE_SIZE_BYTES>,
        ) -> Result<Self, Self::MapError> {
            let root: *mut PageTable = allocator.allocate(1).unwrap() as *mut PageTable;
            unsafe { *root = PageTable::new_empty() }
            Ok(Sv39 { root })
        }

        unsafe fn create_mapping(
            &mut self,
            allocator: &mut crate::heap_alloc::AndyAllocator<PAGE_SIZE_BYTES>,
            virtual_page_num: usize,
            physical_page_num: usize,
            protection: Self::MapProtection,
        ) -> Result<(), Self::MapError> {
            //TODO VOLATILE WRITES
            assert!(virtual_page_num < (1 << 27));
            assert!(protection != super::ProtectionBits::TablePtr);
            let mut curr_table = self.root;
            for level in (0..LEVELS).rev() {
                let vpn = get_vpn_index(virtual_page_num, level);
                let entry = &mut (*curr_table).entries[vpn];
                if entry.is_valid() {
                    if entry.is_leaf()? {
                        assert!(level == 0);
                        return Err(Self::MapError::AlreadyMapped {
                            attempted_ppn: physical_page_num,
                            already_there_ppn: entry.get_ppn()?,
                            vpn: virtual_page_num,
                        });
                    }
                    assert!(level != 0);
                    curr_table = entry.get_table()?;
                } else {
                    if level == 0 {
                        entry.set_protection(protection);
                        entry.set_ppn(physical_page_num);
                        entry.set_accessed_and_dirty();
                        entry.set_is_valid(true);
                        return Ok(());
                    }
                    entry.set_protection(super::ProtectionBits::TablePtr);
                    let new_page: usize = allocator.allocate(1).unwrap();
                    let new_table: *mut PageTable = new_page as *mut PageTable;
                    *new_table = PageTable::new_empty();
                    entry.set_ppn(new_page / PAGE_SIZE_BYTES);
                    entry.set_is_valid(true);

                    curr_table = new_table;
                }
            }

            unreachable!()
        }

        fn find_map(&self, from: VirtualAddr) -> Result<PhysicalAddr, Self::MapError> {
            unsafe {
                let mut curr_table_front: *mut PageTableEntry = &mut (*self.root).entries[0];
                let vpn = from.0 / PAGE_SIZE_BYTES;
                for level in (0..LEVELS).rev() {
                    let vpn_index = get_vpn_index(vpn, level);
                    let entry = curr_table_front.wrapping_add(vpn_index);
                    //kprintln!("level = {}, entry = {:64b}", level, (*entry).bits);
                    //kprintln!("vpn = {:027b}, vpn index = {}", vpn, vpn_index);
                    if (*entry).is_valid() {
                        if (*entry).is_leaf()? {
                            assert!(level == 0);
                            assert!((*entry).is_accessed() && (*entry).is_dirty());
                            return Ok(PhysicalAddr(
                                (*entry).get_page_addr()? | (from.0 & 0b111111111111),
                            ));
                        } else {
                            assert!(!((*entry).is_accessed()) && (!(*entry).is_dirty()));
                        }
                        //curr_table = entry.get_table()?;
                        curr_table_front = (*entry).get_page_addr()? as *mut PageTableEntry;
                    } else {
                        return Err(Self::MapError::WalkingHitInvalidPage);
                    }
                }
            }
            unreachable!("no leaf within that amount of levels, invalid page table")
        }

        unsafe fn activate(&self) -> Result<(), Self::MapError> {
            let satp_val = (8 << 60) | (self.root as usize / PAGE_SIZE_BYTES);
            core::arch::asm!("csrw satp, {}", in(reg) satp_val);

            let pmp_config: usize = 0b00011111;
            core::arch::asm!("csrw pmpcfg0, {}", in(reg) pmp_config);

            let addr_config: usize = 0xffffffff;
            core::arch::asm!("csrw pmpaddr0, {}", in(reg) addr_config);

            Ok(())
        }
    }
    pub fn sv39_setup_identity_mapping(
        allocator: &mut crate::heap_alloc::AndyAllocator<4096>,
        table: &mut Sv39,
    ) -> Result<(), <Sv39 as super::VirtualMemoryScheme>::MapError> {
        let regions: [(usize, usize, super::ProtectionBits); 6] = unsafe {
            [
                (
                    crate::arch::special::TEXT_START,
                    crate::arch::special::TEXT_END,
                    super::ProtectionBits::Execute,
                ),
                (
                    crate::arch::special::RODATA_START,
                    crate::arch::special::RODATA_END,
                    super::ProtectionBits::Read,
                ),
                (
                    crate::arch::special::DATA_START,
                    crate::arch::special::DATA_END,
                    super::ProtectionBits::ReadWrite,
                ),
                (
                    crate::arch::special::BSS_START,
                    crate::arch::special::BSS_END,
                    super::ProtectionBits::ReadWrite,
                ),
                (
                    crate::arch::special::STACK_BOT,
                    crate::arch::special::STACK_TOP,
                    super::ProtectionBits::ReadWrite,
                ),
                (
                    crate::arch::special::HEAP_START,
                    crate::arch::special::HEAP_END,
                    super::ProtectionBits::ReadWrite,
                ),
            ]
        };
        let addrs = unsafe {
            [
                crate::arch::special::SYSCON_ADDR,
                crate::arch::special::UART_ADDR,
            ]
        };

        for region in regions {
            unsafe {
                super::super::identity_map_region(allocator, table, region.0, region.1, region.2)?;
            }
        }

        for addr in addrs {
            let page = addr / 4096;
            unsafe {
                table.create_mapping(allocator, page, page, super::ProtectionBits::ReadWrite)?
            }
        }
        Ok(())
    }
}
