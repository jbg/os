global late_start

section .text
bits 64
late_start:
  ; Clear all DS registers
  mov ax, 0
  mov ss, ax
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax

  ; Call into Rust code
  extern rust_main
  call rust_main

  ; Print "OKAY" to the screen and halt.
  mov rax, 0x2f592f412f4b2f4f
  mov qword [0xb8000], rax
  hlt
