//! Global Descriptor Table init

use x86_64::instructions::segmentation::{load_ds, load_es, load_fs, load_gs, load_ss};
use x86_64::structures::gdt::GlobalDescriptorTable;
use x86_64::structures::gdt::{Descriptor, DescriptorFlags, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

pub static mut TSS: Option<TaskStateSegment> = None;
pub static mut GDT: Option<(GlobalDescriptorTable, Selectors)> = None;

pub const KERNEL_CODE_SEG: u16 = 1;
pub const KERNEL_DATA_SEG: u16 = 2;
pub const KERNEL_TLS_SEG: u16 = 2;
pub const USER_DATA_SEG: u16 = 3;
pub const USER_CODE_SEG: u16 = 4;
pub const USER_TLS_SEG: u16 = 5;
pub const TSS_SEG: u16 = 6;

#[cfg(test)]
#[test_case]
fn test_segment_index() {
    use crate::{serial_print, serial_println};
    serial_print!("test_segment_index...");
    let gdt_sel = unsafe { &GDT.as_ref().unwrap().1 };
    assert_eq!(
        KERNEL_CODE_SEG,
        gdt_sel.code_selector.index(),
        "KERNEL_CODE_SEG"
    );
    assert_eq!(
        KERNEL_DATA_SEG,
        gdt_sel.data_selector.index(),
        "KERNEL_DATA_SEG"
    );
    assert_eq!(
        USER_CODE_SEG,
        gdt_sel.user_code_selector.index(),
        "USER_CODE_SEG"
    );
    assert_eq!(
        USER_DATA_SEG,
        gdt_sel.user_data_selector.index(),
        "USER_DATA_SEG"
    );

    // sysret loads segments from STAR MSR assuming USER_CODE_SEG follows USER_DATA_SEG
    assert_eq!(
        USER_DATA_SEG + 1,
        USER_CODE_SEG,
        "USER_DATA_SEG + 1 == USER_CODE_SEG"
    );
    assert_eq!(TSS_SEG, gdt_sel.tss_selector.index());
    serial_println!("[ok]");
}

pub struct Selectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
    //pub tls_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_tls_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::segmentation::set_cs;

    unsafe {
        TSS = Some({
            let mut tss = TaskStateSegment::new();

            tss.privilege_stack_table[0] = {
                const STACK_SIZE: usize = 4096 * 2;
                static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

                let stack_start = VirtAddr::from_ptr(&STACK);
                stack_start + STACK_SIZE
            };
            tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
                const STACK_SIZE: usize = 4096 * 2;
                static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

                let stack_start = VirtAddr::from_ptr(&STACK);
                stack_start + STACK_SIZE
            };
            tss.interrupt_stack_table[1usize] = {
                const STACK_SIZE: usize = 4096 * 2;
                static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

                let stack_start = VirtAddr::from_ptr(&STACK);
                stack_start + STACK_SIZE
            };
            tss
        });
    }

    unsafe {
        GDT = Some({
            let mut gdt = GlobalDescriptorTable::new();
            let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
            let data_selector = gdt.add_entry(Descriptor::UserSegment(
                (DescriptorFlags::USER_SEGMENT
                    | DescriptorFlags::PRESENT
                    | DescriptorFlags::WRITABLE
                    | DescriptorFlags::LONG_MODE)
                    .bits(),
            ));
            /*
            let tls_selector = gdt.add_entry(Descriptor::UserSegment(
                (DescriptorFlags::USER_SEGMENT
                    | DescriptorFlags::PRESENT
                    | DescriptorFlags::WRITABLE
                    | DescriptorFlags::LONG_MODE)
                    .bits(),
            ));
            */
            let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
            let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
            let user_tls_selector = gdt.add_entry(Descriptor::user_data_segment());
            let tss_selector = gdt.add_entry(Descriptor::tss_segment(TSS.as_ref().unwrap()));
            (
                gdt,
                Selectors {
                    code_selector,
                    data_selector,
                    //tls_selector,
                    user_data_selector,
                    user_code_selector,
                    user_tls_selector,
                    tss_selector,
                },
            )
        });
    }

    let gdt = unsafe { GDT.as_ref().unwrap() };
    unsafe {
        asm!("
            mov ax, 0
            mov ss, ax
            mov ds, ax
            mov es, ax
            mov fs, ax
            mov gs, ax"
         : : : : "intel", "volatile");
    }
    gdt.0.load();
    unsafe {
        set_cs(gdt.1.code_selector);
        load_ss(gdt.1.data_selector);

        /*
            load_ds(gdt.1.data_selector);
            load_es(gdt.1.data_selector);
            load_fs(gdt.1.tls_selector);
            load_fs(gdt.1.data_selector);
            load_gs(gdt.1.data_selector);
        */
        load_ds(SegmentSelector(0));
        load_es(SegmentSelector(0));
        load_fs(SegmentSelector(0));
        load_gs(SegmentSelector(0));

        // Is done later with the real kernel stack
        // load_tss(gdt.1.tss_selector);
    }
}
