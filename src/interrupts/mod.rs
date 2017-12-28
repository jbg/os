mod gdt;

use spin::Once;
use x86_64;
use x86_64::instructions::segmentation::set_cs;
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::structures::idt::{Idt, ExceptionStackFrame};
use x86_64::structures::tss::TaskStateSegment;

use memory::MemoryController;
use self::gdt::{Gdt, Descriptor};

static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<Gdt> = Once::new();
const DOUBLE_FAULT_IST_INDEX: usize = 0;

lazy_static! {
  static ref IDT: Idt = {
    let mut idt = Idt::new();
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    unsafe {
      idt.double_fault.set_handler_fn(double_fault_handler)
                      .set_stack_index(DOUBLE_FAULT_IST_INDEX as u16);
    }
    idt
  };
}

pub fn init(mem_controller: &mut MemoryController) {
  // Allocate a clean stack for use when calling the double-fault exception handler.
  let double_fault_stack = mem_controller.alloc_stack(1)
                                         .expect("failed to allocate double fault stack");
  let tss = TSS.call_once(|| {
    let mut tss = TaskStateSegment::new();
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX] = x86_64::VirtualAddress(double_fault_stack.top());
    tss
  });

  let mut code_selector = SegmentSelector(0);
  let mut tss_selector = SegmentSelector(0);
  let gdt = GDT.call_once(|| {
    let mut gdt = Gdt::new();
    code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
    tss_selector = gdt.add_entry(Descriptor::tss_segment(&tss));
    gdt
  });
  gdt.load();

  unsafe {
    set_cs(code_selector);
    load_tss(tss_selector);
  }

  IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut ExceptionStackFrame) {
  println!("EXCEPTION: BREAKPOINT");
  println!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: &mut ExceptionStackFrame, _error_code: u64) {
  println!("EXCEPTION: DOUBLE FAULT");
  println!("{:#?}", stack_frame);
  println!("sleeping now...");
  loop {}
}
