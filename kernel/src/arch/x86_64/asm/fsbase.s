.section .text, "ax"
.global _rdfsbase
.type _rdfsbase, @function
_rdfsbase:
    rdfsbase %rax
    retq

.section .text, "ax"
.global _wrfsbase
.type _wrfsbase, @function
.code64
_wrfsbase:
    wrfsbase %rdi
    retq
