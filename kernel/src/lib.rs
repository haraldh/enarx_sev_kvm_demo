#![no_std]
#![cfg_attr(test, no_main)]
#![cfg_attr(feature = "nightly", feature(custom_test_frameworks))]
#![cfg_attr(feature = "nightly", feature(abi_x86_interrupt))]
#![cfg_attr(feature = "nightly", feature(alloc_error_handler))]
#![cfg_attr(feature = "nightly", test_runner(crate::test_runner))]
#![cfg_attr(feature = "nightly", feature(lang_items))]
#![cfg_attr(feature = "nightly", reexport_test_harness_main = "test_main")]
#![allow(clippy::empty_loop)]

#[cfg(not(feature = "nightly"))]
#[cfg(test)]
fn foo() {
    compile_error!("testing only on nightly");
}

#[cfg(feature = "allocator")]
extern crate alloc;

use core::panic::PanicInfo;

#[cfg(feature = "allocator")]
use linked_list_allocator::LockedHeap;

pub mod arch;
pub mod libc;
pub mod memory;
pub mod strlen;
pub mod syscall;

#[cfg(feature = "nightly")]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {
    exit_hypervisor(HyperVisorExitCode::Failed);
}

#[cfg(not(feature = "nightly"))]
#[no_mangle]
pub extern "C" fn rust_eh_personality() {
    exit_hypervisor(HyperVisorExitCode::Failed);
}

#[no_mangle]
extern "C" fn _Unwind_Resume() {
    exit_hypervisor(HyperVisorExitCode::Failed);
}

#[cfg(feature = "allocator")]
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

extern "C" {
    fn _context_switch(entry_point: extern "C" fn() -> !, stack_pointer: usize) -> !;
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

#[cfg(feature = "allocator")]
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
