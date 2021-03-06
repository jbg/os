use multiboot2::{ElfSection, ElfSectionFlags};

use memory::PhysicalPage;

bitflags! {
  pub struct EntryFlags: u64 {
    const PRESENT =         1 << 0;
    const WRITABLE =        1 << 1;
    const USER_ACCESSIBLE = 1 << 2;
    const WRITE_THROUGH =   1 << 3;
    const NO_CACHE =        1 << 4;
    const ACCESSED =        1 << 5;
    const DIRTY =           1 << 6;
    const HUGE_PAGE =       1 << 7;
    const GLOBAL =          1 << 8;
    const NO_EXECUTE =      1 << 63;
  }
}

impl EntryFlags {
  pub fn from_elf_section_flags(section: &ElfSection) -> EntryFlags {
    let mut flags = EntryFlags::empty();
    if section.flags().contains(ElfSectionFlags::ALLOCATED) {
      flags = flags | EntryFlags::PRESENT;
    }
    if section.flags().contains(ElfSectionFlags::WRITABLE) {
      flags = flags | EntryFlags::WRITABLE;
    }
    if !section.flags().contains(ElfSectionFlags::EXECUTABLE) {
      flags = flags | EntryFlags::NO_EXECUTE;
    }
    flags
  }
}

pub struct Entry(u64);

impl Entry {
  pub fn is_unused(&self) -> bool {
    self.0 == 0
  }

  pub fn set_unused(&mut self) {
    self.0 = 0;
  }

  pub fn flags(&self) -> EntryFlags {
    EntryFlags::from_bits_truncate(self.0)
  }

  pub fn pointed_physical_page(&self) -> Option<PhysicalPage> {
    if self.flags().contains(EntryFlags::PRESENT) {
      Some(PhysicalPage::containing_address(self.0 & 0x000fffff_fffff000))
    }
    else {
      None
    }
  }

  pub fn set(&mut self, physical_page: PhysicalPage, flags: EntryFlags) {
    assert!(physical_page.start_address() & !0x000fffff_fffff000 == 0);
    self.0 = (physical_page.start_address() as u64) | flags.bits();
  }
}
