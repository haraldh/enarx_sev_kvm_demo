.section .rom, "ax"

# This ROM will be mapped right at the end of the 32-bit address space, but the
# linker assumes all code executes in RAM, and gives symbols addresses in that
# range. To get around this, we manully compute ROM addresses.
gdt32_addr32      = (1 << 32) - (rom_end - gdt32_start)
rom32_addr32      = (1 << 32) - (rom_end - rom32_start)
gdt32_ptr_addr16  = (1 << 16) - (rom_end - gdt32_ptr)

gdt32_ptr:
    .short gdt32_end - gdt32_start - 1 # GDT length is actually (length - 1)
    .long gdt32_addr32
# Note: Out GDT descriptors must be marked "accessed", or the processor will
#       hang when it attempts to update them (as the gdt32 is in ROM).
gdt32_start:
    .quad 0          # First descriptor is always unused
code32_desc: # base = 0x00000000, limit = 0xfffff x 4K
    .short 0xffff    # limit[0..16] = 0xffff
    .short 0x0000    # base [0..16] = 0x0000
    .byte 0x00       # base[16..24] = 0x00
    .byte 0b10011011 # present, DPL = 0, system, code seg, grows up, readable, accessed
    .byte 0b11001111 # 4K gran, 32-bit, limit[16..20] = 0x1111 = 0xf
    .byte 0x00       # base[24..32] = 0x00
data32_desc: # base = 0x00000000, limit = 0xfffff x 4K
    .short 0xffff    # limit 15:0
    .short 0x0000    # base 15:0
    .byte 0x00       # base[16..24] = 0x00
    .byte 0b10010011 # present, DPL = 0, system, data seg, ring0 only, writable, accessed
    .byte 0b11001111 # 4K gran, 32-bit, limit[16..20] = 0x1111 = 0xf
    .byte 0x00       # base[24..32] = 0x00
gdt32_end:

.code32
rom32_start:
    # Now that we are in 32-bit mode, setup all the data segments to be 32-bit.
    movw $(data32_desc - gdt32_start), %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs

    # The rest of the firmware assumes it executes from RAM in a region just
    # above ram_min, so we copy all of that code into RAM and jump to it.
    movl $ram_min, %edi
    # Ideally we would define:
    #   rom_min = (1 << 32) - firmware_rom_size
    # above, and just do
    #   movl $rom_min, %esi
    # However, firmware_rom_size is not known until link time, so the assembler
    # can't handle such code. Thus, the firmware has to do the addreess math.
    xorl %esi, %esi
    # For 32-bit registers: 0 - offset = (1 << 32) - offset
    subl $firmware_rom_size, %esi
    movl $firmware_ram_size, %ecx

    # This code is essentially: memcpy(ram_min, rom_min, firmware_ram_size)
    cld
    rep movsb (%esi), (%edi)

    # Jumping all that way from ROM (~4 GiB) to RAM (~1 MiB) is too far for a
    # relative jump, so we use an aboslute jump.
    movl $ram32_start, %eax
    jmpl *%eax

.code16
rom16:
    # Order of instructions from Intel SDM 9.9.1 "Switching to Protected Mode"
    # Step 1: Disable interrupts
    cli

    # Step 2: Load the GDT
    # We are currently in 16-bit real mode. To enter 32-bit protected mode, we
    # need to load 32-bit code/data segments into our GDT. The gdt32 in ROM is
    # at too high of an address (4 GiB - offset) for the data segment to reach.
    # So, we load gdt32 via the 16-bit code segement, using a 16-bit address.
    movw  $gdt32_ptr_addr16, %bx
    lgdtl %cs:(%bx)

    # Step 3: Set CRO.PE (Protected Mode Enable)
    movl %cr0, %eax
    orb  $0b00000001, %al # Set bit 0
    movl %eax, %cr0

    # Step 4: Far JMP to change execution flow and serializes the processor.
    # Set CS to a 32-bit segment and jump to 32-bit code.
    ljmpl $(code32_desc - gdt32_start), $rom32_addr32

.align 16
reset_vector: # 0xffff_fff0
    jmp rom16
.align 16
rom_end: # 0x1_0000_0000
