use crate::arch::InterruptStack;
use crate::{exit_hypervisor, println, serial_print, HyperVisorExitCode};
use core::convert::TryFrom;

#[derive(Clone, Copy)]
enum Syscall {
    Write = 1,
    Exit = 60,
}

impl TryFrom<usize> for Syscall {
    type Error = ();

    fn try_from(v: usize) -> Result<Self, Self::Error> {
        match v {
            x if x == Syscall::Write as usize => Ok(Syscall::Write),
            x if x == Syscall::Exit as usize => Ok(Syscall::Exit),
            _ => Err(()),
        }
    }
}

pub fn handle_syscall(
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
    _stack_base_bp: usize,
    _stack: &mut InterruptStack,
) -> usize {
    println!("syscall({}, {}, {}, {}, {}, {})", a, b, c, d, e, f);
    //println!("syscall bp={}", stack_base_bp);
    //unsafe { println!("syscall rsp={:#X}", stack.iret.rsp) };

    match Syscall::try_from(a) {
        Ok(Syscall::Exit) => {
            exit_hypervisor(if b == 0 {
                HyperVisorExitCode::Success
            } else {
                HyperVisorExitCode::Failed
            });
            loop {}
        }
        Ok(Syscall::Write) => {
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
                    Err(_) => -22i64 as usize, // EINVAL
                }
            } else {
                -77i64 as usize // EBADFD
            }
        }
        _ => {
            -38i64 as usize // ENOSYS
        }
    }
}
