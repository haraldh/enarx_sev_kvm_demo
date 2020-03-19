// SPDX-License-Identifier: Apache-2.0

//! syscall serialize/deserialize
//!
//! Currently it uses a hard coded page and an I/O trigger.
//! We might want to switch to MMIO.

#![deny(missing_docs)]
#![deny(clippy::all)]
#![no_std]

use core::fmt::{Debug, Formatter};

impl Debug for VmSyscall {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            VmSyscall::Read { .. } => f.write_str("read(2)"),
            VmSyscall::Write { .. } => f.write_str("write(2)"),
            VmSyscall::Madvise { .. } => f.write_str("madvise(2)"),
            VmSyscall::Mmap { .. } => f.write_str("mmap(2)"),
            VmSyscall::Mremap { .. } => f.write_str("mremap(2)"),
            VmSyscall::Munmap { .. } => f.write_str("munmap(2)"),
            VmSyscall::Mprotect { .. } => f.write_str("mprotect(2)"),
        }
    }
}

/// The syscalls to be serialized/deserialized via serde
/// for the Hypervisor <-> VM syscall proxy
pub enum VmSyscall {
    /// ssize_t read(int fd, void *buf, size_t count);
    Read {
        /// see read(2)
        fd: u32,
        /// see read(2)
        count: usize,
    },
    /// ssize_t write(int fd, const void *buf, size_t count);
    Write {
        /// see write(2)
        fd: u32,
        /// see write(2)
        count: usize,
        /// see write(2)
        data: [u8; 4000],
    },
    /// int madvise(void *addr, size_t length, int advice);
    Madvise {
        /// see madvise(2)
        addr: usize,
        /// see madvise(2)
        length: usize,
        /// see madvise(2)
        advice: i32,
    },
    /// void *mmap(void *addr, size_t length, int prot, int flags, …);
    Mmap {
        /// see mmap(2)
        addr: usize,
        /// see mmap(2)
        length: usize,
        /// see mmap(2)
        prot: i32,
        /// see mmap(2)
        flags: i32,
    },
    /// void *mremap(void *old_address, size_t old_size, size_t new_size, int flags, ... /* void *new_address */);
    Mremap {
        /// see mremap(2)
        old_address: usize,
        /// see mremap(2)
        old_size: usize,
        /// see mremap(2)
        new_size: usize,
        /// see mremap(2)
        flags: i32,
    },
    /// int munmap(void *addr, size_t length);
    Munmap {
        /// see munmap(2)
        addr: usize,
        /// see munmap(2)
        length: usize,
    },
    /// int mprotect(void *addr, size_t len, int prot);
    Mprotect {
        /// see mprotect(2)
        addr: usize,
        /// see mprotect(2)
        length: usize,
        /// see mprotect(2)
        prot: i32,
    },
    // Todo: extend with needed hypervisor proxy syscalls
}

/// The return value of the syscalls to be serialized/deserialized via serde
/// for the Hypervisor <-> VM syscall proxy
pub enum VmSyscallRet {
    /// ssize_t read(int fd, void *buf, size_t count);
    Read(Result<(i32, [u8; 4000]), Error>),
    /// ssize_t write(int fd, const void *buf, size_t count);
    Write(Result<i32, Error>),
    /// int madvise(void *addr, size_t length, int advice);
    Madvise(Result<i32, Error>),
    /// void *mmap(void *addr, size_t length, int prot, int flags, …);
    Mmap(Result<usize, Error>),
    /// void *mremap(void *old_address, size_t old_size, size_t new_size, int flags, ... /* void *new_address */);
    Mremap(Result<usize, Error>),
    /// int munmap(void *addr, size_t length);
    Munmap(Result<i32, Error>),
    /// int mprotect(void *addr, size_t len, int prot);
    Mprotect(Result<i32, Error>),
}

/// The error codes of the syscalls to be serialized/deserialized via serde
/// for the Hypervisor <-> VM syscall proxy
#[derive(Debug, PartialEq)]
pub enum Error {
    /// standard error
    Errno(i64),
    /// serialize error
    SerializeError,
    /// deserialize error
    DeSerializeError,
}
