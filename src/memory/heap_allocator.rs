use alloc::alloc::{Alloc, AllocErr, Layout};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct BumpAllocator {
  heap_start: usize,
  heap_end: usize,
  next: AtomicUsize
}

impl BumpAllocator {
  pub const fn new(heap_start: usize, heap_end: usize) -> Self {
    Self { heap_start, heap_end, next: AtomicUsize::new(heap_start) }
  }
}

unsafe impl<'a> Alloc for &'a BumpAllocator {
  unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
    loop {
      let current_next = self.next.load(Ordering::Relaxed);
      let start = align_up(current_next, layout.align());
      let end = start.saturating_add(layout.size());
      if end <= self.heap_end {
        let next_now = self.next.compare_and_swap(current_next, end, Ordering::Relaxed);
        if next_now == current_next {
          return Ok(NonNull::new(start as *mut u8).unwrap());
        }
      }
      else {
        return Err(AllocErr);
      }
    }
  }

  unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
    println!("warning: deallocation of memory is unimplemented");
  }
}

pub fn align_down(addr: usize, align: usize) -> usize {
  if align.is_power_of_two() {
    addr & !(align - 1)
  }
  else if align == 0 {
    addr
  }
  else {
    panic!("align must be a power of 2");
  }
}

pub fn align_up(addr: usize, align: usize) -> usize {
  align_down(addr + align - 1, align)
}
