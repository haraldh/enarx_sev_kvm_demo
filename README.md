### Requirements
* rust nightly
* `$ cargo install cargo-xbuild`

```bash
$ cd vmrun
$ cargo install --path .
$ cd ../kernel
$ cargo xrun --release
```