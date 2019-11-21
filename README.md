### Requirements
* rust nightly
* `$ cargo install cargo-xbuild`

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
