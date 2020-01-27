ENTRY(_start)
OUTPUT_FORMAT(elf64-x86-64)

EXTERN(rom_end);
EXTERN(XEN_ELFNOTE_PHYS32_PTR);
EXTERN(test_syscall_rust);
EXTERN(stack_end);

PHDRS
{
  notes  PT_NOTE               ;
  rodata PT_LOAD               ;
  data   PT_LOAD               ;
  bss    PT_LOAD               ;
  text   PT_LOAD               ;
  app    PT_LOAD               ;
}

/* Loaders like to put stuff in low memory (< 1M), so we don't use it. */
ram_min = 1M;
ram_max = 4M;

/* Pagetable locations loaded by crosvm/Firecracker/cloud-hypervisor */
pml4t = 0x9000;
pml3t = 0xa000;
pml2t = 0xb000;
pml3to = 0xf000;

MEMORY {
    rom : ORIGIN = 1M, LENGTH = 3M
    app : ORIGIN = 4M, LENGTH = 4M
}

SECTIONS
{
	.rodata : { *(.rodata .rodata.*)            } >rom :rodata
	.notes  : { *(.note .note.* .notes)         }  >rom :rodata :notes
	.data   : { *(.data .data.*) *(.got .got.*) } >rom :data
	.text   : {
		*(.text .text.*)
		*(.ram64)
		*(.ram32)
	} > rom :text
	.bss   : { *(.bss .bss.*) } >rom :bss

	firmware_ram_size = . - ram_min;

	/* The ROM code must be at the end of the file (for the reset vector), */
	/* and the filesize must a multiple of 64K to boot on QEMU. */
	. = ALIGN(. + SIZEOF(.rom), 64K) - SIZEOF(.rom);
	/* Avoid explictly setting a PHDR, allowing the linker to remove the .rom */
	/* section if it's not emmitted (i.e. the "rom" cargo feature is off). */
	.rom : { KEEP(*(.rom)) } > rom

	firmware_rom_size = . - ram_min;

	.data  : { KEEP(*(.app)) } >app :app

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
