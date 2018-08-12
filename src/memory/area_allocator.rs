use memory::{PhysicalPage, Allocator};
use multiboot2::{MemoryAreaIter, MemoryArea};

pub struct AreaAllocator {
  next_free: PhysicalPage,
  current_area: Option<&'static MemoryArea>,
  areas: MemoryAreaIter,
  kernel_start: PhysicalPage,
  kernel_end: PhysicalPage,
  multiboot_start: PhysicalPage,
  multiboot_end: PhysicalPage
}

impl Allocator for AreaAllocator {
  fn allocate(&mut self) -> Option<PhysicalPage> {
    if let Some(area) = self.current_area {
      let page = PhysicalPage { number: self.next_free.number };
      let current_area_last_page = {
        let address = area.start_address() + area.size() - 1;
        PhysicalPage::containing_address(address)
      };
      if page > current_area_last_page {
        self.choose_next_area();
      }
      else if page >= self.kernel_start && page <= self.kernel_end {
        self.next_free = PhysicalPage { number: self.kernel_end.number + 1 };
      }
      else if page >= self.multiboot_start && page <= self.multiboot_end {
        self.next_free = PhysicalPage { number: self.multiboot_end.number + 1 };
      }
      else {
        self.next_free.number += 1;
        return Some(page);
      }
      self.allocate()
    }
    else {
      None // Nothing left
    }
  }

  fn deallocate(&mut self, page: PhysicalPage) {
    println!("warning: deallocation of physical pages is unimplemented");
  }
}

impl AreaAllocator {
  pub fn new(kernel_start: u64, kernel_end: u64, multiboot_start: u64, multiboot_end: u64, memory_areas: MemoryAreaIter) -> AreaAllocator {
    let mut allocator = AreaAllocator {
      next_free: PhysicalPage::containing_address(0),
      current_area: None,
      areas: memory_areas,
      kernel_start: PhysicalPage::containing_address(kernel_start),
      kernel_end: PhysicalPage::containing_address(kernel_end),
      multiboot_start: PhysicalPage::containing_address(multiboot_start),
      multiboot_end: PhysicalPage::containing_address(multiboot_end)
    };
    allocator.choose_next_area();
    allocator
  }

  fn choose_next_area(&mut self) {
    self.current_area = self.areas.clone().filter(|area| {
      let address = area.start_address() + area.size() - 1;
      PhysicalPage::containing_address(address) >= self.next_free
    }).min_by_key(|area| area.start_address());
    if let Some(area) = self.current_area {
      let first_page = PhysicalPage::containing_address(area.start_address());
      if self.next_free < first_page {
        self.next_free = first_page;
      }
    }
  }
}
