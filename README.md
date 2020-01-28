[![Build Stats](https://github.com/haraldh/enarx_sev_kvm_demo/workflows/Rust/badge.svg)](https://github.com/haraldh/enarx_sev_kvm_demo/actions)

## Current State
* Sets up kvm in x86 64bit mode with pagetables
* Boots to a modified [blog_os kernel](https://os.phil-opp.com/)
* Exception handling
* Serial print to stdout and stderr
* Exit codes
* Simple static ELF app execution in Ring3 with syscalls
  * glibc
  * musl
  * rust with `--target x86_64-unknown-linux-musl`
* Start elf binary in Ring 3
* Handle syscalls

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

```bash
$ rustup toolchain add nightly
$ rustup toolchain add nightly-2019-11-17
$ rustup target add x86_64-unknown-linux-musl
$ rustup component add rust-src --toolchain nightly
$ rustup component add rust-src --toolchain nightly-2019-11-17
$ rustup component add llvm-tools-preview --toolchain nightly
$ rustup component add llvm-tools-preview --toolchain nightly-2019-11-17
$ cargo install cargo-xbuild
```

*Note*: [`nightly-2019-11-17` has `clippy`](https://rust-lang.github.io/rustup-components-history/index.html)

## Run

### vmrun

```bash
$ cargo build
$ (cd app; cargo build)
```

#### with stderr

```bash
$ (cd kernel; APP=$(pwd)/../target/x86_64-unknown-linux-musl/debug/app cargo +nightly xrun )
```

or directly:

```bash
$ (cd kernel; APP=$(pwd)/../target/x86_64-unknown-linux-musl/debug/app cargo +nightly xbuild )
$ ./target/debug/vmrun  target/x86_64-kernel/release/kernel
```

#### without stderr
```bash
$ (cd kernel; APP=$(pwd)/../target/x86_64-unknown-linux-musl/debug/app cargo +nightly xrun ) 2>/dev/null
```

### qemu

```bash
$ cargo build
$ (cd app; cargo build)
$ (cd kernel; APP=$(pwd)/../target/x86_64-unknown-linux-musl/debug/app cargo +nightly xbuild )
```

#### with stderr
```bash
$ qemu-system-x86_64 -enable-kvm \
    -cpu host -smp 1 -m 128 -vga none -display none -no-reboot \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -chardev stdio,mux=on,id=char0 -mon chardev=char0,mode=readline -serial chardev:char0 -serial chardev:char0 \
    -kernel $(pwd)/target/x86_64-kernel/debug/kernel
```

#### without stderr
```bash
$ qemu-system-x86_64 -enable-kvm \
    -cpu host -smp 1 -m 128 -vga none -display none -no-reboot \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -serial mon:stdio \
    -kernel $(pwd)/target/x86_64-kernel/debug/kernel
```

## Test

```bash
$ cargo build
$ (cd app; cargo build)
$ (cd kernel; APP=$(pwd)/../target/x86_64-unknown-linux-musl/debug/app cargo +nightly xtest )
```

## Clippy for the kernel

```bash
$ cargo clean
$ cargo clippy
$ (cd kernel; APP=$(pwd)/../target/x86_64-unknown-linux-musl/debug/app cargo +nightly-2019-11-17 xclippy )
```

## gdb debugging with the kernel

```bash
$ qemu-system-x86_64 -enable-kvm \
    -cpu host -smp 1 -m 128 -vga none -display none -no-reboot \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -chardev stdio,mux=on,id=char0 -mon chardev=char0,mode=readline -serial chardev:char0 -serial chardev:char0 \
    -d guest_errors,unimp \
    -kernel $(pwd)/target/x86_64-kernel/debug/kernel \
    -S -s
```

in another terminal:

```bash
$ gdb \
    -ex "add-auto-load-safe-path $(pwd)" \
    -ex "file target/x86_64-kernel/debug/kernel" \
    -ex 'set arch i386:x86-64:intel' \
    -ex 'target remote localhost:1234' \
    -ex 'br usermode' \
    -ex 'cont'
```

to debug the app, continue with:
```
> next
> next
> file target/x86_64-unknown-linux-musl/debug/app
> br app::main
> cont
```
