#![no_std]
#![no_main]
#![warn(dead_code)]
#![cfg_attr(any(feature = "nightly", test), feature(custom_test_frameworks))]
#![cfg_attr(any(feature = "nightly", test), test_runner(kernel::test_runner))]
#![cfg_attr(
    any(feature = "nightly", test),
    reexport_test_harness_main = "test_main"
)]
#![allow(clippy::empty_loop)]

use core::panic::PanicInfo;
use kernel::arch::OffsetPageTable;
use kernel::memory::BootInfoFrameAllocator;
use kernel::{entry_point, exit_hypervisor, println, HyperVisorExitCode};
use vmsyscall::bootinfo::BootInfo;

entry_point!(kernel_main);

#[cfg(not(test))]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    fn with_stack_protection(
        mapper: &mut OffsetPageTable,
        frame_allocator: &mut BootInfoFrameAllocator,
        app_entry_point: *const u8,
        app_load_addr: *const u8,
        app_phnum: usize,
    ) -> ! {
        kernel::arch::exec_elf(
            mapper,
            frame_allocator,
            app_entry_point,
            app_load_addr,
            app_phnum,
        );
    }
    kernel::arch::init(boot_info, with_stack_protection)
}

#[cfg(test)]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    fn inner(
        _mapper: &mut OffsetPageTable,
        _frame_allocator: &mut BootInfoFrameAllocator,
        _app_entry_point: *const u8,
        _app_load_addr: *const u8,
        _app_phnum: usize,
    ) -> ! {
        test_main();
        println!("It did not crash!");
        exit_hypervisor(HyperVisorExitCode::Success);
        kernel::hlt_loop()
    }

    kernel::arch::init(boot_info, inner)
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    exit_hypervisor(HyperVisorExitCode::Failed);
    kernel::hlt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}
