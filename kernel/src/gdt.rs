use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, DescriptorFlags, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::{PrivilegeLevel, VirtAddr};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    pub static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.privilege_stack_table[0] = {
            const STACK_SIZE: usize = 4096;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            stack_start + STACK_SIZE
        };
        tss.privilege_stack_table[2] = {
            const STACK_SIZE: usize = 4096;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            stack_start + STACK_SIZE
        };
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            stack_start + STACK_SIZE
        };
        tss
    };
}

#[derive(Debug, Clone)]
pub struct GlobalDescriptorTable {
    table: [u64; 16],
    next_free: usize,
}

impl GlobalDescriptorTable {
    /// Creates an empty GDT.
    pub fn new() -> GlobalDescriptorTable {
        GlobalDescriptorTable {
            table: [0; 16],
            next_free: 1,
        }
    }

    /// Adds the given segment descriptor to the GDT, returning the segment selector.
    ///
    /// Panics if the GDT has no free entries left.
    pub fn add_entry(&mut self, entry: Descriptor) -> SegmentSelector {
        let index = match entry {
            Descriptor::UserSegment(value) => self.push(value),
            Descriptor::SystemSegment(value_low, value_high) => {
                let index = self.push(value_low);
                self.push(value_high);
                index
            }
        };
        SegmentSelector::new(index as u16, PrivilegeLevel::Ring0)
    }

    /// Loads the GDT in the CPU using the `lgdt` instruction. This does **not** alter any of the
    /// segment registers; you **must** (re)load them yourself using [the appropriate
    /// functions](crate::instructions::segmentation):
    /// [load_ss](crate::instructions::segmentation::load_ss),
    /// [set_cs](crate::instructions::segmentation::set_cs).
    #[cfg(target_arch = "x86_64")]
    pub fn load(&'static self) {
        use core::mem::size_of;
        use x86_64::instructions::tables::{lgdt, DescriptorTablePointer};

        let ptr = DescriptorTablePointer {
            base: self.table.as_ptr() as u64,
            limit: (self.table.len() * size_of::<u64>() - 1) as u16,
        };

        unsafe { lgdt(&ptr) };
    }

    fn push(&mut self, value: u64) -> usize {
        if self.next_free < self.table.len() {
            let index = self.next_free;
            self.table[index] = value;
            self.next_free += 1;
            index
        } else {
            panic!("GDT full");
        }
    }
}

lazy_static! {
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::UserSegment(
            (DescriptorFlags::USER_SEGMENT | DescriptorFlags::PRESENT | DescriptorFlags::LONG_MODE)
                .bits(),
        ));
        let tls_selector = gdt.add_entry(Descriptor::UserSegment(
            (DescriptorFlags::USER_SEGMENT | DescriptorFlags::PRESENT | DescriptorFlags::LONG_MODE)
                .bits(),
        ));
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        let user_tls_selector = gdt.add_entry(Descriptor::user_data_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                data_selector,
                tls_selector,
                user_code_selector,
                user_data_selector,
                user_tls_selector,
                tss_selector,
            },
        )
    };
}

pub struct Selectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
    pub tls_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub user_tls_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::segmentation::{self, set_cs};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe {
        set_cs(GDT.1.code_selector);
        segmentation::load_ds(GDT.1.data_selector);
        segmentation::load_es(GDT.1.data_selector);
        segmentation::load_fs(GDT.1.tls_selector);
        segmentation::load_gs(GDT.1.data_selector);
        //segmentation::load_ss(GDT.1.data_selector);
        load_tss(GDT.1.tss_selector);
    }
}
