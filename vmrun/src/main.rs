use kvm_ioctls::VcpuExit;
use std::time::Instant;
use vmrun::kvm_util;

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

    let kvm = kvm_util::KvmVm::vm_create_default(&kernel_blob, 0, 1024, None /*"_start"*/).unwrap();

    use std::io::{self, Write};
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    loop {
        match kvm.cpu_fd[0].run().expect("Hypervisor: VM run failed") {
            VcpuExit::IoOut(port, data) => match port {
                // Qemu exit simulation
                0xf4 if data.eq(&[0x3d, 0, 0, 0]) => {
                    let elapsed = start.elapsed();
                    eprintln!("Hypervisor: qemu-exit trigger");
                    eprintln!("Hypervisor: Creating and running took {:?}", elapsed);
                    break;
                }
                0xf4 if data.eq(&[0x10, 0, 0, 0]) => {
                    std::process::exit(0);
                }
                0xf4 if data.eq(&[0x11, 0, 0, 0]) => {
                    std::process::exit(1);
                }
                // Serial line out
                0x03f8 => {
                    let _err = handle.write_all(data);
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
