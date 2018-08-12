use core::ptr::Unique;

use x86_64;
use x86_64::instructions::tlb;

use memory::{PhysicalPage, PAGE_SIZE, Allocator};
use super::entry::EntryFlags;
use super::table::{Table, Level4, P4};
use super::{PhysicalAddress, VirtualAddress, VirtualPage, ENTRY_COUNT};

pub struct Mapper {
  p4: Unique<Table<Level4>>
}

impl Mapper {
  pub unsafe fn new() -> Mapper {
    Mapper { p4: Unique::new_unchecked(P4) }
  }

  fn p4(&self) -> &Table<Level4> {
    unsafe { self.p4.as_ref() }
  }

  pub fn p4_mut(&mut self) -> &mut Table<Level4> {
    unsafe { self.p4.as_mut() }
  }

  pub fn translate_page(&self, page: VirtualPage) -> Option<PhysicalPage> {
    let p3 = self.p4().next_table(page.p4_index());
    p3.and_then(|p3| p3.next_table(page.p3_index()))
      .and_then(|p2| p2.next_table(page.p2_index()))
      .and_then(|p1| p1[page.p1_index()].pointed_physical_page())
      .or_else(|| {
        p3.and_then(|p3| {
          let p3_entry = &p3[page.p3_index()];
          if let Some(start) = p3_entry.pointed_physical_page() { // 1GiB page
            if p3_entry.flags().contains(EntryFlags::HUGE_PAGE) {
              assert!(start.number % (ENTRY_COUNT * ENTRY_COUNT) == 0, "huge page not aligned correctly");
              return Some(PhysicalPage { number: start.number + page.p2_index() * ENTRY_COUNT + page.p1_index() });
            }
          }
          if let Some(p2) = p3.next_table(page.p3_index()) {
            let p2_entry = &p2[page.p2_index()];
            if let Some(start) = p2_entry.pointed_physical_page() { // 2MiB page
              if p2_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                assert!(start.number % ENTRY_COUNT == 0, "huge page not aligned correctly");
                return Some(PhysicalPage { number: start.number + page.p1_index() });
              }
            }
          }
          None
        })
      })
  }

  pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress> {
    let offset = virtual_address % PAGE_SIZE;
    self.translate_page(VirtualPage::containing_address(virtual_address))
        .map(|physical_page| physical_page.number as u64 * PAGE_SIZE + offset)
  }

  pub fn map_to<A>(&mut self, virtual_page: VirtualPage, physical_page: PhysicalPage, flags: EntryFlags, allocator: &mut A) where A: Allocator {
    let mut p3 = self.p4_mut().next_table_create(virtual_page.p4_index(), allocator);
    let mut p2 = p3.next_table_create(virtual_page.p3_index(), allocator);
    let mut p1 = p2.next_table_create(virtual_page.p2_index(), allocator);
    assert!(p1[virtual_page.p1_index()].is_unused());
    p1[virtual_page.p1_index()].set(physical_page, flags | EntryFlags::PRESENT);
  }

  pub fn map<A>(&mut self, virtual_page: VirtualPage, flags: EntryFlags, allocator: &mut A) where A: Allocator {
    let physical_page = allocator.allocate().expect("out of memory");
    self.map_to(virtual_page, physical_page, flags, allocator);
  }

  pub fn identity_map<A>(&mut self, physical_page: PhysicalPage, flags: EntryFlags, allocator: &mut A) where A: Allocator {
    let virtual_page = VirtualPage::containing_address(physical_page.start_address());
    self.map_to(virtual_page, physical_page, flags, allocator);
  }

  pub fn unmap<A>(&mut self, page: VirtualPage, allocator: &mut A) where A: Allocator {
    assert!(self.translate(page.start_address()).is_some());
    let p1 = self.p4_mut()
                 .next_table_mut(page.p4_index())
                 .and_then(|p3| p3.next_table_mut(page.p3_index()))
                 .and_then(|p2| p2.next_table_mut(page.p2_index()))
                 .expect("huge pages are not supported");
    let physical_page = p1[page.p1_index()].pointed_physical_page().unwrap();
    p1[page.p1_index()].set_unused();
    tlb::flush(x86_64::VirtAddr::new(page.start_address()));
    // TODO free up p1/2/3 tables if not used any more
    allocator.deallocate(physical_page);
  }
}
