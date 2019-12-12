//! Hard coded memory layout

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
pub const BOOT_STACK_POINTER_SIZE: u64 = BOOT_STACK_POINTER - SYSCALL_PHYS_ADDR - 0x1000;
pub const PHYSICAL_MEMORY_OFFSET: u64 = 0x800_0000_0000;

pub const USER_STACK_SIZE: usize = 4 * 1024; // 1 MB
pub const USER_STACK_OFFSET: usize = 0x0000_0080_0000_0000 * 4;

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
