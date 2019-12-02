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
use std::{
    env,
    path::PathBuf,
    process::{self, Command},
};

fn main() {
    if env::var_os("CC").is_none() {
        env::set_var("CC", "clang");
    }

    cc::Build::new()
        .no_default_flags(true)
        .file("src/asm.s")
        .static_flag(false)
        .shared_flag(true)
        .compile("asm");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    // get access to llvm tools shipped in the llvm-tools-preview rustup component
    let llvm_tools = match llvm_tools::LlvmTools::new() {
        Ok(tools) => tools,
        Err(llvm_tools::Error::NotFound) => {
            eprintln!("Error: llvm-tools not found");
            eprintln!("Maybe the rustup component `llvm-tools-preview` is missing?");
            eprintln!("  Install it through: `rustup component add llvm-tools-preview`");
            process::exit(1);
        }
        Err(err) => {
            eprintln!("Failed to retrieve llvm-tools component: {:?}", err);
            process::exit(1);
        }
    };
    // check that kernel executable has code in it
    let llvm_size = llvm_tools
        .tool(&llvm_tools::exe("llvm-size"))
        .expect("llvm-size not found in llvm-tools");
    let objcopy = llvm_tools
        .tool(&llvm_tools::exe("llvm-objcopy"))
        .expect("llvm-objcopy not found in llvm-tools");
    let ar = llvm_tools
        .tool(&llvm_tools::exe("llvm-ar"))
        .unwrap_or_else(|| {
            eprintln!("Failed to retrieve llvm-ar component");
            eprint!("This component is available since nightly-2019-03-29,");
            eprintln!("so try updating your toolchain if you're using an older nightly");
            process::exit(1);
        });

    let env_name = "APP";
    let section_name = "app";

    let elf_path = PathBuf::from(match env::var(env_name) {
        Ok(elf_path) => elf_path,
        Err(_) => {
            eprintln!(
                "The {} environment variable must be set for building the kernel.\n",
                env_name
            );
            process::exit(1);
        }
    });
    let elf_file_name = elf_path
        .file_name()
        .expect(format!("{} has no valid file name", env_name).as_str())
        .to_str()
        .expect(format!("{} file name not valid utf8", section_name).as_str());

    // check that the file exists
    assert!(
        elf_path.exists(),
        format!("{} does not exist: {}", env_name, elf_path.display())
    );

    let mut cmd = Command::new(&llvm_size);
    cmd.arg(&elf_path);
    let output = cmd.output().expect("failed to run llvm-size");
    let output_str = String::from_utf8_lossy(&output.stdout);
    let second_line_opt = output_str.lines().skip(1).next();
    let second_line = second_line_opt.expect("unexpected llvm-size line output");
    let text_size_opt = second_line.split_ascii_whitespace().next();
    let text_size = text_size_opt.expect("unexpected llvm-size output");
    if text_size == "0" {
        panic!("{env} executable has an empty text section. Perhaps the entry point was set incorrectly?\n\n\
            {env} executable at `{path}`\n", env=env_name, path= elf_path.display());
    }

    // strip debug symbols from elf for faster loading
    let stripped_elf_file_name = format!("{}_stripped-{}", section_name, elf_file_name);
    let stripped_elf = out_dir.join(&stripped_elf_file_name);
    let mut cmd = Command::new(&objcopy);
    cmd.arg("--strip-debug");
    cmd.arg(&elf_path);
    cmd.arg(&stripped_elf);
    let exit_status = cmd
        .status()
        .expect("failed to run objcopy to strip debug symbols");
    if !exit_status.success() {
        eprintln!("Error: Stripping debug symbols failed");
        process::exit(1);
    }

    // wrap the elf executable as binary in a new ELF file
    let stripped_elf_file_name_replaced = stripped_elf_file_name.replace('-', "_");
    let elf_bin = out_dir.join(format!("{}_bin-{}.o", section_name, elf_file_name));
    let elf_archive = out_dir.join(format!("lib{}_bin-{}.a", section_name, elf_file_name));
    let mut cmd = Command::new(&objcopy);
    cmd.arg("-I").arg("binary");
    cmd.arg("-O").arg("elf64-x86-64");
    cmd.arg("--binary-architecture=i386:x86-64");
    cmd.arg("--rename-section")
        .arg(format!(".data=.{}", section_name));
    cmd.arg("--redefine-sym").arg(format!(
        "_binary_{}_start=_{}_start_addr",
        stripped_elf_file_name_replaced, section_name
    ));
    cmd.arg("--redefine-sym").arg(format!(
        "_binary_{}_end=_{}_end_addr",
        stripped_elf_file_name_replaced, section_name
    ));
    cmd.arg("--redefine-sym").arg(format!(
        "_binary_{}_size=_{}_size",
        stripped_elf_file_name_replaced, section_name
    ));
    cmd.current_dir(&out_dir);
    cmd.arg(&stripped_elf_file_name);
    cmd.arg(&elf_bin);
    let exit_status = cmd.status().expect("failed to run objcopy");
    if !exit_status.success() {
        eprintln!("Error: Running objcopy failed");
        process::exit(1);
    }

    // create an archive for linking
    let mut cmd = Command::new(&ar);
    cmd.arg("crs");
    cmd.arg(&elf_archive);
    cmd.arg(&elf_bin);
    let exit_status = cmd.status().expect("failed to run ar");
    if !exit_status.success() {
        eprintln!("Error: Running ar failed");
        process::exit(1);
    }

    // pass link arguments to rustc
    println!(
        "cargo:rustc-link-lib=static={}_bin-{}",
        section_name, elf_file_name
    );
    println!("cargo:rustc-link-search=native={}", out_dir.display());

    println!("cargo:rerun-if-changed=src/asm.s");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=layout.ld");
}
