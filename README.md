# a silly operating system (mostly) written in Rust

Make sure you have qemu, nasm and grub (for `grub-mkrescue`) installed and are on an x86_64 system. Then:

```
$ rustup install nightly && rustup default nightly
$ cargo install xargo
$ make run
```

If nothing broke then qemu should launch after a while and boot the OS.
