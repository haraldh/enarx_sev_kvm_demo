use super::gdt;
use crate::{eprintln, exit_hypervisor, hlt_loop, HyperVisorExitCode};

#[cfg(feature = "timer")]
use crate::arch::x86_64::timer::{timer_init, timer_set_idt};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub static mut IDT: Option<InterruptDescriptorTable> = None;

pub fn init() {
    #[cfg(debug_assertions)]
    eprintln!("interrupts::init");

    unsafe {
        IDT.replace({
            let mut idt = InterruptDescriptorTable::new();
            idt.divide_error
                .set_handler_fn(divide_error_handler)
                .set_stack_index(1);
            idt.debug.set_handler_fn(debug_handler).set_stack_index(1);
            idt.overflow
                .set_handler_fn(overflow_handler)
                .set_stack_index(1);
            idt.bound_range_exceeded
                .set_handler_fn(bound_range_exceeded_handler)
                .set_stack_index(1);
            idt.device_not_available
                .set_handler_fn(device_not_available_handler)
                .set_stack_index(1);
            idt.x87_floating_point
                .set_handler_fn(x87_floating_point_handler)
                .set_stack_index(1);
            idt.alignment_check
                .set_handler_fn(alignment_check_handler)
                .set_stack_index(1);
            idt.machine_check
                .set_handler_fn(machine_check_handler)
                .set_stack_index(1);
            idt.simd_floating_point
                .set_handler_fn(simd_floating_point_handler)
                .set_stack_index(1);
            idt.virtualization
                .set_handler_fn(virtualization_handler)
                .set_stack_index(1);
            idt.security_exception
                .set_handler_fn(security_exception_handler)
                .set_stack_index(1);

            idt.non_maskable_interrupt
                .set_handler_fn(non_maskable_interrupt_handler)
                .set_stack_index(1);
            idt.breakpoint
                .set_handler_fn(breakpoint_handler)
                .set_stack_index(1);
            idt.stack_segment_fault
                .set_handler_fn(stack_segment_fault)
                .set_stack_index(1);
            idt.general_protection_fault
                .set_handler_fn(general_protection_fault)
                .set_stack_index(1);
            idt.invalid_opcode
                .set_handler_fn(invalid_opcode_handler)
                .set_stack_index(1);
            idt.invalid_tss
                .set_handler_fn(invalid_tss_handler)
                .set_stack_index(1);
            idt.segment_not_present
                .set_handler_fn(segment_not_present_handler)
                .set_stack_index(1);
            idt.page_fault
                .set_handler_fn(page_fault_handler)
                .set_stack_index(1);
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);

            for i in 32..256 {
                idt[i].set_handler_fn(unknown_interrupt_handler);
            }
            #[cfg(feature = "timer")]
            timer_set_idt(&mut idt);
            idt
        });
        IDT.as_ref().unwrap().load();
    }

    #[cfg(feature = "timer")]
    timer_init();

    x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn stack_segment_fault(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    eprintln!("stack_segment_fault {}", error_code);
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn general_protection_fault(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    eprintln!("general_protection_fault {:#b}", error_code);
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    eprintln!("segment_not_present_handler {}", error_code);
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("invalid_opcode_handler");
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("divide_error_handler");
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn debug_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("debug_handler");
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn overflow_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("overflow_handler");
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("bound_range_exceeded_handler");
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("device_not_available_handler");
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn x87_floating_point_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("x87_floating_point_handler");
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    eprintln!("alignment_check_handler");
    eprintln!("Error Code: {:?}", error_code);
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn machine_check_handler(stack_frame: &mut InterruptStackFrame) -> ! {
    eprintln!("machine_check_handler");
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("simd_floating_point_handler");
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn virtualization_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("virtualization_handler");
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn security_exception_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    eprintln!("security_exception_handler");
    eprintln!("Error Code: {:?}", error_code);
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn invalid_tss_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    eprintln!("invalid_tss_handler {}", error_code);
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("EXCEPTION: BREAKPOINT");
    eprintln!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn non_maskable_interrupt_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("EXCEPTION: NMI");
    eprintln!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    eprintln!("EXCEPTION: PAGE FAULT");
    eprintln!("Error Code: {:?}", error_code);
    eprintln!("{:#?}", stack_frame);
    eprintln!("Accessed Address: {:?}", Cr2::read());
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64, // Always 0
) -> ! {
    eprintln!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn unknown_interrupt_handler(stack_frame: &mut InterruptStackFrame) {
    eprintln!("EXCEPTION: unknown interrupt");
    eprintln!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

#[cfg(test)]
#[test_case]
fn test_breakpoint_exception() {
    use crate::{print, println};
    print!("test_breakpoint_exception...");
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
    println!("[ok]");
}
