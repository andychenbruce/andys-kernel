	/* some symbols from the linker rust will need */
	/* mostly info about the memory layout and heap size */

	.section .rodata

	.global MEMORY_START
MEMORY_START: .dword memory_start
	.global MEMORY_END
MEMORY_END: .dword memory_end

	.global TEXT_START
TEXT_START: .dword text_start
	.global TEXT_END
TEXT_END: .dword text_end
	.global RODATA_START
RODATA_START: .dword rodata_start
	.global RODATA_END
RODATA_END: .dword rodata_end
	.global DATA_START
DATA_START: .dword data_start
	.global DATA_END
DATA_END: .dword data_end

	.global BSS_START
BSS_START: .dword bss_start
	.global BSS_END 
BSS_END: .dword bss_end

	.global STACK_TOP
STACK_TOP: .dword stack_top
	.global STACK_BOT
STACK_BOT: .dword stack_bot

	.global HEAP_START
HEAP_START: .dword heap_start
	.global HEAP_END
HEAP_END: .dword heap_end
	.global HEAP_SIZE
HEAP_SIZE: .dword heap_size







	.global SYSCON_ADDR
SYSCON_ADDR: .dword 0x00100000
	.global UART_ADDR
UART_ADDR: .dword 0x10000000
