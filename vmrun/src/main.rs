use kvm_ioctls::{Kvm, VcpuExit};
use std::path::Path;
use std::process::{exit, Command};
use std::time::Instant;
use vmbootspec::layout::SYSCALL_TRIGGER_PORT;
use vmrun::kvmvm;

const PORT_QEMU_EXIT: u16 = 0xF4;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let kvm = Kvm::new();

    match args.len() {
        4..=std::usize::MAX if args[1].eq("--force-qemu") => {
            main_qemu(&args[2], &args[3], &args[4..])
        }
        4..=std::usize::MAX if args[1].eq("--fallback-qemu") => match kvm {
            Ok(_) => main_kvm(&args[2], &args[3]),
            Err(_) => main_qemu(&args[2], &args[3], &args[4..]),
        },
        3 => main_kvm(&args[1], &args[2]),
        _ => {
            eprintln!(
                "Usage: {} [--fallback-qemu] <elf binary> <kernelblob>",
                args[0],
            );
            exit(1);
        }
    }
}

fn main_qemu(_elf_binary: &str, kernel_blob: &str, extra_args: &[String]) -> ! {
    if !Path::new(kernel_blob).exists() {
        eprintln!("Kernel image `{}` not found!", kernel_blob);
        exit(1);
    }

    let has_kvm = Kvm::new().is_ok();

    let start = Instant::now();

    eprintln!("Starting QEMU {}", kernel_blob);
    let mut cmd = Command::new("qemu-system-x86_64");
    let mut args = vec![
        "-smp",
        "1",
        "-m",
        "128",
        "-nodefaults",
        "-vga",
        "none",
        "-display",
        "none",
        "-no-reboot",
        "-device",
        "isa-debug-exit,iobase=0xf4,iosize=0x04",
        "-chardev",
        "stdio,mux=on,id=char0",
        "-mon",
        "chardev=char0,mode=readline",
        "-serial",
        "chardev:char0",
        "-serial",
        "chardev:char0",
    ];
    if has_kvm {
        args.push("-enable-kvm");
        args.push("-cpu");
        args.push("host");
    } else {
        args.push("-cpu");
        args.push("max");
    }
    args.push("-kernel");
    args.push(kernel_blob);
    if !extra_args.is_empty() && extra_args[0].eq("--") {
        //eprintln!("Extra args! {:#?}", extra_args);
        args.extend(extra_args.iter().skip(1).map(String::as_str));
    }
    cmd.args(args);
    let mut child = cmd.spawn().expect("Unable to start qemu-system-x86_64");
    let status = child.wait().expect("Failed to wait on qemu-system-x86_64");
    let elapsed = start.elapsed();
    eprintln!("QEMU: Creating and running took {:?}", elapsed);
    match status.code() {
        Some(33) => exit(0),
        Some(v) => exit(v),
        None => {
            eprintln!("qemu terminated by signal");
            exit(1);
        }
    }
}

fn main_kvm(elf_blob: &str, kernel_blob: &str) {
    let start = Instant::now();

    if !Path::new(kernel_blob).exists() {
        eprintln!("Kernel image `{}` not found!", kernel_blob);
        exit(1);
    }

    if !Path::new(elf_blob).exists() {
        eprintln!("Application elf binary `{}` not found!", elf_blob);
        exit(1);
    }

    eprintln!("Starting {} with {}", kernel_blob, elf_blob);

    let mut kvm = kvmvm::KvmVm::vm_create_default(&kernel_blob, &elf_blob, 0).unwrap();

    loop {
        let ret = kvm
            .cpu_fd
            .get(0)
            .unwrap()
            .run()
            .expect("Hypervisor: VM run failed");

        match ret {
            VcpuExit::IoOut(port, data) => match port {
                // Qemu exit simulation
                PORT_QEMU_EXIT if data.eq(&[0x10, 0, 0, 0]) => {
                    let elapsed = start.elapsed();
                    eprintln!("Hypervisor: Creating and running took {:?}", elapsed);
                    std::process::exit(0);
                }
                PORT_QEMU_EXIT if data.eq(&[0x11, 0, 0, 0]) => {
                    std::process::exit(1);
                }
                SYSCALL_TRIGGER_PORT => {
                    if let Err(e) = kvm.handle_syscall() {
                        panic!("Handle syscall: {:#?}", e);
                    }
                }
                _ => {
                    let regs = kvm.cpu_fd.get(0).unwrap().get_regs().unwrap();
                    panic!(
                        "Hypervisor: Unexpected IO port {:#X} {:#?}!\n{:#?}",
                        port, data, regs
                    )
                }
            },
            VcpuExit::Hlt => {
                let elapsed = start.elapsed();
                eprintln!("Hypervisor: VcpuExit::Hlt");
                eprintln!("Hypervisor: Creating and running took {:?}", elapsed);
                break;
            }
            exit_reason => {
                let regs = kvm.cpu_fd.get(0).unwrap().get_regs().unwrap();
                eprintln!(
                    "Hypervisor: unexpected exit reason: {:?}\n{:#?}",
                    exit_reason, regs
                );
                std::process::exit(1);
            }
        }
    }
    eprintln!("Hypervisor: Done");
}
