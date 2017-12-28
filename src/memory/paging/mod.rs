mod entry;
mod mapper;
mod table;
mod temporary;

use core::ops::{Deref, DerefMut, Add};

use multiboot2::BootInformation;
use x86_64;
use x86_64::instructions::tlb;
use x86_64::registers::control_regs;

use memory::{PAGE_SIZE, Allocator, PhysicalPage};
pub use self::entry::EntryFlags;
use self::mapper::Mapper;
use self::temporary::TemporaryPage;

const ENTRY_COUNT: usize = 512;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualPage {
  number: usize
}

impl VirtualPage {
  pub fn containing_address(address: VirtualAddress) -> VirtualPage {
    assert!(address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000, "invalid address: 0x{:x}", address);
    VirtualPage { number: address / PAGE_SIZE }
  }

  pub fn start_address(&self) -> usize {
    self.number * PAGE_SIZE
  }

  fn p4_index(&self) -> usize {
    (self.number >> 27) & 0o777
  }

  fn p3_index(&self) -> usize {
    (self.number >> 18) & 0o777
  }

  fn p2_index(&self) -> usize {
    (self.number >> 9) & 0o777
  }

  fn p1_index(&self) -> usize {
    (self.number >> 0) & 0o777
  }

  pub fn range_inclusive(start: VirtualPage, end: VirtualPage) -> VirtualPageIter {
    VirtualPageIter { start: start, end: end }
  }
}

impl Add<usize> for VirtualPage {
    type Output = VirtualPage;

    fn add(self, rhs: usize) -> VirtualPage {
        VirtualPage { number: self.number + rhs }
    }
}

#[derive(Clone)]
pub struct VirtualPageIter {
  start: VirtualPage,
  end: VirtualPage
}

impl Iterator for VirtualPageIter {
  type Item = VirtualPage;

  fn next(&mut self) -> Option<VirtualPage> {
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

pub struct ActivePageTable {
  mapper: Mapper
}

impl Deref for ActivePageTable {
  type Target = Mapper;

  fn deref(&self) -> &Mapper {
    &self.mapper
  }
}

impl DerefMut for ActivePageTable {
  fn deref_mut(&mut self) -> &mut Mapper {
    &mut self.mapper
  }
}

impl ActivePageTable {
  unsafe fn new() -> ActivePageTable {
    ActivePageTable { mapper: Mapper::new() }
  }

  pub fn with<F>(&mut self, table: &mut InactivePageTable, temporary_page: &mut TemporaryPage, f: F) where F: FnOnce(&mut Mapper) {
    {
      // unsafe because reading CR3 throws a CPU exception if not in kernel mode. but we're a kernel!
      let backup = PhysicalPage::containing_address(control_regs::cr3().0 as usize);
      let p4_table = temporary_page.map_table_physical_page(backup.clone(), self);
      self.p4_mut()[511].set(table.p4.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
      tlb::flush_all();
      f(self);
      p4_table[511].set(backup, EntryFlags::PRESENT | EntryFlags::WRITABLE);
      tlb::flush_all();
    }
    temporary_page.unmap(self);
  }

  pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
    let old_table = InactivePageTable {
      p4: PhysicalPage::containing_address(control_regs::cr3().0 as usize)
    };
    unsafe {
      control_regs::cr3_write(x86_64::PhysicalAddress(new_table.p4.start_address() as u64));
    }
    old_table
  }
}

pub struct InactivePageTable {
  p4: PhysicalPage
}

impl InactivePageTable {
  pub fn new(page: PhysicalPage, active_table: &mut ActivePageTable, temporary_page: &mut TemporaryPage) -> InactivePageTable {
    {
      let table = temporary_page.map_table_physical_page(page.clone(), active_table);
      table.zero();
      table[511].set(page.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
    }
    temporary_page.unmap(active_table);
    InactivePageTable { p4: page }
  }
}

pub fn remap_kernel<A>(allocator: &mut A, boot_info: &BootInformation) -> ActivePageTable where A: Allocator {
  let mut temporary_page = TemporaryPage::new(VirtualPage { number: 0xcafebabe }, allocator);
  let mut active_table = unsafe { ActivePageTable::new() };
  let mut new_table = {
    let physical_page = allocator.allocate().expect("out of memory");
    InactivePageTable::new(physical_page, &mut active_table, &mut temporary_page)
  };
  active_table.with(&mut new_table, &mut temporary_page, |mapper| {
    // Identity map all kernel sections
    let elf_sections_tag = boot_info.elf_sections_tag().expect("elf sections tag missing from boot info");
    for section in elf_sections_tag.sections() {
      if !section.is_allocated() {
        continue;
      }
      assert!(section.start_address() % PAGE_SIZE == 0, "elf section not page aligned");
      println!("remapping kernel section at addr: {:#x}, size: {:#x}", section.addr, section.size);
      let flags = EntryFlags::from_elf_section_flags(section);
      let start = PhysicalPage::containing_address(section.start_address());
      let end = PhysicalPage::containing_address(section.end_address() - 1);
      for page in PhysicalPage::range_inclusive(start, end) {
        mapper.identity_map(page, flags, allocator);
      }
    }

    // Identity map the VGA buffer
    println!("remapping vga buffer");
    let vga_buffer_page = PhysicalPage::containing_address(0xb8000);
    mapper.identity_map(vga_buffer_page, EntryFlags::WRITABLE, allocator);

    // Identity map the multiboot info structure
    let multiboot_start = PhysicalPage::containing_address(boot_info.start_address());
    let multiboot_end = PhysicalPage::containing_address(boot_info.end_address() - 1);
    println!("remapping multiboot info ({:?} - {:?})", multiboot_start, multiboot_end);
    for page in PhysicalPage::range_inclusive(multiboot_start, multiboot_end) {
      mapper.identity_map(page, EntryFlags::PRESENT, allocator);
    }
  });

  let old_table = active_table.switch(new_table);
  println!("switched to new page table");

  let old_p4_page = VirtualPage::containing_address(old_table.p4.start_address());
  active_table.unmap(old_p4_page, allocator);
  println!("guard page at {:#x}", old_p4_page.start_address());

  active_table
}
