.set stack_size,      0xF000

.section .ram64, "ax"
.global ram64_start
.code64

ram64_start:
    # Indicate (via serial) that we are in long/64-bit mode
    movw $0x2f8, %dx
    movb $'L', %al
    outb %al, %dx
    movb $'\n', %al
    outb %al, %dx

    # Clear CR0.EM and Set CR0.MP
    movq %cr0, %rax
    andb $0b11111011, %al # Clear bit 2
    orb  $0b00000010, %al # Set bit 1
    movq %rax, %cr0
    # Set CR4.OSFXSR and CR4.OSXMMEXCPT
    movq %cr4, %rax
    orb  $0b00000110, %ah # Set bits 9 and 10
    movq %rax, %cr4

    # Setup some stack
    movq $stack_start, %rsp

    jmp _start_e820

halt_loop:
    hlt
    jmp halt_loop

.section .bss.stack, "a"
.align 16
	stack_end: .skip stack_size
	stack_start: .skip 0