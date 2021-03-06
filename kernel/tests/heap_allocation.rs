#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use core::panic::PanicInfo;
use kernel::arch::OffsetPageTable;
use kernel::memory::BootInfoFrameAllocator;
use kernel::{entry_point, serial_print, serial_println};
use vmsyscall::bootinfo::BootInfo;

entry_point!(main);

fn main(boot_info: &'static mut BootInfo) -> ! {
    fn inner(
        _mapper: &mut OffsetPageTable,
        _frame_allocator: &mut BootInfoFrameAllocator,
        _app_entry_point: *const u8,
        _app_load_addr: *const u8,
        _app_phnum: usize,
    ) -> ! // trigger a stack overflow
    {
        test_main();
        loop {}
    }
    kernel::arch::init(boot_info, inner)
}

#[test_case]
fn simple_allocation() {
    serial_print!("simple_allocation... ");
    let heap_value = Box::new(41);
    assert_eq!(*heap_value, 41);
    serial_println!("[ok]");
}

#[test_case]
fn large_vec() {
    serial_print!("large_vec... ");
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
    serial_println!("[ok]");
}

#[test_case]
fn many_boxes() {
    serial_print!("many_boxes... ");
    for i in 0..10_000 {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    serial_println!("[ok]");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}
