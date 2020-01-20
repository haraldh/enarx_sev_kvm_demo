use crate::arch::x86_64::{brk_user, mmap_user, NEXT_MMAP};
//use crate::arch::SyscallStack;
use crate::{eprintln, exit_hypervisor, print, println, HyperVisorExitCode};
//use vmbootspec::layout::USER_HEAP_OFFSET;
use linux_errno::*;
use linux_syscall::*;

trait NegAsUsize {
    fn neg_as_usize(self) -> usize;
}

impl NegAsUsize for Errno {
    fn neg_as_usize(self) -> usize {
        -Into::<i64>::into(self) as _
    }
}

pub fn handle_syscall(
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
    nr: usize,
) -> usize {
    eprintln!(
        "> syscall({}, 0x{:X}, 0x{:X}, 0x{:X}, {}, {}, 0x{:X})",
        nr, a, b, c, d, e, f
    );
    match (nr as u64).into() {
        SYSCALL_EXIT => {
            println!("exit({})", a);
            exit_hypervisor(if a == 0 {
                HyperVisorExitCode::Success
            } else {
                HyperVisorExitCode::Failed
            });
            loop {}
        }
        SYSCALL_EXIT_GROUP => {
            println!("exit_group({})", a);
            exit_hypervisor(if a == 0 {
                HyperVisorExitCode::Success
            } else {
                HyperVisorExitCode::Failed
            });
            loop {}
        }
        SYSCALL_WRITE => {
            let fd = a;
            let data = b as *const u8;
            let len = c;
            match fd {
                1 | 2 => {
                    let cstr = unsafe { core::slice::from_raw_parts(data, len) };
                    match core::str::from_utf8(cstr) {
                        Ok(s) => {
                            print!("{}", s);
                            eprintln!("write({}, {:#?}) = {}", fd, s, len);
                            len
                        }
                        Err(_) => {
                            eprintln!("write({}, …) = -EINVAL", fd);
                            EINVAL.neg_as_usize()
                        }
                    }
                }
                _ => {
                    eprintln!("write({}, \"…\") = -EBADFD", a);
                    EBADFD.neg_as_usize()
                }
            }
        }
        SYSCALL_ARCH_PRCTL => {
            const ARCH_SET_GS: usize = 0x1001;
            const ARCH_SET_FS: usize = 0x1002;
            const ARCH_GET_FS: usize = 0x1003;
            const ARCH_GET_GS: usize = 0x1004;

            match a {
                ARCH_SET_FS => {
                    println!("arch_prctl(ARCH_SET_FS, 0x{:X}) = 0", b);
                    let value: u64 = b as _;
                    unsafe {
                        asm!("wrfsbase $0" :: "r" (value) );
                    }
                    0
                }
                ARCH_GET_FS => unimplemented!(),
                ARCH_SET_GS => unimplemented!(),
                ARCH_GET_GS => unimplemented!(),
                x => {
                    println!("arch_prctl(0x{:X}, 0x{:X}) = -EINVAL", x, b);
                    EINVAL.neg_as_usize()
                }
            }
        }
        SYSCALL_MMAP => {
            println!("mmap(0x{:X}, {}, …)", a, b);
            if a == 0 {
                let ret = mmap_user(b);
                println!("Syscall::MMap = {:#?}", ret);
                ret as _
            } else {
                todo!();
            }
        }
        SYSCALL_BRK => unsafe {
            match a {
                0 => {
                    println!("brk(0x{:X}) = 0x{:X}", a, NEXT_MMAP);
                    NEXT_MMAP as _
                }
                n => {
                    brk_user(n - NEXT_MMAP as usize);
                    println!("brk(0x{:X}) = 0x{:X}", a, NEXT_MMAP);
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
            let uts_ptr: *mut NewUtsname = a as _;
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
            let pathname = unsafe { CStr::from_ptr(a as _) };
            let outbuf = unsafe { core::slice::from_raw_parts_mut(b as _, c as _) };
            outbuf[..6].copy_from_slice(b"/init\0");
            println!(
                "readlink({:#?}, \"/init\", {}) = 5",
                pathname.to_string_lossy(),
                c
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
        SYSCALL_IOCTL => match a {
            1 => {
                match b {
                    0x5413 /* TIOCGWINSZ */ => {
                        #[repr(C, packed)]
                        struct WinSize {
                            ws_row: u16,
                            ws_col: u16,
                            ws_xpixel: u16,
                            ws_ypixel: u16,
                        };
                        let p: *mut WinSize = c as _;
                        let winsize = WinSize {
                            ws_row: 40,
                            ws_col: 80,
                            ws_xpixel: 0,
                            ws_ypixel: 0
                        };
                        unsafe {
                            p.write_volatile(winsize);
                        }
                        println!("ioctl(1, TIOCGWINSZ, {{ws_row=40, ws_col=80, ws_xpixel=0, ws_ypixel=0}}) = 0");
                        0
                    },
                    _ => EINVAL.neg_as_usize(),
                }
            }
            _ => EINVAL.neg_as_usize(),
        },
        _ => {
            println!("syscall({}, {}, {}, {}, {}, {}, {})", nr, a, b, c, d, e, f);
            //stack.dump();
            panic!("syscall {} not yet implemented", nr)
            // ENOSYS.neg_as_usize(),
        }
    }
}
