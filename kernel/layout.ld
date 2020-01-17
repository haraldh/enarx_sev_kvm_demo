ENTRY(_start)
OUTPUT_FORMAT(elf64-x86-64)

EXTERN(rom_end);
EXTERN(XEN_ELFNOTE_PHYS32_PTR);

PHDRS
{
  notes  PT_NOTE               ;
  rodata PT_LOAD FILEHDR PHDRS ;
  data   PT_LOAD               ;
  text   PT_LOAD               ;
  /* thread_local   PT_TLS              ; */
}

/* Loaders like to put stuff in low memory (< 1M), so we don't use it. */
ram_min = 1M;
ram_max = 2M;
/* Our stack grows down from ram_max. TODO: Add a guard for stack overflows. */
stack_size = 64K;

/* Pagetable locations loaded by crosvm/Firecracker/cloud-hypervisor */
pml4t = 0x9000;
pml3t = 0xa000;
pml2t = 0xb000;
pml3to = 0xf000;

SECTIONS
{
	/* Mapping in the program headers makes it easier to mmap the whole file. */
	. = ram_min;

	. += SIZEOF_HEADERS;
    . = ALIGN(4096);

	.notes  : { *(.note .note.* .notes)         } :rodata :notes
	.rodata : { *(.rodata .rodata.*)            } :rodata
	.data   : { *(.data .data.*) *(.bss .bss.*) } :data
	.text   : {
		*(.text .text.*)
		*(.ram64)
		*(.ram32)
	} :text

/* Thread local
 … has an STT_TLS symbol but doesn't have an SHF_TLS section

    .tdata : {
        __tdata_start = .;
        *(.tdata*)
        . = ALIGN(4096);
        __tdata_end = .;
        __tbss_start = .;
        *(.tbss*)
        . += 8;
        . = ALIGN(4096);
        __tbss_end = .;
    } :thread_local
*/

	firmware_ram_size = . - ram_min;

	/* The ROM code must be at the end of the file (for the reset vector), */
	/* and the filesize must a multiple of 64K to boot on QEMU. */
	. = ALIGN(. + SIZEOF(.rom), 64K) - SIZEOF(.rom);
	/* Avoid explictly setting a PHDR, allowing the linker to remove the .rom */
	/* section if it's not emmitted (i.e. the "rom" cargo feature is off). */
	.rom : { KEEP(*(.rom)) }

	firmware_rom_size = . - ram_min;

	/* Memory for identity mapping, keep synced with ADDRESS_SPACE_GIB */
	address_space_gib = 4;
	. = ALIGN(4K);
	. += 4K * address_space_gib;

	ASSERT((. <= ram_max - stack_size), "firmware size too big for RAM region")

	/DISCARD/ : {
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
