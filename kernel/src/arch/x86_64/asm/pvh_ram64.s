stack_size = 0xF000

.section .ram64, "ax"
.global ram64_start
.code64

ram64_start:
    # Indicate (via serial) that we are in long/64-bit mode
    /*
    movw $0x2f8, %dx
    movb $'L', %al
    outb %al, %dx
    movb $'\n', %al
    outb %al, %dx
    */

    # Setup some stack
    movq $pvh_stack, %rsp
    addq $stack_size, %rsp

    # HvmStartInfo is in %rbp
    # move to first C argument
    movq %rbx, %rdi
    movabs $_start_e820,%rax
    jmp _setup_pto

.halt_loop:
    hlt
    jmp .halt_loop

.section .bss.stack, "a"
.global pvh_stack
.align 4096
	pvh_stack: .skip stack_size
	pvh_stack_end: .skip 0
