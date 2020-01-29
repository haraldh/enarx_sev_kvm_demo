use crate::arch::x86_64::PAGESIZE;
use serde::export::fmt::Error;
use serde::export::Formatter;
use vmbootspec::layout::{BOOTINFO_PHYS_ADDR, PHYSICAL_MEMORY_OFFSET, SYSCALL_PHYS_ADDR};
use vmbootspec::{BootInfo, FrameRange, MemoryMap, MemoryRegion, MemoryRegionType};
use x86_64::PhysAddr;

extern "C" {
    fn _start(bootinfo: *mut BootInfo) -> !;
}

#[repr(C, packed)]
pub struct HvmStartInfo {
    magic: u32, /* Contains the magic value 0x336ec578       */
    /* ("xEn3" with the 0x80 bit of the "E" set).*/
    version: u32,       /* Version of this structure.                */
    flags: u32,         /* SIF_xxx flags.                            */
    nr_modules: u32,    /* Number of modules passed to the kernel.   */
    modlist_paddr: u64, /* Physical address of an array of           */
    /* hvm_modlist_entry.                        */
    cmdline_paddr: u64, /* Physical address of the command line.     */
    rsdp_paddr: u64,    /* Physical address of the RSDP ACPI data    */
    /* structure.                                */
    /* All following fields only present in version 1 and newer */
    memmap_paddr: u64, /* Physical address of an array of           */
    /* hvm_memmap_table_entry.                   */
    memmap_entries: u32, /* Number of entries in the memmap table.    */
    /* Value will be zero if there is no memory  */
    /* map being provided.                       */
    reserved: u32, /* Must be zero.                             */
}

/// https://github.com/Xilinx/xen/blob/master/xen/include/public/arch-x86/hvm/start_info.h#L105
#[repr(C, packed)]
pub struct HvmMemmapTableEntry {
    addr: u64,       /* Base address of the memory region         */
    size: u64,       /* Size of the memory region in bytes        */
    entry_type: u32, /* Mapping type                              */
    reserved: u32,   /* Must be zero for Version 1.               */
}

impl HvmMemmapTableEntry {
    pub fn get_type(&self) -> HvmMemmapTableEntryType {
        const XEN_HVM_MEMMAP_TYPE_RAM: u32 = 1;
        const XEN_HVM_MEMMAP_TYPE_RESERVED: u32 = 2;
        const XEN_HVM_MEMMAP_TYPE_ACPI: u32 = 3;
        const XEN_HVM_MEMMAP_TYPE_NVS: u32 = 4;
        const XEN_HVM_MEMMAP_TYPE_UNUSABLE: u32 = 5;
        const XEN_HVM_MEMMAP_TYPE_DISABLED: u32 = 6;
        const XEN_HVM_MEMMAP_TYPE_PMEM: u32 = 7;

        match self.entry_type {
            XEN_HVM_MEMMAP_TYPE_RAM => HvmMemmapTableEntryType::RAM,
            XEN_HVM_MEMMAP_TYPE_RESERVED => HvmMemmapTableEntryType::Reserved,
            XEN_HVM_MEMMAP_TYPE_ACPI => HvmMemmapTableEntryType::ACPI,
            XEN_HVM_MEMMAP_TYPE_NVS => HvmMemmapTableEntryType::NVS,
            XEN_HVM_MEMMAP_TYPE_UNUSABLE => HvmMemmapTableEntryType::Unusable,
            XEN_HVM_MEMMAP_TYPE_DISABLED => HvmMemmapTableEntryType::Disabled,
            XEN_HVM_MEMMAP_TYPE_PMEM => HvmMemmapTableEntryType::PMEM,
            _ => HvmMemmapTableEntryType::Unknown,
        }
    }
}

impl alloc::fmt::Debug for HvmMemmapTableEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        unsafe {
            write!(
                f,
                "HvmMemmapTableEntry({:#?} {:#x}..{:#x})",
                self.get_type(),
                self.addr,
                self.addr + self.size
            )
        }
    }
}

#[derive(Debug)]
pub enum HvmMemmapTableEntryType {
    RAM,
    Reserved,
    ACPI,
    NVS,
    Unusable,
    Disabled,
    PMEM,
    Unknown,
}

extern "C" {
    static _app_start_addr: usize;
    static _app_size: usize;
    static _kernel_start: usize;
    static _kernel_end: usize;
    static pvh_stack: usize;
}

trait MemoryMapMarkAllocated {
    fn mark_allocated_region(&mut self, region: MemoryRegion);
}

impl MemoryMapMarkAllocated for MemoryMap {
    fn mark_allocated_region(&mut self, region: MemoryRegion) {
        for r in self.iter_mut() {
            if r.region_type == region.region_type
                && r.range.start_frame_number == region.range.start_frame_number
                && r.range.end_frame_number == region.range.end_frame_number
            {
                return;
            }

            if r.region_type == region.region_type
                && r.range.end_frame_number >= region.range.start_frame_number
                && r.range.end_frame_number <= region.range.end_frame_number
            {
                r.range.end_frame_number = region.range.start_frame_number;
            }

            if region.range.start_frame_number >= r.range.end_frame_number {
                continue;
            }
            if region.range.end_frame_number <= r.range.start_frame_number {
                continue;
            }

            if r.region_type != MemoryRegionType::Usable {
                panic!(
                    "region {:x?} overlaps with non-usable region {:x?}",
                    region, r
                );
            }

            if region.range.start_frame_number == r.range.start_frame_number {
                if region.range.end_frame_number < r.range.end_frame_number {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // ----RRRR-----------
                    r.range.start_frame_number = region.range.end_frame_number;
                    self.add_region(region);
                } else {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // ----RRRRRRRRRRRRRR-
                    *r = region;
                }
            } else if region.range.start_frame_number > r.range.start_frame_number {
                if region.range.end_frame_number < r.range.end_frame_number {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // ------RRRR---------
                    let mut behind_r = *r;
                    behind_r.range.start_frame_number = region.range.end_frame_number;
                    r.range.end_frame_number = region.range.start_frame_number;
                    self.add_region(behind_r);
                    self.add_region(region);
                } else {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // -----------RRRR---- or
                    // -------------RRRR--
                    r.range.end_frame_number = region.range.start_frame_number;
                    self.add_region(region);
                }
            } else {
                // Case: (r = `r`, R = `region`)
                // ----rrrrrrrrrrr----
                // --RRRR-------------
                r.range.start_frame_number = region.range.end_frame_number;
                self.add_region(region);
            }
            return;
        }
        panic!(
            "region {:x?} is not a usable memory region\n{:#?}",
            region, self
        );
    }
}

#[export_name = "_start_e820"]
pub unsafe extern "C" fn rust_start_820(hvm_start_info: *const HvmStartInfo) -> ! {
    //eprintln!("rust_start_820");
    let app_start_ptr = &_app_start_addr as *const _ as u64;
    let app_end_ptr = &_app_start_addr as *const _ as u64 + &_app_size as *const _ as u64;
    let kernel_start_ptr = &_kernel_start as *const _ as u64;
    let kernel_end_ptr = &_kernel_end as *const _ as u64;
    let pvh_stack_ptr = &pvh_stack as *const _ as u64;

    let e820_count = (*hvm_start_info).memmap_entries;
    let e820_table = core::slice::from_raw_parts_mut(
        (*hvm_start_info).memmap_paddr as *mut HvmMemmapTableEntry,
        e820_count as _,
    );
    //eprintln!("e820_count={}", e820_count);

    core::ptr::write(
        BOOTINFO_PHYS_ADDR as *mut BootInfo,
        BootInfo::new(MemoryMap::new()),
    );

    let boot_info: *mut BootInfo = BOOTINFO_PHYS_ADDR as _;

    //eprintln!("{:#?}", e820_table);

    for entry in e820_table {
        let end = entry.addr + entry.size;
        let start = entry.addr;
        match entry.get_type() {
            HvmMemmapTableEntryType::RAM => {
                (*boot_info).memory_map.add_region(MemoryRegion {
                    range: FrameRange::new(
                        PhysAddr::new(start).align_up(PAGESIZE as u64).as_u64(),
                        PhysAddr::new(end).align_down(PAGESIZE as u64).as_u64(),
                    ),
                    region_type: MemoryRegionType::Usable,
                });
            }
            /*
            HvmMemmapTableEntryType::Reserved => {
                (*boot_info).memory_map.add_region(MemoryRegion {
                    range: FrameRange::new(
                        PhysAddr::new(start).align_down(PAGESIZE as u64).as_u64(),
                        PhysAddr::new(end).align_up(PAGESIZE as u64).as_u64(),
                    ),
                    region_type: MemoryRegionType::Reserved,
                });
            }
            */
            _ => {}
        }
    }
    //eprintln!("{:#?}", (*boot_info).memory_map);

    (*boot_info).memory_map.mark_allocated_region(MemoryRegion {
        range: FrameRange::new(0, 0x1_0000),
        region_type: MemoryRegionType::Reserved,
    });
    (*boot_info).memory_map.mark_allocated_region(MemoryRegion {
        range: FrameRange::new(SYSCALL_PHYS_ADDR, SYSCALL_PHYS_ADDR + 0x1000),
        region_type: MemoryRegionType::SysCall,
    });
    (*boot_info).memory_map.mark_allocated_region(MemoryRegion {
        range: FrameRange::new(kernel_start_ptr, kernel_end_ptr),
        region_type: MemoryRegionType::Kernel,
    });
    (*boot_info).memory_map.mark_allocated_region(MemoryRegion {
        range: FrameRange::new(pvh_stack_ptr, pvh_stack_ptr + 0xF000),
        region_type: MemoryRegionType::KernelStack,
    });
    (*boot_info).memory_map.mark_allocated_region(MemoryRegion {
        range: FrameRange::new(
            app_start_ptr - PHYSICAL_MEMORY_OFFSET,
            app_end_ptr - PHYSICAL_MEMORY_OFFSET,
        ),
        region_type: MemoryRegionType::App,
    });

    _start(boot_info)
}
