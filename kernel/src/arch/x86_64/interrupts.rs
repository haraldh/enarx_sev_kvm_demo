// The x86-interrupt calling convention leads to the following LLVM error
// when compiled for a Windows target: "offset is not a multiple of 16". This
// happens for example when running `cargo test` on Windows. To avoid this
// problem we skip compilation of this module on Windows.
#![cfg(not(windows))]

use super::gdt;
use crate::{exit_hypervisor, hlt_loop, println, HyperVisorExitCode};
use pic8259_simple::ChainedPics;
use spin;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

pub static mut IDT: Option<InterruptDescriptorTable> = None;

pub fn init_idt() {
    unsafe {
        IDT.replace({
            let mut idt = InterruptDescriptorTable::new();
            idt.stack_segment_fault
                .set_handler_fn(stack_segment_fault)
                .set_stack_index(1);
            idt.general_protection_fault
                .set_handler_fn(general_protection_fault)
                .set_stack_index(1);
            idt.breakpoint
                .set_handler_fn(breakpoint_handler)
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
            idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
            //idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
            idt
        });
        IDT.as_ref().unwrap().load();
    }
}

extern "x86-interrupt" fn stack_segment_fault(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    println!("stack_segment_fault {}", error_code);
    println!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn general_protection_fault(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    println!("general_protection_fault {}", error_code);
    println!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    println!("segment_not_present_handler {}", error_code);
    println!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn invalid_tss_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    println!("invalid_tss_handler {}", error_code);
    println!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT");
    println!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64, // Always 0
) -> ! {
    println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    println!(".");
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

#[cfg(test)]
#[test_case]
fn test_breakpoint_exception() {
    use crate::{serial_print, serial_println};
    serial_print!("test_breakpoint_exception...");
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
    serial_println!("[ok]");
}
