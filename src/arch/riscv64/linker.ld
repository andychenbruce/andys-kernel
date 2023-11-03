OUTPUT_ARCH(riscv)
ENTRY(kentry)

MEMORY {
       RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 0x08000000
}

PHDRS
{
	text PT_LOAD;
	data PT_LOAD;
	bss PT_LOAD;
}

SECTIONS
{
    .text ALIGN(4K) : {
        text_start = .;
        *(.init)
        *(.text .text.*)
        text_end = .;
    } >RAM AT>RAM :text

    .rodata ALIGN(4K) : {
        rodata_start = .;
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
	rodata_end = .;
    } >RAM AT>RAM :text

    .data ALIGN(4K) : {
        data_start = .;
        *(.data .data.*)
	global_pointer = .;
        *(.sdata .sdata.*)
        data_end = .;
    } >RAM AT>RAM :data

    .bss ALIGN(4K) : {
        . = ALIGN(4K);
        bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        bss_end = .;
    } >RAM AT>RAM :bss

    PROVIDE(memory_start = ORIGIN(RAM));
    PROVIDE(memory_end = ORIGIN(RAM) + LENGTH(RAM));

    . = ALIGN(4K);
    PROVIDE(stack_bot = . );
    PROVIDE(stack_top = stack_bot + 0x80000); /* make our stack reasonably big */
	
    . = ALIGN(4K);
    PROVIDE(heap_start = stack_top);
    PROVIDE(heap_end = memory_end); /* rest of ram goes to heap */
    PROVIDE(heap_size = heap_end - heap_start); /* rest of ram goes to heap */

}
