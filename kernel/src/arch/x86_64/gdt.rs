use x86_64::instructions::segmentation::{load_ds, load_es, load_fs, load_gs, load_ss};
use x86_64::structures::gdt::{
    Descriptor, DescriptorFlags, GlobalDescriptorTable, SegmentSelector,
};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

pub static mut TSS: Option<TaskStateSegment> = None;

pub static mut GDT: Option<(GlobalDescriptorTable, Selectors)> = None;

pub struct Selectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
    pub tls_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
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
            let tls_selector = gdt.add_entry(Descriptor::UserSegment(
                (DescriptorFlags::USER_SEGMENT
                    | DescriptorFlags::PRESENT
                    | DescriptorFlags::WRITABLE
                    | DescriptorFlags::LONG_MODE)
                    .bits(),
            ));
            let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
            let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
            let user_tls_selector = gdt.add_entry(Descriptor::user_data_segment());
            let tss_selector = gdt.add_entry(Descriptor::tss_segment(TSS.as_ref().unwrap()));
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
        load_ds(gdt.1.data_selector);
        load_es(gdt.1.data_selector);
        load_fs(gdt.1.data_selector);
        load_gs(gdt.1.data_selector);
        load_ss(gdt.1.data_selector);
        //load_tss(gdt.1.tss_selector);
    }
}
