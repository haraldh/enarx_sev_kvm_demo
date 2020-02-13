#!/usr/bin/env python

# This file MUST NOT be named `ld`, otherwise Rust will try to outsmart us.

import shutil
import pprint
import sys
import os

spec = {
    # This crate should produce a relocatable, self-contained binary
    # with a custom entry point. I **think** it is currently working.
    "kernel": {
        "remove": [ "-Wl,-Bdynamic" ],
        "remove_endswith": [ "/crtn.o", "/crt1.o", "/crti.o" ],
        "insert": [ "-nostartfiles",
                    "/home/harald/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-musl/lib/liblibc-b1733b99f755c896.rlib",
                    "-Wl,-Tkernel/layout.ld",
                    "-Wl,-as-needed",
                    "-fuse-ld=lld",
                    ],
    }
}

# Find the real compiler
cc = os.getenv('CC')
if cc is None:
    cc = shutil.which('cc')
assert(cc is not None)
argv = [cc] + sys.argv[1:]

crate = os.getenv("CARGO_PKG_NAME")
build = len([a for a in argv if "build_script_build" in a]) > 0

def is_in_endswith(ele, dct):
    for e in dct:
        if ele.endswith(e):
            return True
    return False

# Substitute flags
data = spec.get(crate, {})
if data and not build:
    argv = list(filter(lambda x: x not in data.get("remove", []), argv))
    argv = list(filter(lambda x: not is_in_endswith(x, data.get("remove_endswith", [])), argv))
    for flag in data.get("insert", []):
        argv.append(flag)

#with open("/dev/stderr", "w") as f:
#    pprint.pprint(os.getcwd(), f)
#    pprint.pprint(dict(os.environ), f)
#    pprint.pprint(argv, f)
#    pprint.pprint(build, f)

# Execute the real linker
#if crate == 'code':
#    sys.exit(1)
os.execvp(argv[0], argv)
