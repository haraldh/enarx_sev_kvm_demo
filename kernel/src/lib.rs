#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(asm)]
#![feature(global_asm)]
#![feature(naked_functions)]
#![feature(thread_local)]
#![allow(clippy::empty_loop)]

extern crate alloc;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

pub mod arch;
pub mod libc;
pub mod memory;
pub mod strlen;
pub mod syscall;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub unsafe fn context_switch(entry_point: fn() -> !, stack_pointer: usize) -> ! {
    let entry_point: u64 = entry_point as usize as u64 + PHYSICAL_MEMORY_OFFSET;
    asm!("call $1; ${:private}.spin.${:uid}: jmp ${:private}.spin.${:uid}" ::
         "{rsp}"(stack_pointer), "r"(entry_point) :: "intel");
    ::core::hint::unreachable_unchecked()
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    exit_hypervisor(HyperVisorExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_hypervisor(HyperVisorExitCode::Failed);
    hlt_loop();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum HyperVisorExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_hypervisor(exit_code: HyperVisorExitCode) {
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
use vmbootspec::entry_point;
use vmbootspec::layout::PHYSICAL_MEMORY_OFFSET;

#[cfg(test)]
entry_point!(test_lib_main);

/// Entry point for `cargo xtest`
#[cfg(test)]
fn test_lib_main(boot_info: &'static mut vmbootspec::BootInfo) -> ! {
    use crate::arch::OffsetPageTable;
    use crate::memory::BootInfoFrameAllocator;

    fn inner(_mapper: &mut OffsetPageTable, _frame_allocator: &mut BootInfoFrameAllocator) -> ! // trigger a stack overflow
    {
        test_main();
        hlt_loop();
    }
    //println!("{}:{} test_lib_main", file!(), line!());

    crate::arch::init(boot_info, inner);
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
