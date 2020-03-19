// Copyright 2020 Red Hat
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate cc;
use std::ffi::OsString;
use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let manifest_dir = manifest_dir.to_string_lossy();

    /*
    for (k, v) in env::vars_os() {
        eprintln!("{:#?}={:#?}", k, v);
    }
    */

    let mut asm_dir = PathBuf::from(manifest_dir.as_ref());
    asm_dir.push("src/arch/x86_64/asm");
    let entries = fs::read_dir(asm_dir)
        .unwrap()
        .filter_map(|f| {
            f.ok().and_then(|e| {
                let path = e.path();
                match path.extension() {
                    Some(ext) if ext.eq(&OsString::from("s")) => Some(path),
                    Some(ext) if ext.eq(&OsString::from("S")) => Some(path),
                    _ => None,
                }
            })
        })
        .collect::<Vec<_>>();

    cc::Build::new()
        .no_default_flags(true)
        .files(&entries)
        .pic(true)
        .static_flag(true)
        .shared_flag(false)
        .compile("asm");

    for e in entries {
        println!("cargo:rerun-if-changed={}", e.to_str().unwrap());
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=layout.ld");
}
