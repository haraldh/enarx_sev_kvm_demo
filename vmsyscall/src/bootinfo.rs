//! Common interface between the hypervisor and kernel
//!
//! including hard coded memory layout and the bootinfo
//! structure handed over to the start of the kernel
//!
//! copied from
//! https://github.com/rust-osdev/bootloader/blob/90f5b8910d146d6d489b70a6341d778253663cfa/src/bootinfo/mod.rs

use crate::memory_map::MemoryMap;
use core::fmt;

/// Hard coded trigger port
pub const SYSCALL_TRIGGER_PORT: u16 = 0xFF;

/// This structure represents the information that the bootloader passes to the kernel.
///
/// The information is passed as an argument to the entry point:
///
/// ```ignore
/// pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
///    // […]
/// }
/// ```
///
/// Note that no type checking occurs for the entry point function, so be careful to
/// use the correct argument types. To ensure that the entry point function has the correct
/// signature, use the [`entry_point`] macro.
#[derive(Clone)]
#[repr(C)]
pub struct BootInfo {
    /// A map of the physical memory regions of the underlying machine.
    ///
    /// The bootloader queries this information from the BIOS/UEFI firmware and translates this
    /// information to Rust types. It also marks any memory regions that the bootloader uses in
    /// the memory map before passing it to the kernel. Regions marked as usable can be freely
    /// used by the kernel.
    pub memory_map: MemoryMap,
    /// Elf entry point of the ring3 executable
    pub entry_point: *const u8,
    /// Elf program headers of the ring3 executable
    pub load_addr: *const u8,
    /// Elf number of program headers of the ring3 executable
    pub elf_phnum: usize,
    /// Syscall trigger port
    pub syscall_trigger_port: u16,
}

impl fmt::Debug for BootInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BootInfo")
            .field("memory_map", &self.memory_map)
            .finish()
    }
}

extern "C" {
    fn _improper_ctypes_check(_boot_info: BootInfo);
}

#[test]
fn check_bootinfo_size() {
    use crate::memory_map::PAGE_SIZE;
    assert!(core::mem::size_of::<BootInfo>() <= (PAGE_SIZE as _));
}
