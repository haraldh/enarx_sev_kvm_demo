use enarx_boot_spec::layout::SYSCALL_PHYS_ADDR;
use serde::ser::Serialize;
use serde_cbor;
use serde_cbor::ser::SliceWrite;
use serde_cbor::Serializer;
pub use vmsyscall::Error;
use vmsyscall::{KvmSyscall, KvmSyscallRet, PORT};
use x86_64::instructions::port::Port;
use x86_64::VirtAddr;

mod mmap;
pub use mmap::*;

#[cfg(test)]
mod test;

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

#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum c_void {
    #[doc(hidden)]
    __variant1,
    #[doc(hidden)]
    __variant2,
}
