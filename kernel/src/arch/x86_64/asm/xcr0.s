.section .text, "ax"
.global _read_xcr0
.type _read_xcr0, @function
_read_xcr0:
    xor    %ecx,%ecx
    xgetbv
    shl    $0x20,%rdx
    mov    %eax,%eax
    or     %rdx,%rax
    retq

.section .text, "ax"
.global _write_xcr0
.type _writex_cr0, @function
.code64
_write_xcr0:
    mov    %rdi,%rax
    mov    %rdi,%rdx
    shr    $0x20,%rdx
    xor    %ecx,%ecx
    xsetbv
    retq
