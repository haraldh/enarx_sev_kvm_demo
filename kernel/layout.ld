ENTRY(_start)

PHDRS
{
  rodata PT_LOAD FILEHDR PHDRS ;
  text   PT_LOAD               ;
  data   PT_LOAD               ;
}


/* Loaders like to put stuff in low memory (< 1M), so we don't use it. */
ram_min = 1M;
ram_max = 2M;
/* Our stack grows down from ram_max. TODO: Add a guard for stack overflows. */
stack_size = 64K;

/* Pagetable locations loaded by Firecracker/cloud-hypervisor */
PML4T = 0x9000;
PML3T = 0xa000;

/* Pagetable location loaded by enarx */
PML2T = 0xb000;

SECTIONS
{

	. = ram_min;
	. += SIZEOF_HEADERS;

	.rodata ALIGN(4K) : { *(.rodata .rodata.*) *(.eh_frame .eh_frame.*) } :rodata
	.text   ALIGN(4K) : { *(.text .text.*)                              } :text
	.data   ALIGN(4K) : { *(.data .data.*) *(.bss .bss.*) KEEP(*(.app)) } :data

	/* firmware_ram_size = . - ram_min; */

	ASSERT((. <= ram_max - stack_size), "kernel size too big for RAM region")

	/* Match edk2's GccBase.lds DISCARD section */
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
