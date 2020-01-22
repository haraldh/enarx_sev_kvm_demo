use vmbootspec::layout::{BOOTINFO_PHYS_ADDR, HIMEM_START};
use vmbootspec::{
    layout::{PHYSICAL_MEMORY_OFFSET, PML4_START},
    BootInfo, FrameRange, MemoryRegion, MemoryRegionType,
};

#[inline(always)]
pub fn read_ebx() -> u64 {
    let val: u64;
    unsafe {
        asm!(
            "mov $0, rbx"
            : "=r"(val) ::: "intel", "volatile"
        );
    }
    val
}

#[export_name = "_start_e820"]
pub extern "C" fn __impl_start_820() -> ! {
    unsafe { rust_start_820(read_ebx() as *mut HvmStartInfo) };
}

extern "C" {
    fn _start(bootinfo: &mut BootInfo) -> !;
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

#[repr(C, packed)]
pub struct HvmMemmapTableEntry {
    addr: u64,       /* Base address of the memory region         */
    size: u64,       /* Size of the memory region in bytes        */
    entry_type: u32, /* Mapping type                              */
    reserved: u32,   /* Must be zero for Version 1.               */
}

const TYPE_RAM: u32 = 1;

unsafe fn rust_start_820(hvm_start_info: *mut HvmStartInfo) -> ! {
    eprintln!("rust_start_820");

    let e820_count = (*hvm_start_info).memmap_entries;
    let e820_table = core::slice::from_raw_parts_mut(
        (*hvm_start_info).memmap_paddr as *mut HvmMemmapTableEntry,
        e820_count as _,
    );
    eprintln!("e820_count={}", e820_count);

    let boot_info_addr: *mut BootInfo = BOOTINFO_PHYS_ADDR as _;
    core::ptr::write(boot_info_addr, core::mem::zeroed());

    let boot_info: &mut BootInfo = &mut *boot_info_addr;

    boot_info.physical_memory_offset = PHYSICAL_MEMORY_OFFSET;
    boot_info.recursive_page_table_addr = PML4_START as _;

    boot_info.memory_map.add_region(MemoryRegion {
        range: FrameRange::new(0, (HIMEM_START as u64) * 2),
        region_type: MemoryRegionType::Reserved,
    });

    for entry in e820_table {
        if entry.entry_type == TYPE_RAM {
            if entry.addr < (HIMEM_START as u64) {
                continue;
            }
            let end = entry.addr + entry.size;
            let mut start = entry.addr;

            if start < (HIMEM_START as u64) {
                continue;
            }

            if end > ((HIMEM_START as u64) * 2) {
                start = (HIMEM_START as u64) * 2;
            }
            eprintln!("RAM found");
            boot_info.memory_map.add_region(MemoryRegion {
                range: FrameRange::new(start, end),
                region_type: MemoryRegionType::Usable,
            });
        }
    }

    _start(boot_info)
}
