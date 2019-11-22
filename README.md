### Requirements

```bash
$ rustup toolchain add nightly-2019-11-17
$ rustup component add rust-src
$ cargo install cargo-xbuild
```

*Note*: `nightly-2019-11-17` has `clippy`

### Run

```bash
$ cd vmrun
$ cargo install --path .
$ cd ../kernel
$ cargo xrun --release
```

or

```bash
$ cd kernel
$ cargo xbuild --release
$ cd ../vmrun
$ cargo run -- ../kernel/target/x86_64-kernel/release/kernel
```

### Test

```bash
$ cd vmrun
$ cargo install --path .
$ cd ../kernel
$ cargo xtest
```

### Clippy

```bash
$ cd vmrun
$ cargo clean; cargo clippy
$ cd ../kernel
$ cargo clean; cargo xclippy
```
