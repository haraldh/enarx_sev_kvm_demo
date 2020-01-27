//! Common interface between the hypervisor and kernel
//!
//! including hard coded memory layout and the bootinfo
//! structure handed over to the start of the kernel
//!
//! copied from
//! https://github.com/rust-osdev/bootloader/blob/90f5b8910d146d6d489b70a6341d778253663cfa/src/bootinfo/mod.rs

#![no_std]
#![deny(improper_ctypes)]

use core::fmt;

pub use self::memory_map::*;

pub mod layout;
mod memory_map;

/// Defines the entry point function.
///
/// The function must have the signature `fn(&'static BootInfo) -> !`.
///
/// This macro just creates a function named `_start`, which the linker will use as the entry
/// point. The advantage of using this macro instead of providing an own `_start` function is
/// that the macro ensures that the function and argument types are correct.
#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        #[export_name = "_start"]
        pub extern "C" fn __impl_start(boot_info: &'static mut $crate::BootInfo) -> ! {
            // validate the signature of the program entry point
            let f: fn(&'static mut $crate::BootInfo) -> ! = $path;

            f(boot_info)
        }
    };
}

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
#[repr(C)]
pub struct BootInfo {
    /// A map of the physical memory regions of the underlying machine.
    ///
    /// The bootloader queries this information from the BIOS/UEFI firmware and translates this
    /// information to Rust types. It also marks any memory regions that the bootloader uses in
    /// the memory map before passing it to the kernel. Regions marked as usable can be freely
    /// used by the kernel.
    pub memory_map: MemoryMap,
    /// The virtual address of the recursively mapped level 4 page table.
    //pub recursive_page_table_addr: u64,
    /// The offset into the virtual address space where the physical memory is mapped.
    ///
    /// Physical addresses can be converted to virtual addresses by adding this offset to them.
    ///
    /// The mapping of the physical memory allows to access arbitrary physical frames. Accessing
    /// frames that are also mapped at other virtual addresses can easily break memory safety and
    /// cause undefined behavior. Only frames reported as `USABLE` by the memory map in the `BootInfo`
    /// can be safely accessed.
    pub physical_memory_offset: u64,
    pub recursive_page_table_addr: u64,
}

impl BootInfo {
    /// Create a new boot information structure. This function is only for internal purposes.
    #[allow(unused_variables)]
    #[doc(hidden)]
    pub const fn new(
        memory_map: MemoryMap,
        recursive_page_table_addr: u64,
        physical_memory_offset: u64,
    ) -> Self {
        BootInfo {
            memory_map,
            recursive_page_table_addr,
            physical_memory_offset,
        }
    }
}

impl fmt::Debug for BootInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BootInfo")
            .field("memory_map", &self.memory_map)
            .field(
                "physical_memory_offset",
                &format_args!("{:#X}", self.physical_memory_offset),
            )
            .field(
                "recursive_page_table_addr",
                &format_args!("{:#X}", self.recursive_page_table_addr),
            )
            .finish()
    }
}

extern "C" {
    fn _improper_ctypes_check(_boot_info: BootInfo);
}
