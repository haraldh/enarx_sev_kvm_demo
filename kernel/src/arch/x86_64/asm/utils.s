.section .text, "ax"
.global _context_switch
.type _context_switch, @function
_context_switch:
    movq %rsi, %rsp
    callq  *%rdi
.CSWSP:
    jmp .CSWSP

.section .text, "ax"
.global _read_rsp
.type _read_rsp, @function
_read_rsp:
    movq %rsp, %rax
    retq
