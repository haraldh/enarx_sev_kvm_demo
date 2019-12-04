ENTRY(_start)

PHDRS
{
  rodata PT_LOAD FILEHDR PHDRS ;
  data   PT_LOAD               ;
  tdata  PT_LOAD               ;
  text   PT_LOAD               ;
}

SECTIONS
{
       . = 0x40000000;
       . += SIZEOF_HEADERS;

       .rodata ALIGN(4K) : { *(.rodata .rodata.*)                } :rodata
       .data   ALIGN(4K) : { *(.data .data.*) *(.bss .bss.*)     } :data
       .tdata  ALIGN(4K) : { *(.tdata .tdata.*) *(.tbss .tbss.*) } :tdata
       .text   ALIGN(4K) : { *(.text .text.*)                    } :text

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
