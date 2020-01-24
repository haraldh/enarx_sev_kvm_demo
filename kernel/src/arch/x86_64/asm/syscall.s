.section .text, "ax"
.global syscall_instruction
.code64

syscall_instruction:
    cli
    swapgs                 // Set gs segment to TSS
    mov    %rsp,%gs:0x1c   // Save userspace rsp
    mov    %gs:0x4,%rsp    // Load kernel rsp
    pushq  $0x1b           // Push userspace data segment  ((gdt::USER_DATA_SEG << 3) | 3)
    pushq  %gs:0x1c        // Push userspace rsp
    movq   $0x0,%gs:0x1c   // Clear userspace rs
    push   %r11            // Push rflags stored in r11
    pushq  $0x23           // Push userspace code segment  ((gdt::USER_CODE_SEG << 3) | 3)
    push   %rcx            // Push userspace return pointer
    swapgs                 // Restore gs
    sti

    // SYSV:    rdi, rsi, rdx, rcx, r8, r9
    // SYSCALL: rdi, rsi, rdx, r10, r8, r9
    mov    %r10, %rcx
    push   %rdi
    push   %rsi
    push   %rdx
    push   %r10
    push   %r8
    push   %r9
    push   %rax
    callq  syscall_rust
    pop    %rcx
    pop    %r9
    pop    %r8
    pop    %r10
    pop    %rdx
    pop    %rsi
    pop    %rdi

    // FIXME: want to protect the kernel against userspace?
    // https://www.kernel.org/doc/Documentation/x86/entry_64.txt
    // use:
    iretq

    // FIXME: comment out iretq for fast return with sysretq
    cli
    swapgs
    pop    %rcx             // Pop userspace return pointer
    add    $0x8,%rsp        // Pop userspace code segment
    pop    %r11             // pop rflags to r11
    popq   %gs:0x1c         // Pop userspace rsp
    mov    %gs:0x1c,%rsp    // Restore userspace rsp
    swapgs
    sti
    sysretq
