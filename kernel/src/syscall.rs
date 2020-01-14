use crate::arch::x86_64::{mmap_user, NEXT_MMAP};
use crate::arch::InterruptStack;
use crate::{exit_hypervisor, println, serial_print, HyperVisorExitCode};
use vmbootspec::layout::USER_HEAP_OFFSET;
use vmsyscall::errno;
use vmsyscall::syscall::*;

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
    match a as u64 {
        SYSCALL_EXIT => {
            println!("exit({})", b);
            exit_hypervisor(if b == 0 {
                HyperVisorExitCode::Success
            } else {
                HyperVisorExitCode::Failed
            });
            loop {}
        }
        SYSCALL_EXIT_GROUP => {
            println!("exit_group({})", b);
            exit_hypervisor(if b == 0 {
                HyperVisorExitCode::Success
            } else {
                HyperVisorExitCode::Failed
            });
            loop {}
        }
        SYSCALL_WRITE => {
            let fd = b;
            let data = c as *const u8;
            let len = d;
            if fd == 1 {
                let cstr = unsafe { core::slice::from_raw_parts(data, len) };
                match core::str::from_utf8(cstr) {
                    Ok(s) => {
                        println!("write({}, {:#?}) = {}", b, s, len);
                        len
                    }
                    Err(_) => {
                        println!("write({}, …) = -EINVAL", b);
                        -errno::EINVAL as _
                    }
                }
            } else {
                println!("write({}, \"…\") = -EBADFD", b);
                -errno::EBADFD as _
            }
        }
        SYSCALL_ARCH_PRCTL => {
            const ARCH_SET_GS: usize = 0x1001;
            const ARCH_SET_FS: usize = 0x1002;
            const ARCH_GET_FS: usize = 0x1003;
            const ARCH_GET_GS: usize = 0x1004;

            match b {
                ARCH_SET_FS => {
                    println!("arch_prctl(ARCH_SET_FS, 0x{:X}) = 0", c);
                    stack.fs = c;
                    0
                }
                ARCH_GET_FS => unimplemented!(),
                ARCH_SET_GS => unimplemented!(),
                ARCH_GET_GS => unimplemented!(),
                x => {
                    println!("arch_prctl(0x{:X}, 0x{:X}) = -EINVAL", x, c);
                    -errno::EINVAL as _
                }
            }
        }
        SYSCALL_MMAP => {
            println!("mmap(0x{:X}, {}, …)", b, c);
            if b == 0 {
                let ret = mmap_user(c);
                println!("Syscall::MMap = {:#?}", ret);
                ret as _
            } else {
                todo!();
            }
        }
        SYSCALL_BRK => unsafe {
            match b {
                0 => {
                    println!("brk(0x{:X}) = 0x{:X}", b, NEXT_MMAP);
                    NEXT_MMAP as _
                }
                n => {
                    mmap_user(n - NEXT_MMAP as usize);
                    println!("brk(0x{:X}) = 0x{:X}", b, NEXT_MMAP);
                    n as _
                }
            }
        },
        SYSCALL_UNAME => {
            println!(
                r##"uname({{sysname="Linux", nodename="enarx", release="5.4.8", version="1", machine="x86_64", domainname="(none)"}}) = 0"##
            );
            #[repr(C)]
            struct NewUtsname {
                sysname: [u8; 65],
                nodename: [u8; 65],
                release: [u8; 65],
                version: [u8; 65],
                machine: [u8; 65],
                domainname: [u8; 65],
            };
            let uts_ptr: *mut NewUtsname = b as _;
            unsafe {
                (*uts_ptr).sysname[..6].copy_from_slice(b"Linux\0");
                (*uts_ptr).nodename[..6].copy_from_slice(b"enarx\0");
                (*uts_ptr).release[..6].copy_from_slice(b"5.4.8\0");
                (*uts_ptr).version[..2].copy_from_slice(b"1\0");
                (*uts_ptr).machine[..7].copy_from_slice(b"x86_64\0");
                (*uts_ptr).domainname[..1].copy_from_slice(b"\0");
            }
            0
        }
        SYSCALL_READLINK => {
            use cstrptr::CStr;
            let pathname = unsafe { CStr::from_ptr(b as _) };
            let outbuf = unsafe { core::slice::from_raw_parts_mut(c as _, d as _) };
            outbuf[..6].copy_from_slice(b"/init\0");
            println!(
                "readlink({:#?}, \"/init\", {}) = 5",
                pathname.to_string_lossy(),
                d
            );
            5
        }
        SYSCALL_RT_SIGACTION => {
            println!("rt_sigaction(…) = 0");
            0
        }
        SYSCALL_RT_SIGPROCMASK => {
            println!("rt_sigprocmask(…) = 0");
            0
        }
        SYSCALL_SIGALTSTACK => {
            println!("sigaltstack(…) = 0");
            0
        }
        SYSCALL_SET_TID_ADDRESS => {
            println!("set_tid_address(…) = 63618");
            63618
        }
        _ => {
            println!("syscall({}, {}, {}, {}, {}, {})", a, b, c, d, e, f);
            stack.dump();
            panic!("syscall {} not yet implemented", a)
        } //-ENOSYS as usize,
    }
}
