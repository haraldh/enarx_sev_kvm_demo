.text
.globl _start
.type _start, @function

_start:
/*
.intel_syntax noprefix
    mov dx, 0x3f8
    mov al, 's'
    out dx, al
    mov dx, 0x3f8
    mov al, '\n'
    out dx, al
.att_syntax
*/
	mov $1,    %rax /* SYS_write */
	mov $1,    %rdi /* STDOUT_FILENO */
	mov $.LC0, %rsi /* string */
	mov $3,    %rdx /* length */
	syscall

    mov $60,   %rax /* SYS_exit */
    mov $0,    %rdi /* exit status */
    syscall

.data
.LC0:
        .string "hi\n"
