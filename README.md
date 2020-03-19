[![Build Stats](https://github.com/haraldh/enarx_sev_kvm_demo/workflows/Rust/badge.svg)](https://github.com/haraldh/enarx_sev_kvm_demo/actions)

## Current State
* Sets up kvm in x86 64bit mode with pagetables
* Boots to a modified [blog_os kernel](https://os.phil-opp.com/)
* Exception handling
* Print to stdout and stderr
* Exit codes
* Simple static ELF app execution in Ring3 with syscalls
  * C with glibc
  * C with musl
  * rust with `--target x86_64-unknown-linux-musl`
* Start elf binary in Ring 3
* Handle syscalls

* qemu running and debugging broken, because of no more serial line support
  and no dynamic app loading via qemu

## TODO
### vmrun
* Lots of refactoring!
* Use other crates:
    * https://github.com/firecracker-microvm/firecracker
    * https://github.com/rust-vmm
    * https://github.com/cloud-hypervisor/rust-hypervisor-firmware
    * https://github.com/rust-osdev/x86_64
### kernel    
* Handle more syscalls
* Memory management via mmap() proxying to vmrun
* Thread creation via clone() in vmrun
* Maybe use [mimalloc](https://github.com/microsoft/mimalloc) as [allocator](https://github.com/purpleprotocol/mimalloc_rust) 

## Requirements

```console
$ rustup target add x86_64-unknown-linux-musl
$ rustup component add llvm-tools-preview
```

## Build

```console
$ cargo build --all
```


## Run

```console
$ (cd kernel; cargo run)
```

or:

```console
$ cargo run --package vmrun -- target/x86_64-unknown-linux-musl/debug/kernel
```


## Run with qemu - **NOTE**: CURRENTLY BROKEN

```console
$ cargo build --all
```

Currently, we need nightly for timers and interrupts.

```console
$ (cd kernel; cargo +nightly build --features qemu)
$ cargo run --package vmrun -- --force-qemu target/x86_64-unknown-linux-musl/debug/kernel
```

## Test

```console
$ cargo test -p vmrun
$ (cd kernel; cargo +nightly test --features qemu)
```

## gdb debugging with the kernel  - **NOTE**: CURRENTLY BROKEN

Currently, we need nightly for timers and interrupts.

```console
$ (cd kernel; cargo +nightly build --features qemu)
$ cargo run --package vmrun -- --force-qemu \
    target/x86_64-unknown-linux-musl/debug/kernel \
    -S -s
```

in another terminal:

```console
$ gdb \
    -ex "add-auto-load-safe-path $(pwd)" \
    -ex "file target/x86_64-unknown-linux-musl/debug/kernel" \
    -ex 'set arch i386:x86-64:intel' \
    -ex 'target remote localhost:1234' \
    -ex 'br _before_jump' -ex 'cont' \
    -ex 'br _usermode' \
    -ex 'cont'
```

to debug the app, continue with:
```console
> next
> next
> file target/x86_64-unknown-linux-musl/debug/app
> br _start
> br app::main
> cont
```
