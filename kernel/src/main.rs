#![no_std]
#![no_main]
#![warn(dead_code)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use boot::{entry_point, layout::*, BootInfo, MemoryRegionType};
use kernel::arch::x86_64::start::{kstart, KernelArgs};
use kernel::kmain;
use kernel::{print, println};

entry_point!(kernel_main);

extern "C" {
    static _app_start_addr: usize;
    static _app_size: usize;
}

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    println!("Hello World!!");

    unsafe {
        kstart(&KernelArgs {
            kernel_base: 0,
            kernel_size: 100 * 1024 * 1024,
            stack_base: BOOT_STACK_POINTER,
            stack_size: BOOT_STACK_POINTER_SIZE,
            env_base: 0,
            env_size: 0,
        })
    }
    //kmain(cpus: usize, env: &'static [u8])
}
