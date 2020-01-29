stack_size = 0xF000

.section .ram64, "ax"
.global ram64_start
.code64

ram64_start:
    # Indicate (via serial) that we are in long/64-bit mode
    #movw $0x2f8, %dx
    #movb $'L', %al
    #outb %al, %dx
    #movb $'\n', %al
    #outb %al, %dx

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
    movq $pvh_stack, %rsp
    addq $stack_size-16, %rsp

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
