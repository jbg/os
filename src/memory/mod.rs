mod area_allocator;
pub mod heap_allocator;
mod paging;
mod stack_allocator;

use core::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};

use multiboot2::BootInformation;

use super::{HEAP_START, HEAP_SIZE};
use self::paging::{PhysicalAddress, VirtualPage, ActivePageTable};
use self::paging::EntryFlags;
pub use self::paging::remap_kernel;
use self::stack_allocator::StackAllocator;
pub use self::stack_allocator::Stack;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalPage {
  number: usize
}

pub const PAGE_SIZE: u64 = 4096;

impl PhysicalPage {
  fn containing_address(address: u64) -> PhysicalPage {
    PhysicalPage { number: (address / PAGE_SIZE) as usize }
  }

  fn start_address(&self) -> PhysicalAddress {
    self.number as u64 * PAGE_SIZE
  }

  fn clone(&self) -> PhysicalPage {
    PhysicalPage { number: self.number }
  }

  fn range_inclusive(start: PhysicalPage, end: PhysicalPage) -> PhysicalPageIter {
    PhysicalPageIter { start: start, end: end }
  }
}

struct PhysicalPageIter {
  start: PhysicalPage,
  end: PhysicalPage
}

impl Iterator for PhysicalPageIter {
  type Item = PhysicalPage;

  fn next(&mut self) -> Option<PhysicalPage> {
    if self.start <= self.end {
      let page = self.start.clone();
      self.start.number += 1;
      Some(page)
    }
    else {
      None
    }
  }
}

pub trait Allocator {
  fn allocate(&mut self) -> Option<PhysicalPage>;
  fn deallocate(&mut self, page: PhysicalPage);
}

pub use self::area_allocator::AreaAllocator;

static MEMORY_INITIALISED: AtomicBool = ATOMIC_BOOL_INIT;

pub fn init(boot_info: &BootInformation) -> MemoryController {
  let already_initialised = MEMORY_INITIALISED.swap(true, Ordering::Relaxed);
  assert!(!already_initialised, "attempted to call memory::init() a second time");
  let memory_map_tag = boot_info.memory_map_tag().expect("memory map tag not found");
  let elf_sections_tag = boot_info.elf_sections_tag().expect("elf sections tag not found");

  println!("Memory areas:");
  for area in memory_map_tag.memory_areas() {
    println!("    start: {:#x}, length: {:#x}", area.start_address(), area.size());
  }

  println!("Kernel sections:");
  for section in elf_sections_tag.sections() {
    println!("    addr: {:#x}, size: {:#x}, flags: {:#x}", section.start_address(), section.size(), section.flags());
  }

  let kernel_start = elf_sections_tag.sections()
                                     .filter(|s| s.is_allocated())
                                     .map(|s| s.start_address())
                                     .min()
                                     .unwrap();
  let kernel_end = elf_sections_tag.sections()
                                   .filter(|s| s.is_allocated())
                                   .map(|s| s.end_address())
                                   .max()
                                   .unwrap();
  let multiboot_start = boot_info.start_address() as u64;
  let multiboot_end = boot_info.end_address() as u64;
  println!("kernel: {:#x}-{:#x}, multiboot: {:#x}-{:#x}", kernel_start, kernel_end, multiboot_start, multiboot_end);

  print!("Setting up memory allocator... ");
  let mut allocator = AreaAllocator::new(kernel_start as u64, kernel_end as u64, multiboot_start, multiboot_end, memory_map_tag.memory_areas());
  println!("done.");

  println!("Remapping kernel sections...");
  let mut active_table = remap_kernel(&mut allocator, boot_info);

  let heap_start_page = VirtualPage::containing_address(HEAP_START);
  let heap_end_page = VirtualPage::containing_address(HEAP_START + HEAP_SIZE);
  for page in VirtualPage::range_inclusive(heap_start_page, heap_end_page) {
    active_table.map(page, EntryFlags::WRITABLE, &mut allocator);
  }

  let stack_allocator = {
    let start = heap_end_page + 1;
    let end = start + 100;
    let range = VirtualPage::range_inclusive(start, end);
    StackAllocator::new(range)
  };

  MemoryController { active_table, allocator, stack_allocator }
}

pub struct MemoryController {
  active_table: ActivePageTable,
  allocator: AreaAllocator,
  stack_allocator: StackAllocator
}

impl MemoryController {
  pub fn alloc_stack(&mut self, size_in_pages: usize) -> Option<Stack> {
    let &mut MemoryController { ref mut active_table, ref mut allocator, ref mut stack_allocator } = self;
    stack_allocator.alloc_stack(active_table, allocator, size_in_pages)
  }
}
