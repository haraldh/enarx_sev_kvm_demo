use crate::println;
use core::ops::{Deref, DerefMut};
use core::{mem, slice};

use crate::gdt;
use crate::pti;
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

pub unsafe fn init() {
    Star::MSR.write(((gdt::GDT.1.code_selector.index() as u64) << 3) << 32);
    LStar::MSR.write(syscall_instruction as u64);
    FMask::MSR.write(0x300); // Clear trap flag and interrupt enable
    KernelGsBase::write(VirtAddr::new(&gdt::TSS as *const _ as u64));
    Efer::write(Efer::read() | EferFlags::SYSTEM_CALL_EXTENSIONS);
}

#[allow(dead_code)]
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

macro_rules! scratch_push {
    () => (asm!(
        "push rax
        push rcx
        push rdx
        push rdi
        push rsi
        push r8
        push r9
        push r10
        push r11"
        : : : : "intel", "volatile"
    ));
}

macro_rules! scratch_pop {
    () => (asm!(
        "pop r11
        pop r10
        pop r9
        pop r8
        pop rsi
        pop rdi
        pop rdx
        pop rcx
        pop rax"
        : : : : "intel", "volatile"
    ));
}

#[allow(dead_code)]
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

macro_rules! preserved_push {
    () => (asm!(
        "push rbx
        push rbp
        push r12
        push r13
        push r14
        push r15"
        : : : : "intel", "volatile"
    ));
}

macro_rules! preserved_pop {
    () => (asm!(
        "pop r15
        pop r14
        pop r13
        pop r12
        pop rbp
        pop rbx"
        : : : : "intel", "volatile"
    ));
}
#[allow(dead_code)]
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
    }
}

// Not a function pointer because it somehow messes up the returning
// from clone() (via clone_ret()). Not sure what the problem is.
macro_rules! with_interrupt_stack {
    (unsafe fn $wrapped:ident($stack:ident) -> usize $code:block) => {
        #[inline(never)]
        unsafe fn $wrapped(stack: *mut InterruptStack) {
            // If syscall not ignored
            let $stack = &mut *stack;
            $stack.scratch.rax = $code;
        }
    };
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
#[cfg(target_arch = "x86_64")]
pub struct IntRegisters {
    // TODO: Some of these don't get set by Redox yet. Should they?
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub rbp: usize,
    pub rbx: usize,
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rax: usize,
    pub rcx: usize,
    pub rdx: usize,
    pub rsi: usize,
    pub rdi: usize,
    // pub orig_rax: usize,
    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,
    pub rsp: usize,
    pub ss: usize,
    // pub fs_base: usize,
    // pub gs_base: usize,
    // pub ds: usize,
    // pub es: usize,
    pub fs: usize,
    // pub gs: usize
}

impl Deref for IntRegisters {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self as *const IntRegisters as *const u8,
                mem::size_of::<IntRegisters>(),
            )
        }
    }
}

impl DerefMut for IntRegisters {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                self as *mut IntRegisters as *mut u8,
                mem::size_of::<IntRegisters>(),
            )
        }
    }
}

#[allow(dead_code)]
#[repr(packed)]
pub struct InterruptStack {
    pub fs: usize,
    pub preserved: PreservedRegisters,
    pub scratch: ScratchRegisters,
    pub iret: IretRegisters,
}

impl InterruptStack {
    pub fn dump(&self) {
        self.iret.dump();
        self.scratch.dump();
        self.preserved.dump();
        println!("FS:    {:>016X}", { self.fs });
    }
    /// Saves all registers to a struct used by the proc:
    /// scheme to read/write registers.
    pub fn save(&self, all: &mut IntRegisters) {
        all.fs = self.fs;

        all.r15 = self.preserved.r15;
        all.r14 = self.preserved.r14;
        all.r13 = self.preserved.r13;
        all.r12 = self.preserved.r12;
        all.rbp = self.preserved.rbp;
        all.rbx = self.preserved.rbx;
        all.r11 = self.scratch.r11;
        all.r10 = self.scratch.r10;
        all.r9 = self.scratch.r9;
        all.r8 = self.scratch.r8;
        all.rsi = self.scratch.rsi;
        all.rdi = self.scratch.rdi;
        all.rdx = self.scratch.rdx;
        all.rcx = self.scratch.rcx;
        all.rax = self.scratch.rax;
        all.rip = self.iret.rip;
        all.cs = self.iret.cs;
        all.rflags = self.iret.rflags;

        // Set rsp and ss:

        const CPL_MASK: usize = 0b11;

        let cs: usize;
        unsafe {
            asm!("mov $0, cs" : "=r"(cs) ::: "intel");
        }

        if self.iret.cs & CPL_MASK == cs & CPL_MASK {
            // Privilege ring didn't change, so neither did the stack
            all.rsp = self as *const Self as usize // rsp after Self was pushed to the stack
                + mem::size_of::<Self>() // disregard Self
                - mem::size_of::<usize>() * 2; // well, almost: rsp and ss need to be excluded as they aren't present
            unsafe {
                asm!("mov $0, ss" : "=r"(all.ss) ::: "intel");
            }
        } else {
            all.rsp = self.iret.rsp;
            all.ss = self.iret.ss;
        }
    }
    /// Loads all registers from a struct used by the proc:
    /// scheme to read/write registers.
    pub fn load(&mut self, all: &IntRegisters) {
        // TODO: Which of these should be allowed to change?

        // self.fs = all.fs;
        self.preserved.r15 = all.r15;
        self.preserved.r14 = all.r14;
        self.preserved.r13 = all.r13;
        self.preserved.r12 = all.r12;
        self.preserved.rbp = all.rbp;
        self.preserved.rbx = all.rbx;
        self.scratch.r11 = all.r11;
        self.scratch.r10 = all.r10;
        self.scratch.r9 = all.r9;
        self.scratch.r8 = all.r8;
        self.scratch.rsi = all.rsi;
        self.scratch.rdi = all.rdi;
        self.scratch.rdx = all.rdx;
        self.scratch.rcx = all.rcx;
        self.scratch.rax = all.rax;
        self.iret.rip = all.rip;

        // These should probably be restricted
        // self.iret.cs = all.cs;
        // self.iret.rflags = all.eflags;
    }
    /// Enables the "Trap Flag" in the FLAGS register, causing the CPU
    /// to send a Debug exception after the next instruction. This is
    /// used for singlestep in the proc: scheme.
    pub fn set_singlestep(&mut self, enabled: bool) {
        if enabled {
            self.iret.rflags |= 1 << 8;
        } else {
            self.iret.rflags &= !(1 << 8);
        }
    }
}

pub fn syscall(
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
    bp: usize,
    stack: &mut InterruptStack,
) -> usize {
    println!("syscall a={}", a);
    println!("syscall b={}", b);
    println!("syscall c={}", c);
    println!("syscall d={}", d);
    println!("syscall e={}", e);
    println!("syscall f={}", f);
    println!("syscall bp={}", bp);
    println!("syscall stack={:#X}", stack as *const _ as u64);
    38 // ENOSYS
}

#[naked]
pub unsafe extern "C" fn syscall_instruction() {
    with_interrupt_stack! {
        unsafe fn inner(stack) -> usize {
            let rbp;
            asm!("" : "={rbp}"(rbp) : : : "intel", "volatile");

            let scratch = &stack.scratch;
            syscall(scratch.rax, scratch.rdi, scratch.rsi, scratch.rdx, scratch.r10, scratch.r8, rbp, stack)
        }
    }

    // Yes, this is magic. No, you don't need to understand
    asm!("
          swapgs                    // Set gs segment to TSS
          mov gs:[28], rsp          // Save userspace rsp
          mov rsp, gs:[4]           // Load kernel rsp
          push 5 * 8 + 3            // Push userspace data segment
          push qword ptr gs:[28]    // Push userspace rsp
          mov qword ptr gs:[28], 0  // Clear userspace rsp
          push r11                  // Push rflags
          push 4 * 8 + 3            // Push userspace code segment
          push rcx                  // Push userspace return pointer
          swapgs                    // Restore gs
          "
          :
          :
          :
          : "intel", "volatile");

    // Push scratch registers
    scratch_push!();
    preserved_push!();
    asm!("push fs
         mov r11, 0x18
         mov fs, r11"
         : : : : "intel", "volatile");

    // Get reference to stack variables
    let rsp: usize;
    asm!("" : "={rsp}"(rsp) : : : "intel", "volatile");

    // Map kernel
    pti::map();

    inner(rsp as *mut InterruptStack);

    // Unmap kernel
    pti::unmap();

    // Interrupt return
    asm!("pop fs" : : : : "intel", "volatile");
    preserved_pop!();
    scratch_pop!();
    asm!("iretq" : : : : "intel", "volatile");
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
              :   "{r10}"(gdt::GDT.1.user_data_selector.index() << 3 | 3), // Data segment
                  "{r11}"(sp), // Stack pointer
                  "{r12}"(1 << 9), // Flags - Set interrupt enable flag
                  "{r13}"(gdt::GDT.1.user_code_selector.index() << 3 | 3), // Code segment
                  "{r14}"(ip), // IP
                  "{r15}"(arg) // Argument
              : // No clobbers
              : "intel", "volatile");

    // Unmap kernel
    pti::unmap();

    // Go to usermode
    asm!("mov ds, r14d
         mov es, r14d
         mov fs, r15d
         mov gs, r14d
         xor rax, rax
         xor rbx, rbx
         xor rcx, rcx
         xor rdx, rdx
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
         fninit
         pop rdi
         iretq"
         : // No output because it never returns
         :   "{r14}"(gdt::GDT.1.user_data_selector.index() << 3 | 3), // Data segment
             "{r15}"(gdt::GDT.1.user_tls_selector.index() << 3 | 3) // TLS segment
         : // No clobbers because it never returns
         : "intel", "volatile");
    unreachable!();
}
