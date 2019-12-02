// Copyright 2019 Red Hat
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

fn main() {
    if std::env::var_os("CC").is_none() {
        std::env::set_var("CC", "clang");
    }

    cc::Build::new()
        .no_default_flags(true)
        .file("src/asm.s")
        .static_flag(false)
        .shared_flag(true)
        .compile("asm");

    println!("cargo:rustc-cdylib-link-arg=nostartfiles");
}
