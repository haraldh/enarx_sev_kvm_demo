OUTPUT_FORMAT(elf64-x86-64)

ENTRY(_pto_start)

PHDRS
{
    pvh_notes   PT_NOTE FLAGS(0);
    pvh_text    PT_LOAD;
    pvh_bss     PT_LOAD;

    rodata  PT_LOAD;
    data    PT_LOAD;
    text    PT_LOAD;
    bss     PT_LOAD;

    app     PT_LOAD;
}

KERNEL_OFFSET = 0x80000000000;

/* Loaders like to put stuff in low memory (< 1M), so we don't use it. */
ram_min = 1M;
ram_max = 4M;

/* Pagetable locations loaded by crosvm/Firecracker/cloud-hypervisor/enarx */
pml4t = 0x9000;
pml3t = 0xa000;
pml2t = 0xb000;
pml3to = 0xf000;

MEMORY {
    rom : ORIGIN = 1M, LENGTH = 1M
    kernel: ORIGIN = (2M + 0x80000000000), LENGTH = 2M
    app : ORIGIN = (4M + 0x80000000000), LENGTH = 4M
}

SECTIONS
{
    . = 1M;
    .pvh_notes  : { KEEP(*(.note .note.* .notes)) } >rom :pvh_notes
    .pvh_text   : { KEEP(*(.ram64 .ram32))
                    KEEP(*(.entry64))             } >rom :pvh_text
    .pvh_bss    : { KEEP(*(.bss.stack))           } >rom :pvh_bss

    .rodata : AT(ADDR(.rodata) - KERNEL_OFFSET) { *(.rodata .rodata.*)            } >kernel :rodata
    .data   : AT(ADDR(.data)   - KERNEL_OFFSET) { *(.data .data.*) *(.got .got.*) } >kernel :data
    .text   : AT(ADDR(.text)   - KERNEL_OFFSET) { *(.text .text.*)                } >kernel :text
    .bss    : AT(ADDR(.bss)    - KERNEL_OFFSET) { *(.bss .bss.*)                  } >kernel :bss

    _kernel_start = ADDR(.rodata) - KERNEL_OFFSET;
    _kernel_end = . - KERNEL_OFFSET;

    .appdata  : AT(ADDR(.appdata) - KERNEL_OFFSET) { KEEP(*(.app)) } >app :app

    /DISCARD/ : {
        *(.eh_frame*)
        *(.note.GNU-stack)
        *(.gnu_debuglink)
        *(.interp)
        *(.dynsym)
        *(.dynstr)
        *(.dynamic)
        *(.hash .gnu.hash)
        *(.comment)
        *(COMMON)
    }
}
