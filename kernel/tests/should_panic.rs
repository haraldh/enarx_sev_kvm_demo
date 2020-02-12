#![no_std]
#![no_main]

use core::panic::PanicInfo;
use kernel::{exit_hypervisor, serial_print, serial_println, HyperVisorExitCode};

#[no_mangle]
pub extern "C" fn _start_main() -> ! {
    should_fail();
    serial_println!("[test did not panic]");
    exit_hypervisor(HyperVisorExitCode::Failed);
    loop {}
}

fn should_fail() {
    serial_print!("should_fail... ");
    assert_eq!(0, 1);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_hypervisor(HyperVisorExitCode::Success);
    loop {}
}
