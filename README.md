# a silly operating system (mostly) written in Rust

Make sure you have qemu, nasm and grub (for `grub-mkrescue`) installed and are on an x86_64 system. Then:

```
$ rustup install nightly && rustup default nightly
$ cargo install xargo
$ make run
```

If nothing broke then qemu should launch after a while and boot the OS.

## What works

 * boot information is received from a multiboot2-compliant bootloader (e.g. Grub)
 * switching to long mode (64-bit)
 * calling into Rust (assembly is only used for the very early stage of boot)
 * VGA console with colour
 * 4-level page table with recursive mapping
 * remapping the kernel into the page table, with NX and write-protect
 * stack with guard page
 * heap allocator (allowing Rust Box, Vec, BTreeMap, etc to be used)
 * interrupts: breakpoint & double fault handlers, with double fault handler called with a separate stack to prevent triple faults

## Next

 * keyboard input
