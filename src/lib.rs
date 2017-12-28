#![feature(lang_items)]
#![feature(const_fn)]
#![feature(unique)]
#![feature(alloc)]
#![feature(allocator_api)]
#![feature(global_allocator)]
#![feature(abi_x86_interrupt)]
#![no_std]

#[macro_use] extern crate alloc;
extern crate bit_field;
#[macro_use] extern crate bitflags;
#[macro_use] extern crate lazy_static;
extern crate linked_list_allocator;
extern crate multiboot2;
extern crate rlibc;
extern crate spin;
extern crate volatile;
extern crate x86_64;

#[macro_use] mod vga; // this is first so that other modules can use the macros

mod interrupts;
mod memory;

use linked_list_allocator::LockedHeap;
use x86_64::registers::control_regs;
use x86_64::registers::msr::{IA32_EFER, rdmsr, wrmsr};

pub const HEAP_START: usize = 0o_000_001_000_000_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[no_mangle]
pub extern fn rust_main(multiboot_info_addr: usize) {
  vga::clear_screen();
  println!("os v0.1.0");
  println!("");

  print!("Loading Multiboot tags... ");
  let boot_info = unsafe { multiboot2::load(multiboot_info_addr) };
  println!("done.");

  print!("Enabling NX... ");
  enable_nx();
  println!("done.");

  print!("Enabling write-protect for kernel sections... ");
  enable_write_protect();
  println!("done.");

  let mut mem_controller = memory::init(boot_info);

  print!("Setting up interrupt handlers... ");
  interrupts::init(&mut mem_controller);
  println!("done.");

  print!("Initialising the heap... ");
  unsafe {
    HEAP_ALLOCATOR.lock().init(HEAP_START, HEAP_START + HEAP_SIZE);
  }
  println!("done.");

  print!("Testing heap allocation... ");
  use alloc::boxed::Box;
  let heap_test = Box::new(42);
  println!("success!");

  println!("Testing breakpoint exception handling...");
  x86_64::instructions::interrupts::int3();

  println!("");
  println!("up and running. going to sleep now.");
  loop {}
}

fn enable_nx() {
  let nxe_bit = 1 << 11;
  unsafe {
    let efer = rdmsr(IA32_EFER);
    wrmsr(IA32_EFER, efer | nxe_bit);
  };
}

fn enable_write_protect() {
  unsafe { control_regs::cr0_write(control_regs::cr0() | control_regs::Cr0::WRITE_PROTECT) };
}

#[lang = "eh_personality"] extern fn eh_personality() {}
#[lang = "panic_fmt"] #[no_mangle] pub extern fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
  println!("kernel panic in {} at line {}:", file, line);
  println!("    {}", fmt);
  loop {}
}
