use kvm_ioctls::VcpuExit;
use serde::ser::Serialize;
use serde_cbor;
use serde_cbor::ser::SliceWrite;
use serde_cbor::Serializer;
use std::path::Path;
use std::process::{exit, Command, Stdio};
use std::time::Instant;
use vmbootspec::layout::SYSCALL_TRIGGER_PORT;
use vmrun::kvmvm;
use vmsyscall::VmSyscall;

const PORT_QEMU_EXIT: u16 = 0xF4;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.len() {
        3 if args[1].eq("--fallback-qemu") => match Path::new("/dev/kvm").exists() {
            true => main_kvm(&args[2]),
            false => main_qemu(&args[2]),
        },
        2 => main_kvm(&args[1]),
        _ => {
            eprintln!("Usage: {} [--fallback-qemu] <kernelblob>", args[0],);
            exit(1);
        }
    }
}

fn main_qemu(kernel_blob: &str) -> ! {
    let start = Instant::now();

    eprintln!("Starting QEMU {}", kernel_blob);
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args(&[
        "-machine",
        "q35",
        "-cpu",
        "max",
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
        "-serial",
        "chardev:char0",
        "-serial",
        "chardev:char0",
        "-kernel",
    ])
    .arg(kernel_blob)
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit());
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

fn main_kvm(kernel_blob: &str) {
    let start = Instant::now();

    eprintln!("Starting {}", kernel_blob);

    let mut kvm = kvmvm::KvmVm::vm_create_default(&kernel_blob, 0, None /*"_start"*/).unwrap();

    let mut syscall_request: Option<VmSyscall> = None;
    let mut syscall_reply_size: Option<usize> = None;

    let mut portio = vmrun::device_manager::legacy::PortIODeviceManager::new().unwrap();
    let _ = portio.register_devices().unwrap();
    let _ = kvm.kvm_fd.register_irqfd(&portio.com_evt_1_3, 4).unwrap();
    let _ = kvm.kvm_fd.register_irqfd(&portio.com_evt_2_4, 3).unwrap();

    loop {
        let ret = kvm
            .cpu_fd
            .get(0)
            .unwrap()
            .run()
            .expect("Hypervisor: VM run failed");
        match ret {
            VcpuExit::IoIn(port, data) => match port {
                SYSCALL_TRIGGER_PORT => {
                    let size = syscall_reply_size.take().unwrap();
                    data[0] = (size & 0xFF) as _;
                    data[1] = ((size >> 8) & 0xFF) as _;
                    continue;
                }
                _ => {
                    portio.io_bus.read(port as _, data);
                } //_ => panic!("Hypervisor: Unexpected IO port {:#X}!", port),
            },
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
                    let syscall_page = kvm.syscall_hostvaddr.unwrap();

                    let mut syscall_slice = unsafe {
                        core::slice::from_raw_parts_mut(
                            syscall_page.as_u64() as *mut u8,
                            (data[0] as u16 + data[1] as u16 * 256) as _,
                        )
                    };

                    let s: VmSyscall = serde_cbor::de::from_mut_slice(&mut syscall_slice).unwrap();

                    syscall_request.replace(s);

                    //eprintln!("syscall in: {:#?}", syscall_request);
                }
                _ => {
                    portio.io_bus.write(port as _, data);
                } //_ => panic!("Hypervisor: Unexpected IO port {:#X} {:#?}!", port, data),
            },
            VcpuExit::Hlt => {
                let elapsed = start.elapsed();
                eprintln!("Hypervisor: VcpuExit::Hlt");
                eprintln!("Hypervisor: Creating and running took {:?}", elapsed);
                break;
            }
            exit_reason => {
                eprintln!("Hypervisor: unexpected exit reason: {:?}", exit_reason);
                std::process::exit(1);
            }
        }

        // Handle syscall request
        if let Some(syscall) = syscall_request.take() {
            let syscall_page = kvm.syscall_hostvaddr.unwrap();

            let mut syscall_slice = unsafe {
                core::slice::from_raw_parts_mut(syscall_page.as_u64() as *mut u8, 4096 as _)
            };

            let ret = kvm.handle_syscall(syscall);

            let writer = SliceWrite::new(&mut syscall_slice);
            let mut ser = Serializer::new(writer);

            ret.serialize(&mut ser).unwrap();
            let writer = ser.into_inner();
            syscall_reply_size.replace(writer.bytes_written());
        }
    }
    eprintln!("Hypervisor: Done");
}
