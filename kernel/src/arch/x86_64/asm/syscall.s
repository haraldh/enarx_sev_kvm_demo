.section .text, "ax"
.global syscall_instruction
.code64

syscall_instruction:
    swapgs                 // Set gs segment to TSS
    mov    %rsp,%gs:0x1c   // Save userspace rsp
    mov    %gs:0x4,%rsp    // Load kernel rsp
    pushq  $0x1b           // Push userspace data segment
    pushq  %gs:0x1c        // Push userspace rsp
    movq   $0x0,%gs:0x1c   // Clear userspace rs
    push   %r11            // Push rflags
    pushq  $0x23           // Push userspace code segment
    push   %rcx            // Push userspace return pointer
    swapgs                 // Restore gs
    push   %rax
    push   %rcx
    push   %rdx
    push   %rdi
    push   %rsi
    push   %r8
    push   %r9
    push   %r10
    push   %r11
    push   %rbx
    push   %rbp
    push   %r12
    push   %r13
    push   %r14
    push   %r15
    rdfsbase %r11
    push   %r11
    mov    $0x0,%r11
    mov    %r11,%fs
    mov    %rsp,%rdi
    callq  syscall_rust
    pop    %r11
    wrfsbase %r11
    pop    %r15
    pop    %r14
    pop    %r13
    pop    %r12
    pop    %rbp
    pop    %rbx
    pop    %r11
    pop    %r10
    pop    %r9
    pop    %r8
    pop    %rsi
    pop    %rdi
    pop    %rdx
    pop    %rcx
    pop    %rax

    // iretq would work here, too

    swapgs
    pop    %rcx             // Pop rflags
    pop    %r11             // Pop userspace code segment
    pop    %r11             // Pop userspace return pointer
    popq   %gs:0x1c         // Pop userspace rsp
    mov    %gs:0x1c,%rsp    // Restore userspace rsp
    swapgs
    sysretq
