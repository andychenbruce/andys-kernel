OUTPUT_ARCH(riscv)
ENTRY(kentry)

SECTIONS
{
    . = 0x80000000;
    
    .text ALIGN(4K) : {
        text_start = .;
        *(.init)
        *(.text .text.*)
        text_end = .;
    }

    .rodata ALIGN(4K) : {
        rodata_start = .;
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
	rodata_end = .;
    }

    .data ALIGN(4K) : {
        data_start = .;
        *(.data .data.*)
	global_pointer = .;
        *(.sdata .sdata.*)
        data_end = .;
    }

    .bss ALIGN(4K) : {
        . = ALIGN(4K);
        bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        bss_end = .;
    }

    . = ALIGN(4K);
    PROVIDE(stack_bot = . );
    PROVIDE(stack_top = stack_bot + 0x80000); /* make our stack reasonably big */
	
}
