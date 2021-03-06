use super::mmap::*;
use crate::{serial_print, serial_println};
use linux_errno::ErrNo;
pub use vmsyscall::Error;

#[test_case]
fn test_madvise() {
    serial_print!("test_madvise...");
    let ret = madvise(core::ptr::null_mut(), 0, 0).unwrap_err();
    assert_eq!(ret, Error::Errno(ErrNo::ENOSYS.into()));
    serial_println!("[ok]");
}

#[test_case]
fn test_mmap() {
    serial_print!("test_mmap...");
    let ret = mmap(core::ptr::null_mut(), 0, 0, 0).unwrap_err();
    assert_eq!(ret, Error::Errno(ErrNo::ENOSYS.into()));
    serial_println!("[ok]");
}

#[test_case]
fn test_mremap() {
    serial_print!("test_mremap...");
    let ret = mremap(core::ptr::null_mut(), 0, 0, 0).unwrap_err();
    assert_eq!(ret, Error::Errno(ErrNo::ENOSYS.into()));
    serial_println!("[ok]");
}

#[test_case]
fn test_munmap() {
    serial_print!("test_munmap...");
    let ret = munmap(core::ptr::null_mut(), 0).unwrap_err();
    assert_eq!(ret, Error::Errno(ErrNo::ENOSYS.into()));
    serial_println!("[ok]");
}

#[test_case]
fn test_mprotect() {
    serial_print!("test_mprotect...");
    let ret = mprotect(core::ptr::null_mut(), 0, 0).unwrap_err();
    assert_eq!(ret, Error::Errno(ErrNo::ENOSYS.into()));
    serial_println!("[ok]");
}
