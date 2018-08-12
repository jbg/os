#![feature(abi_x86_interrupt)]
#![feature(alloc)]
#![feature(allocator_api)]
#![feature(const_fn)]
#![feature(lang_items)]
#![feature(panic_implementation)]
#![feature(alloc_error_handler)]
#![feature(ptr_internals)]
#![feature(unique)]
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

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;
use x86_64::registers::control::{Cr0, Cr0Flags};
use x86_64::registers::model_specific::{Efer, EferFlags};

pub const HEAP_START: u64 = 0o_000_001_000_000_0000;
pub const HEAP_SIZE: u64 = 100 * 1024; // 100 KiB

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

  let mut mem_controller = memory::init(&boot_info);

  print!("Setting up interrupt handlers... ");
  interrupts::init(&mut mem_controller);
  println!("done.");

  print!("Initialising the heap... ");
  unsafe {
    HEAP_ALLOCATOR.lock().init(HEAP_START as usize, (HEAP_START + HEAP_SIZE) as usize);
  }
  println!("done.");

  print!("Testing heap allocation... ");
  use alloc::boxed::Box;
  let heap_test = Box::new(42);
  println!("success!");

  println!("Testing breakpoint exception handling...");
  x86_64::instructions::int3();

  println!("");
  println!("up and running. going to sleep now.");
  loop {}
}

fn enable_nx() {
  let nxe_bit = 1 << 11;
  unsafe {
    Efer::write(Efer::read() | EferFlags::NO_EXECUTE_ENABLE);
  };
}

fn enable_write_protect() {
  unsafe { Cr0::write(Cr0::read() | Cr0Flags::WRITE_PROTECT) };
}

#[lang = "eh_personality"] extern fn eh_personality() {}

#[panic_implementation]
#[no_mangle]
pub fn panic(_info: &PanicInfo) -> ! {
  loop {}
}

#[alloc_error_handler]
#[no_mangle]
pub fn alloc_error(_: core::alloc::Layout) -> ! {
    panic!()
}
