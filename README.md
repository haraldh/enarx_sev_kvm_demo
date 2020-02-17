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

```console
$ rustup toolchain add nightly
$ rustup toolchain add nightly-2020-02-13 --force
$ rustup target add x86_64-unknown-linux-musl
$ rustup target add x86_64-unknown-linux-musl --toolchain nightly
$ rustup component add rust-src --toolchain nightly
$ rustup component add rust-src --toolchain nightly-2020-02-13
$ rustup component add llvm-tools-preview --toolchain nightly
$ rustup component add llvm-tools-preview --toolchain nightly-2020-02-13
```

*Note*: [`nightly-2020-02-13` has `clippy`](https://rust-lang.github.io/rustup-components-history/index.html)

## Run

### vmrun

```console
$ cargo build
```

#### with stderr

```console
$ (cd kernel; cargo run )
```

or directly:

```console
$ (cd kernel; cargo build )
$ cargo +nightly-2020-02-13 clippy --target x86_64-unknown-linux-gnu --package kernel
```

#### without stderr
```bash
$ (cd kernel; cargo run ) 2>/dev/null
```

### qemu

```console
$ cargo build
$ (cd kernel; cargo build )
```

#### with stderr
```console
$ cargo run --package vmrun -- --force-qemu target/x86_64-unknown-linux-musl/debug/kernel
```

#### without stderr
```console
$ cargo run --package vmrun -- --force-qemu target/x86_64-unknown-linux-musl/debug/kernel 2>/dev/null
```

## Test

```console
$ cargo build
$ cargo test
$ (cd kernel; cargo test)
```

## Clippy for the kernel

```console
$ cargo clean
$ cargo clippy
$ (cd kernel; cargo +nightly-2020-02-13 clippy --target x86_64-unknown-linux-gnu)
```

## gdb debugging with the kernel

```console
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
    -ex 'br usermode' \
    -ex 'cont'
```

to debug the app, continue with:
```console
> next
> next
> file target/x86_64-unknown-linux-musl/debug/app
> br app::main
> cont
```
