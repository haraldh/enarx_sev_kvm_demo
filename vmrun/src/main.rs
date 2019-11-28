use kvm_ioctls::VcpuExit;
use serde::ser::Serialize;
use serde_cbor;
use serde_cbor::ser::SliceWrite;
use serde_cbor::Serializer;
use std::io;
use std::io::Write;
use std::time::Instant;
use vmrun::kvm_util;
use vmsyscall::PORT as PORT_SYSCALL;
use vmsyscall::{KvmSyscall, KvmSyscallRet};

const PORT_SERIAL_OUT: u16 = 0x03F8;
const PORT_QEMU_EXIT: u16 = 0xF4;

fn main() {
    let start = Instant::now();

    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!(
            "Usage: {} <kernelblob> - {:#?}",
            args[0],
            std::env::current_dir()
        );
        std::process::exit(1);
    }

    let kernel_blob = args[1].to_string();
    eprintln!("Starting {}", kernel_blob);

    let kvm = kvm_util::KvmVm::vm_create_default(&kernel_blob, 0, None /*"_start"*/).unwrap();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let mut syscall: Option<KvmSyscall> = None;
    let cpu_fd = kvm.cpu_fd.get(0).unwrap();

    loop {
        let ret = cpu_fd.run().expect("Hypervisor: VM run failed");
        match ret {
            VcpuExit::IoIn(port, data) => match port {
                PORT_SYSCALL => {
                    let syscall_page = kvm.syscall_hostvaddr.unwrap();

                    let mut syscall_slice = unsafe {
                        core::slice::from_raw_parts_mut(syscall_page.as_u64() as *mut u8, 4096 as _)
                    };

                    let ret = kvm.handle_syscall(syscall.take().unwrap());

                    let writer = SliceWrite::new(&mut syscall_slice);
                    let mut ser = Serializer::new(writer);

                    ret.serialize(&mut ser).unwrap();
                    let writer = ser.into_inner();
                    let size = writer.bytes_written();

                    data[0] = (size & 0xFF) as _;
                    data[1] = ((size >> 8) & 0xFF) as _;
                    continue;
                }
                _ => panic!("Hypervisor: Unexpected IO port {:#X}!", port),
            },
            VcpuExit::IoOut(port, data) => match port {
                // Qemu exit simulation
                PORT_QEMU_EXIT if data.eq(&[0x3d, 0, 0, 0]) => {
                    let elapsed = start.elapsed();
                    eprintln!("Hypervisor: qemu-exit trigger");
                    eprintln!("Hypervisor: Creating and running took {:?}", elapsed);
                    break;
                }
                PORT_QEMU_EXIT if data.eq(&[0x10, 0, 0, 0]) => {
                    std::process::exit(0);
                }
                PORT_QEMU_EXIT if data.eq(&[0x11, 0, 0, 0]) => {
                    std::process::exit(1);
                }
                // Serial line out
                PORT_SERIAL_OUT => {
                    handle.write_all(data).unwrap();
                }
                PORT_SYSCALL => {
                    let syscall_page = kvm.syscall_hostvaddr.unwrap();

                    let mut syscall_slice = unsafe {
                        core::slice::from_raw_parts_mut(
                            syscall_page.as_u64() as *mut u8,
                            (data[0] as u16 + data[1] as u16 * 256) as _,
                        )
                    };

                    syscall = serde_cbor::de::from_mut_slice(&mut syscall_slice).unwrap();

                    eprintln!("syscall in: {:#?}", syscall);
                    continue;
                }
                _ => panic!("Hypervisor: Unexpected IO port {:#X} {:#?}!", port, data),
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
    }
    eprintln!("Hypervisor: Done");
}
