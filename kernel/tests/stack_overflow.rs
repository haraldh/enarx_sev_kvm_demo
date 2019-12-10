#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use boot::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kernel::memory::BootInfoFrameAllocator;
use kernel::{exit_hypervisor, serial_println, HyperVisorExitCode};
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use x86_64::structures::paging::OffsetPageTable;

entry_point!(main);

fn main(boot_info: &'static mut BootInfo) -> ! {
    fn inner(_mapper: &mut OffsetPageTable, _frame_allocator: &mut BootInfoFrameAllocator) -> ! // trigger a stack overflow
    {
        init_test_idt();
        stack_overflow();

        panic!("Execution continued after stack overflow");
    }

    kernel::init(boot_info, inner)
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow(); // for each recursion, the return address is pushed
}

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(test_double_fault_handler)
                .set_stack_index(kernel::arch::x86_64::gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}

extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    serial_println!("[ok]");
    exit_hypervisor(HyperVisorExitCode::Success);
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}
