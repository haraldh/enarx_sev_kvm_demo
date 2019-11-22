### Requirements

```bash
$ rustup toolchain add nightly
$ rustup toolchain add nightly-2019-11-17
$ rustup component add rust-src
$ cargo install cargo-xbuild
```

*Note*: [`nightly-2019-11-17` has `clippy`](https://rust-lang.github.io/rustup-components-history/index.html)

### Run

```bash
$ cargo build --package vmrun --release
$ cargo +nightly xrun --package kernel --release --target kernel/x86_64-kernel.json
```

or

```bash
$ cargo +nightly xbuild --package kernel --release --target kernel/x86_64-kernel.json
$ cargo run --package vmrun --bin vmrun -- target/x86_64-kernel/release/kernel
```

### Test

```bash
$ cargo build --package vmrun --release
$ cargo +nightly xtest --package kernel --target kernel/x86_64-kernel.json
```

### Clippy

```bash
$ cargo clean
$ cargo clippy
$ cargo +nightly-2019-11-17 xclippy --package kernel --target kernel/x86_64-kernel.json
```
