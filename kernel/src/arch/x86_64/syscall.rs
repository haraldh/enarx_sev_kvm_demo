use super::gdt;
use x86_64::registers::model_specific::{KernelGsBase, LStar, SFMask, Star};
use x86_64::registers::rflags::RFlags;
use x86_64::VirtAddr;

extern "C" {
    fn _syscall_enter() -> !;
    fn _usermode(ip: usize, sp: usize, arg: usize) -> !;
}

pub unsafe fn init() {
    // FIXME: might (not) want to use sysret someday for performance
    Star::write(
        gdt::GDT.as_ref().unwrap().1.user_code_selector,
        gdt::GDT.as_ref().unwrap().1.user_data_selector,
        gdt::GDT.as_ref().unwrap().1.code_selector,
        gdt::GDT.as_ref().unwrap().1.data_selector,
    )
    .unwrap();

    LStar::write(VirtAddr::new(_syscall_enter as usize as u64));

    // Clear trap flag and interrupt enable
    SFMask::write(RFlags::INTERRUPT_FLAG | RFlags::TRAP_FLAG);

    KernelGsBase::write(VirtAddr::new(gdt::TSS.as_ref().unwrap() as *const _ as u64));
}

#[allow(clippy::many_single_char_names)]
#[no_mangle]
pub unsafe extern "C" fn syscall_rust(
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
    nr: usize,
) -> usize {
    crate::syscall::handle_syscall(a, b, c, d, e, f, nr)
}

#[inline(always)]
pub unsafe fn usermode(ip: usize, sp: usize, arg: usize) -> ! {
    _usermode(ip, sp, arg)
}
