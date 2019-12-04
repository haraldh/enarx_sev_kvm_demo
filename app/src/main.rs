#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(global_asm)]

use core::panic::PanicInfo;

global_asm!(include_str!("asm.s"));

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
