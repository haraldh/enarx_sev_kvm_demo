use crate::arch::InterruptStack;
use crate::{exit_hypervisor, println, serial_print, HyperVisorExitCode};
use vmsyscall::error::*;

enum Syscall {
    Write = 1,
    Exit = 60,
    ArchPrctl = 158,
}

pub fn handle_syscall(
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
    _stack_base_bp: usize,
    stack: &mut InterruptStack,
) -> usize {
    println!("syscall({}, {}, {}, {}, {}, {})", a, b, c, d, e, f);
    stack.dump();

    match a {
        x if x == Syscall::Exit as usize => {
            println!("Syscall::Exit");
            exit_hypervisor(if b == 0 {
                HyperVisorExitCode::Success
            } else {
                HyperVisorExitCode::Failed
            });
            loop {}
        }
        x if x == Syscall::Write as usize => {
            println!("Syscall::Write");
            let fd = b;
            let data = c as *const u8;
            let len = d;
            if fd == 1 {
                let cstr = unsafe { core::slice::from_raw_parts(data, len) };
                match core::str::from_utf8(cstr) {
                    Ok(s) => {
                        serial_print!("SYS_WRITE: {}", s);
                        len
                    }
                    Err(_) => -EINVAL as usize,
                }
            } else {
                -EBADFD as usize
            }
        }
        x if x == Syscall::ArchPrctl as usize => {
            println!("Syscall::ArchPrctl");
            enum ArchPrctlCode {
                ArchSetGs = 0x1001,
                ArchSetFs = 0x1002,
                ArchGetFs = 0x1003,
                ArchGetGs = 0x1004,
            };
            match b {
                x if x == ArchPrctlCode::ArchSetFs as usize => unimplemented!(),
                x if x == ArchPrctlCode::ArchGetFs as usize => unimplemented!(),
                x if x == ArchPrctlCode::ArchSetGs as usize => unimplemented!(),
                x if x == ArchPrctlCode::ArchGetGs as usize => unimplemented!(),
                _ => -EINVAL as usize,
            }
        }
        _ => unimplemented!(), //-ENOSYS as usize,
    }
}
