use memory::{Allocator, PAGE_SIZE};
use memory::paging::{ActivePageTable, EntryFlags, VirtualPageIter, VirtualPage};

pub struct StackAllocator {
  range: VirtualPageIter
}

impl StackAllocator {
  pub fn new(page_range: VirtualPageIter) -> StackAllocator {
    StackAllocator { range: page_range }
  }

  pub fn alloc_stack<A: Allocator>(&mut self, active_table: &mut ActivePageTable, allocator: &mut A, size_in_pages: usize) -> Option<Stack> {
    if size_in_pages == 0 { return None; }

    let mut range = self.range.clone();
    let guard_page = range.next();
    let stack_start = range.next();
    let stack_end = if size_in_pages == 1 {
      stack_start
    }
    else {
      range.nth(size_in_pages - 2)
    };

    match (guard_page, stack_start, stack_end) {
      (Some(_), Some(start), Some(end)) => {
        self.range = range;

        for page in VirtualPage::range_inclusive(start, end) {
          active_table.map(page, EntryFlags::WRITABLE, allocator);
        }

        let top_of_stack = end.start_address() + PAGE_SIZE;
        Some(Stack::new(top_of_stack, start.start_address()))
      },
      _ => None  // not enough pages
    }
  }
}

#[derive(Debug)]
pub struct Stack {
  top: u64,
  bottom: u64
}

impl Stack {
  fn new(top: u64, bottom: u64) -> Stack {
    assert!(top > bottom);
    Stack { top, bottom }
  }

  pub fn top(&self) -> u64 { self.top }
  pub fn bottom(&self) -> u64 { self.bottom }
}
