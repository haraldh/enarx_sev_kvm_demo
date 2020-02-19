.section .entry64, "ax"
.global _start
.global _setup_pto
.code64

.p2align 4
_start:
    # Indicate (via serial) that we are in long/64-bit mode
    /*
    movw $0x2f8, %dx
    movb $'L', %al
    outb %al, %dx
    movb $'\n', %al
    outb %al, %dx
    */

    movabs $_start_main,%rax


# %rax  = jmp to start function
# %rdi  = first parameter for start function
.p2align 4
_setup_pto:
    mov    %rdi, %r11
    mov    %rax, %r12

/*
        Cr4::update(|f| {
            f.insert(
                Cr4Flags::FSGSBASE
                    | Cr4Flags::PHYSICAL_ADDRESS_EXTENSION
                    | Cr4Flags::OSFXSR
                    | Cr4Flags::OSXMMEXCPT_ENABLE
                    | Cr4Flags::OSXSAVE,
            )
        });
        Cr0::update(|cr0| {
            cr0.insert(
                Cr0Flags::PROTECTED_MODE_ENABLE | Cr0Flags::NUMERIC_ERROR | Cr0Flags::PAGING,
            );
            cr0.remove(Cr0Flags::EMULATE_COPROCESSOR | Cr0Flags::MONITOR_COPROCESSOR)
        });

        Efer::update(|efer| {
            efer.insert(
                EferFlags::LONG_MODE_ACTIVE
                    | EferFlags::LONG_MODE_ENABLE
                    | EferFlags::NO_EXECUTE_ENABLE
                    | EferFlags::SYSTEM_CALL_EXTENSIONS,
            )
        });
*/
    mov    %cr4,%rax
    or     $0x50620,%rax
    mov    %rax,%cr4

    mov    %cr0,%rax
    and    $0x60050008,%eax
    mov    $0x80000021,%ecx
    or     %rax,%rcx
    mov    %rcx,%cr0

    mov    $0xc0000080,%ecx
    rdmsr
    or     $0xd01,%eax
    mov    $0xc0000080,%ecx
    wrmsr

    # setup physical offset page table
    movl $pml3to, %eax
    orb  $0b00000011, %al # writable (bit 1), present (bit 0)
    movl $pml4t, %edx
    addl $128, %edx
    movl %eax, (%edx)

    # setup pml3to
    movabs $0x100000000,%r8
    mov    $pml3to,%ecx
    lea    -0x3ffffe7d(%r8),%rdx
    mov    $0x3,%esi
    mov    $0x183,%edi
.L2:
    mov    %rdi,-0x18(%rcx,%rsi,8)
    lea    -0x80000000(%rdx),%rax
    mov    %rax,-0x10(%rcx,%rsi,8)
    lea    -0x40000000(%rdx),%rax
    mov    %rax,-0x8(%rcx,%rsi,8)
    mov    %rdx,(%rcx,%rsi,8)
    add    %r8,%rdx
    add    $0x4,%rsi
    add    %r8,%rdi
    cmp    $0x203,%rsi
    jne    .L2

_before_jump:
    mov    %r11, %rdi
    mov    %r12, %rax

    # align stack
    addq   $8, %rsp
    andq   $0xFFFFFFFFFFFFFFC0, %rsp
    subq   $8, %rsp
    # align rbp
    addq   $8, %rbp
    andq   $0xFFFFFFFFFFFFFFC0, %rbp
    subq   $8, %rbp


    # jump into kernel address space
    jmpq *%rax

.pto_halt_loop:
    hlt
    jmp .pto_halt_loop
