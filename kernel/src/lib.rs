#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(asm)]

extern crate alloc;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

pub mod allocator;
pub mod gdt;
pub mod interrupts;
pub mod libc;
pub mod memory;
pub mod serial;
pub mod sysconf;

/* test bsalloc
    pub mod bsalloc;
    pub mod alloc_fmt;
    pub mod mmap_alloc;
    pub mod object_alloc;
    use crate::bsalloc::BsAlloc;
    pub use alloc_fmt::*;
*/

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
//static ALLOCATOR: BsAlloc = BsAlloc;

use x86_64::structures::paging::OffsetPageTable;

pub static mut MAPPER: Option<OffsetPageTable> = None;

pub unsafe fn context_switch(entry_point: fn() -> !, stack_pointer: usize) -> ! {
    asm!("call $1; ${:private}.spin.${:uid}: jmp ${:private}.spin.${:uid}" ::
         "{rsp}"(stack_pointer), "r"(entry_point) :: "intel");
    ::core::hint::unreachable_unchecked()
}

pub fn init() {
    gdt::init();
    interrupts::init_idt();
    x86_64::instructions::interrupts::enable();
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::PortWriteOnly;

    unsafe {
        let mut port = PortWriteOnly::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
use boot::entry_point;

#[cfg(test)]
entry_point!(test_kernel_main);

/// Entry point for `cargo xtest`
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static mut boot::BootInfo) -> ! {
    init();
    test_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
