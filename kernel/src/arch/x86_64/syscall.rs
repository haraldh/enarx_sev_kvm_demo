use super::gdt;
use crate::println;
use core::hint::unreachable_unchecked;
use x86_64::registers::control::EferFlags;
use x86_64::registers::model_specific::{Efer, KernelGsBase, Msr};
use x86_64::VirtAddr;

/// KernelGsBase Model Specific Register.
#[derive(Debug)]
pub struct Star;
impl Star {
    /// The underlying model specific register.
    pub const MSR: Msr = Msr::new(0xc0000081);
}

pub struct LStar;
impl LStar {
    /// The underlying model specific register.
    pub const MSR: Msr = Msr::new(0xc0000082);
}

pub struct FMask;
impl FMask {
    /// The underlying model specific register.
    pub const MSR: Msr = Msr::new(0xc0000084);
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
    LStar::MSR.write(syscall_instruction as u64);
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

#[repr(packed)]
pub struct ScratchRegisters {
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rax: usize,
}

impl ScratchRegisters {
    pub fn dump(&self) {
        println!("RAX:   {:>016X}", { self.rax });
        println!("RCX:   {:>016X}", { self.rcx });
        println!("RDX:   {:>016X}", { self.rdx });
        println!("RDI:   {:>016X}", { self.rdi });
        println!("RSI:   {:>016X}", { self.rsi });
        println!("R8:    {:>016X}", { self.r8 });
        println!("R9:    {:>016X}", { self.r9 });
        println!("R10:   {:>016X}", { self.r10 });
        println!("R11:   {:>016X}", { self.r11 });
    }
}

#[repr(packed)]
pub struct PreservedRegisters {
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub rbp: usize,
    pub rbx: usize,
}

impl PreservedRegisters {
    pub fn dump(&self) {
        println!("RBX:   {:>016X}", { self.rbx });
        println!("RBP:   {:>016X}", { self.rbp });
        println!("R12:   {:>016X}", { self.r12 });
        println!("R13:   {:>016X}", { self.r13 });
        println!("R14:   {:>016X}", { self.r14 });
        println!("R15:   {:>016X}", { self.r15 });
    }
}

#[repr(packed)]
pub struct IretRegisters {
    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,
    // Will only be present if interrupt is raised from another
    // privilege ring
    pub rsp: usize,
    pub ss: usize,
}

impl IretRegisters {
    pub fn dump(&self) {
        println!("RFLAG: {:>016X}", { self.rflags });
        println!("CS:    {:>016X}", { self.cs });
        println!("RIP:   {:>016X}", { self.rip });
        println!("RSP:   {:>016X}", { self.rsp });
        println!("SS:    {:>016X}", { self.ss });
    }
}

#[repr(packed)]
pub struct SyscallStack {
    pub fs: usize,
    pub preserved: PreservedRegisters,
    pub scratch: ScratchRegisters,
    pub iret: IretRegisters,
}

impl SyscallStack {
    pub fn dump(&self) {
        self.iret.dump();
        self.scratch.dump();
        self.preserved.dump();
        println!("FS:    {:>016X}", { self.fs });
    }
}

#[no_mangle]
pub unsafe extern "C" fn test_syscall_rust() -> usize {
    syscall_rust(1, 2, 3, 4, 5, 6, 7)
}

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
         mov fs, r15d         
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
         wrfsbase r11
         fninit
         pop rdi
         iretq"
         : // No output because it never returns
         :   "{r14}"((gdt::USER_DATA_SEG << 3) | 3), // Data segment
             "{r15}"((gdt::USER_TLS_SEG << 3) | 3) // TLS segment
         : // No clobbers because it never returns
         : "intel", "volatile");
    unreachable_unchecked()
}
