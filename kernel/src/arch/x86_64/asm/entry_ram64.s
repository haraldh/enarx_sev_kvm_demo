.section .entry64, "ax"
.global _pto_start
.code64

_pto_start:
    movabs $_start,%rax
    # Indicate (via serial) that we are in long/64-bit mode
    #movw $0x2f8, %dx
    #movb $'L', %al
    #outb %al, %dx
    #movb $'\n', %al
    #outb %al, %dx


# %rax  = jmp to start function
# %rdi  = first parameter for start function
_setup_pto:
    mov    %rdi, %r11
    mov    %rax, %r12

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

    mov    %r11, %rdi
    mov    %r12, %rax
    jmpq *%rax

.pto_halt_loop:
    hlt
    jmp .pto_halt_loop
