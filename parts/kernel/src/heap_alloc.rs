//use crate::kprintln;

pub struct AndyAllocator<const PAGE_SIZE: usize> {
    heap_start: usize,
    heap_end: usize,
}

#[derive(Debug)]
pub enum AllocErr {
    NoBigEnoughBlocks,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
enum PageStatus {
    MetaData,
    Empty,
    Taken(TakenPage),
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct TakenPage {
    is_start: bool,
    is_last: bool,
}

impl<const PAGE_SIZE: usize> AndyAllocator<PAGE_SIZE> {
    pub unsafe fn new(heap_start: usize, heap_end: usize) -> Self {
        let allocator = AndyAllocator {
            heap_start,
            heap_end,
        };

        let num_pages: usize = allocator.total_pages();
        let pages_start: usize = allocator.pages_start();

        for page_num in 0..num_pages {
            let page_status = allocator.get_page_status_addr(page_num);

            if page_num < pages_start {
                *page_status = PageStatus::MetaData;
            } else {
                *page_status = PageStatus::Empty;
            }
        }

        allocator
    }

    pub fn allocate(&mut self, num_pages: usize) -> Result<usize, AllocErr> {
        let total_pages = self.total_pages();
        let mut start_page = self.pages_start();
        while start_page < total_pages {
            let end_page = start_page + num_pages;
            match self.check_if_range_unused(start_page, end_page) {
                None => {
                    unsafe {
                        for page_num in start_page..end_page {
                            *self.get_page_status_addr(page_num) = PageStatus::Taken(TakenPage {
                                is_start: page_num == start_page,
                                is_last: page_num == end_page - 1,
                            });
                        }
                    }
                    //kprintln!("allocated page {}", start_page);
                    return Ok(self.get_page_addr(start_page));
                }
                Some(next_free_page) => start_page = next_free_page,
            }
        }

        Err(AllocErr::NoBigEnoughBlocks)
    }

    fn check_page_num(&self, page_num: usize) -> PageStatus {
        let page_addr = self.get_page_status_addr(page_num);
        unsafe { *page_addr }
    }

    fn find_end_of_block(&self, start_num: usize) -> usize {
        assert!(match self.check_page_num(start_num) {
            PageStatus::Taken(page) => page.is_start,
            _ => false,
        });

        for page_num in start_num..self.total_pages() {
            match self.check_page_num(page_num) {
                PageStatus::Taken(page) => {
                    if page.is_last {
                        return page_num + 1;
                    }
                }
                _ => panic!("block has wierd thing inside"),
            }
        }
        panic!("block wthout end");
    }

    fn check_if_range_unused(&self, start_num: usize, end_num: usize) -> Option<usize> {
        for page_num in start_num..end_num {
            match self.check_page_num(page_num) {
                PageStatus::Empty => {}
                PageStatus::Taken(page) => {
                    if page.is_start {
                        return Some(self.find_end_of_block(page_num));
                    }
                }
                x => panic!("page {} is {:?}", page_num, x),
            }
        }
        None
    }

    fn total_pages(&self) -> usize {
        (self.heap_end - self.heap_start) / PAGE_SIZE
    }
    fn page_data_end(&self) -> usize {
        self.heap_start + (self.total_pages() * core::mem::size_of::<PageStatus>())
    }
    fn pages_start(&self) -> usize {
        (self.page_data_end() - self.heap_start) / PAGE_SIZE + 1
    }
    fn get_page_status_addr(&self, page_num: usize) -> *mut PageStatus {
        let page_addr = self.heap_start + (page_num * core::mem::size_of::<PageStatus>());
        assert!(page_addr >= self.heap_start);
        assert!(page_addr < self.page_data_end());
        page_addr as *mut PageStatus
    }
    fn get_page_addr(&self, page_num: usize) -> usize {
        let page_addr = self.heap_start + (page_num * PAGE_SIZE);
        assert!(page_addr >= self.heap_start);
        assert!(page_addr < self.heap_end);
        page_addr
    }
}
