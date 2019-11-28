use boot::layout::SYSCALL_PHYS_ADDR;
use serde::ser::Serialize;
use serde_cbor;
use serde_cbor::ser::SliceWrite;
use serde_cbor::Serializer;
pub use vmsyscall::Error;
use vmsyscall::{KvmSyscall, KvmSyscallRet, PORT};
use x86_64::instructions::port::Port;
use x86_64::VirtAddr;

pub fn vm_syscall(syscall: KvmSyscall) -> Result<KvmSyscallRet, Error> {
    let syscall_page = VirtAddr::new(SYSCALL_PHYS_ADDR);

    let mut syscall_slice =
        unsafe { core::slice::from_raw_parts_mut(syscall_page.as_u64() as *mut u8, 4096 as usize) };

    syscall_slice.iter_mut().for_each(|d| *d = 0);

    let writer = SliceWrite::new(&mut syscall_slice);
    let mut ser = Serializer::new(writer);

    syscall
        .serialize(&mut ser)
        .map_err(|_| Error::SerializeError)?;

    let writer = ser.into_inner();
    let mut size = writer.bytes_written();

    unsafe {
        let mut port = Port::<u16>::new(PORT);
        port.write(size as u16);
        size = port.read() as usize;
    }

    let mut syscall_slice =
        unsafe { core::slice::from_raw_parts_mut(syscall_page.as_u64() as *mut u8, size) };

    // FIXME: unwrap
    serde_cbor::de::from_mut_slice(&mut syscall_slice).map_err(|_| Error::DeSerializeError)
}

pub fn madvise(addr: *mut c_void, len: usize, advice: i32) -> Result<i32, Error> {
    let s = KvmSyscall::Madvise {
        addr: addr as usize,
        len,
        advice,
    };
    let ret = vm_syscall(s)?;
    match ret {
        KvmSyscallRet::Madvise(res) => res,
        _ => panic!("Unknown KvmSyscallRet"),
    }
}

pub fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32) -> Result<*mut c_void, Error> {
    let s = KvmSyscall::Mmap {
        addr: addr as usize,
        len,
        prot,
        flags,
    };
    let ret = vm_syscall(s)?;
    match ret {
        KvmSyscallRet::Mmap(res) => match res {
            Ok(l) => Ok(l as _),
            Err(e) => Err(e),
        },
        _ => panic!("Unknown KvmSyscallRet"),
    }
}

pub fn mremap(
    addr: *mut c_void,
    len: usize,
    new_len: usize,
    flags: i32,
) -> Result<*mut c_void, Error> {
    let s = KvmSyscall::Mremap {
        addr: addr as usize,
        len,
        new_len,
        flags,
    };
    let ret = vm_syscall(s)?;
    match ret {
        KvmSyscallRet::Mremap(res) => match res {
            Ok(l) => Ok(l as _),
            Err(e) => Err(e),
        },
        _ => panic!("Unknown KvmSyscallRet"),
    }
}

pub fn munmap(addr: *mut c_void, len: usize) -> Result<i32, Error> {
    let s = KvmSyscall::Munmap {
        addr: addr as usize,
        len,
    };
    let ret = vm_syscall(s)?;
    match ret {
        KvmSyscallRet::Madvise(res) => res,
        _ => panic!("Unknown KvmSyscallRet"),
    }
}

pub fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> Result<i32, Error> {
    let s = KvmSyscall::Mprotect {
        addr: addr as usize,
        len,
        prot,
    };
    let ret = vm_syscall(s)?;
    match ret {
        KvmSyscallRet::Madvise(res) => res,
        _ => panic!("Unknown KvmSyscallRet"),
    }
}

#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum c_void {
    #[doc(hidden)]
    __variant1,
    #[doc(hidden)]
    __variant2,
}

pub const MREMAP_MAYMOVE: i32 = 1;
pub const MREMAP_FIXED: i32 = 2;

pub const MADV_NORMAL: i32 = 0;
pub const MADV_RANDOM: i32 = 1;
pub const MADV_SEQUENTIAL: i32 = 2;
pub const MADV_WILLNEED: i32 = 3;
pub const MADV_DONTNEED: i32 = 4;
pub const MADV_FREE: i32 = 8;
pub const MADV_REMOVE: i32 = 9;
pub const MADV_DONTFORK: i32 = 10;
pub const MADV_DOFORK: i32 = 11;
pub const MADV_MERGEABLE: i32 = 12;
pub const MADV_UNMERGEABLE: i32 = 13;
pub const MADV_HUGEPAGE: i32 = 14;
pub const MADV_NOHUGEPAGE: i32 = 15;
pub const MADV_DONTDUMP: i32 = 16;
pub const MADV_DODUMP: i32 = 17;
pub const MADV_HWPOISON: i32 = 100;
pub const PROT_NONE: i32 = 0;
pub const PROT_READ: i32 = 1;
pub const PROT_WRITE: i32 = 2;
pub const PROT_EXEC: i32 = 4;
pub const ENOMEM: i32 = 12;
pub const MAP_HUGETLB: i32 = 0x040000;
pub const MAP_LOCKED: i32 = 0x02000;
pub const MAP_NORESERVE: i32 = 0x04000;
pub const MAP_32BIT: i32 = 0x0040;
pub const MAP_ANON: i32 = 0x0020;
pub const MAP_ANONYMOUS: i32 = 0x0020;
pub const MAP_DENYWRITE: i32 = 0x0800;
pub const MAP_EXECUTABLE: i32 = 0x01000;
pub const MAP_POPULATE: i32 = 0x08000;
pub const MAP_NONBLOCK: i32 = 0x010000;
pub const MAP_STACK: i32 = 0x020000;

pub const MAP_FILE: i32 = 0x0000;
pub const MAP_SHARED: i32 = 0x0001;
pub const MAP_PRIVATE: i32 = 0x0002;
pub const MAP_FIXED: i32 = 0x0010;

pub const MAP_FAILED: *mut c_void = !0 as *mut c_void;
