//! Provides boot information to the kernel.

#![no_std]
#![deny(improper_ctypes)]

pub use self::memory_map::*;
use core::fmt;

mod memory_map;

pub const BOOT_GDT_OFFSET: usize = 0x500;
pub const BOOT_IDT_OFFSET: usize = 0x520;

// Initial pagetables.
pub const PML4_START: usize = 0x9000;
pub const PDPTE_START: usize = 0xA000;
pub const PDE_START: usize = 0xB000;
pub const PDPTE_OFFSET_START: usize = 0xF000;

pub const PAGETABLE_LEN: u64 = core::mem::size_of::<PageTables>() as _;
pub const BOOTINFO_PHYS_ADDR: u64 = PML4_START as u64 + PAGETABLE_LEN;
pub const SYSCALL_PHYS_ADDR: u64 = BOOTINFO_PHYS_ADDR + 0x1000;
pub const HIMEM_START: usize = 0x0010_0000; //1 MB.
pub const BOOT_STACK_POINTER: u64 = HIMEM_START as u64 - 0x1000;
pub const BOOT_STACK_POINTER_SIZE: u64 = 0xE0000;

#[repr(C)]
pub struct PageTables {
    pub pml4t: [u64; 512],           // 0x9000
    pub pml3t_ident: [u64; 512],     // 0xA000
    pub pml2t_ident: [u64; 512 * 4], // 0xB000
    pub pml3t_offset: [u64; 512],    // 0xF000
}

impl Default for PageTables {
    fn default() -> Self {
        PageTables {
            pml4t: [0u64; 512],
            pml3t_ident: [0u64; 512],
            pml2t_ident: [0u64; 512 * 4],
            pml3t_offset: [0u64; 512],
        }
    }
}

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
        pub extern "C" fn __impl_start(boot_info: &'static $crate::BootInfo) -> ! {
            // validate the signature of the program entry point
            let f: fn(&'static $crate::BootInfo) -> ! = $path;

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
    pub fn new(
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
