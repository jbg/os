use super::{ActivePageTable, VirtualAddress, VirtualPage};
use super::entry::EntryFlags;
use super::table::{Table, Level1};
use memory::{Allocator, PhysicalPage};

pub struct TemporaryPage {
  page: VirtualPage,
  allocator: TinyAllocator
}

impl TemporaryPage {
  pub fn new<A>(page: VirtualPage, allocator: &mut A) -> TemporaryPage where A: Allocator {
    TemporaryPage {
      page: page,
      allocator: TinyAllocator::new(allocator)
    }
  }

  pub fn map(&mut self, physical_page: PhysicalPage, active_table: &mut ActivePageTable) -> VirtualAddress {
    assert!(active_table.translate_page(self.page).is_none(), "temporary page is already mapped");
    active_table.map_to(self.page, physical_page, EntryFlags::WRITABLE, &mut self.allocator);
    self.page.start_address()
  }

  pub fn unmap(&mut self, active_table: &mut ActivePageTable) {
    active_table.unmap(self.page, &mut self.allocator);
  }

  pub fn map_table_physical_page(&mut self, physical_page: PhysicalPage, active_table: &mut ActivePageTable) -> &mut Table<Level1> {
    unsafe { &mut *(self.map(physical_page, active_table) as *mut Table<Level1>) }
  }
}

struct TinyAllocator([Option<PhysicalPage>; 3]);

impl Allocator for TinyAllocator {
  fn allocate(&mut self) -> Option<PhysicalPage> {
    for maybe in &mut self.0 {
      if maybe.is_some() {
        return maybe.take();
      }
    }
    None
  }

  fn deallocate(&mut self, page: PhysicalPage) {
    for maybe in &mut self.0 {
      if maybe.is_none() {
        *maybe = Some(page);
        return;
      }
    }
    panic!("tiny allocator can only hold 3 pages");
  }
}

impl TinyAllocator {
  fn new<A>(allocator: &mut A) -> TinyAllocator where A: Allocator {
    let mut f = || allocator.allocate();
    let pages = [f(), f(), f()];
    TinyAllocator(pages)
  }
}
