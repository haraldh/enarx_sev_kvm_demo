use super::gdt;
use core::hint::unreachable_unchecked;
use x86_64::registers::control::EferFlags;
use x86_64::registers::model_specific::{Efer, KernelGsBase, Msr};
use x86_64::VirtAddr;

/// KernelGsBase Model Specific Register.
#[derive(Debug)]
pub struct Star;
impl Star {
    /// The underlying model specific register.
    pub const MSR: Msr = Msr::new(0xc000_0081);
}

pub struct LStar;
impl LStar {
    /// The underlying model specific register.
    pub const MSR: Msr = Msr::new(0xc000_0082);
}

pub struct FMask;
impl FMask {
    /// The underlying model specific register.
    pub const MSR: Msr = Msr::new(0xc000_0084);
}

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
    LStar::MSR.write(syscall_instruction as usize as u64);
    FMask::MSR.write(0x300); // Clear trap flag and interrupt enable
    KernelGsBase::write(VirtAddr::new(gdt::TSS.as_ref().unwrap() as *const _ as u64));
    Efer::update(|f| {
        f.insert(
            EferFlags::LONG_MODE_ACTIVE
                | EferFlags::LONG_MODE_ENABLE
                | EferFlags::NO_EXECUTE_ENABLE
                | EferFlags::SYSTEM_CALL_EXTENSIONS,
        )
    });
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
    asm!("mov ds, r14d
         mov es, r14d
         mov gs, r14d
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
         mov fs, r11         
         wrfsbase r11
         fninit
         pop rdi
         iretq"
         : // No output because it never returns
         :   "{r14}"((gdt::USER_DATA_SEG << 3) | 3) // Data segment
         : // No clobbers because it never returns
         : "intel", "volatile");
    unreachable_unchecked()
}
