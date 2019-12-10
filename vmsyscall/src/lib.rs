#![no_std]

pub const PORT: u16 = 0xFF;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum KvmSyscall {
    Madvise {
        addr: usize,
        len: usize,
        advice: i32,
    },
    Mmap {
        addr: usize,
        len: usize,
        prot: i32,
        flags: i32,
    },
    Mremap {
        addr: usize,
        len: usize,
        new_len: usize,
        flags: i32,
    },
    Munmap {
        addr: usize,
        len: usize,
    },
    Mprotect {
        addr: usize,
        len: usize,
        prot: i32,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum KvmSyscallRet {
    Madvise(Result<i32, Error>),
    Mmap(Result<usize, Error>),
    Mremap(Result<usize, Error>),
    Munmap(Result<i32, Error>),
    Mprotect(Result<i32, Error>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Error {
    ENOMEM,
    NotImplemented,
    SerializeError,
    DeSerializeError,
}
