use crate::arch::InterruptStack;
use crate::{exit_hypervisor, println, serial_print, HyperVisorExitCode};

const SYS_EXIT: usize = 60;
const SYS_WRITE: usize = 1;

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

    match a {
        SYS_EXIT => {
            exit_hypervisor(if b == 0 {
                HyperVisorExitCode::Success
            } else {
                HyperVisorExitCode::Failed
            });
            loop {}
        }
        SYS_WRITE => {
            let fd = b;
            let data = c as *const u8;
            if fd == 1 {
                let mut len = 0;
                let mut cptr = data;
                unsafe {
                    // FIXME: poor man's very unsafe strlen(data)
                    loop {
                        if cptr.read() == 0 {
                            break;
                        }
                        len += 1;
                        cptr = (c + len) as *const u8;
                    }
                    let cstr = core::slice::from_raw_parts(c as *const u8, len);
                    let s = core::str::from_utf8_unchecked(cstr);
                    serial_print!("SYS_WRITE: {}", s);
                }
                len
            } else {
                -77i64 as usize // EBADFD
            }
        }
        _ => {
            -38i64 as usize // ENOSYS
        }
    }
}
