use super::gdt;
use core::hint::unreachable_unchecked;
use x86_64::registers::model_specific::{KernelGsBase, LStar, SFMask, Star};
use x86_64::registers::rflags::RFlags;
use x86_64::VirtAddr;

extern "C" {
    fn syscall_instruction() -> !;
}

pub unsafe fn init() {
    Star::MSR.write(
        (((gdt::GDT.as_ref().unwrap().1.code_selector.index() as u64) << 3) << 32)
            // FIXME: might (not) want to use sysret someday for performance
            | ((((gdt::GDT.as_ref().unwrap().1.user_data_selector.index() as u64 - 1) << 3) | 3)
                << 48),
    );
    LStar::write(VirtAddr::new(syscall_instruction as usize as u64));
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

#[naked]
pub unsafe fn usermode(ip: usize, sp: usize, arg: usize) -> ! {
    asm!("push r10
          push r11
          push r12
          push r13
          push r14
          push r15"
          : // No output
          :   "{r10}"((gdt::USER_DATA_SEG << 3) | 3), // Data segment
              "{r11}"(sp), // Stack pointer
              "{r12}"(1 << 9), // Flags - Set interrupt enable flag
              "{r13}"((gdt::USER_CODE_SEG << 3) | 3), // Code segment
              "{r14}"(ip), // IP
              "{r15}"(arg) // Argument
          : // No clobbers
          : "intel", "volatile");

    // Go to usermode
    asm!("
         xor rax, rax
         xor rdx, rdx
         xor rbx, rbx
         xor rcx, rcx
         xor rsi, rsi
         xor rdi, rdi
         xor rbp, rbp
         xor r8, r8
         xor r9, r9
         xor r10, r10
         xor r11, r11
         xor r12, r12
         xor r13, r13
         xor r14, r14
         xor r15, r15
         mov ds, r11
         mov es, r11
         mov fs, r11
         mov gs, r11
         wrfsbase r11
         wrgsbase r11
         fninit
         pop rdi
         iretq"
         : // No output because it never returns
         :
         : // No clobbers because it never returns
         : "intel", "volatile");
    unreachable_unchecked()
}
