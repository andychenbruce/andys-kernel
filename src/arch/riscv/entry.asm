	.section .rodata

	.global MEMORY_START
	.global MEMORY_END
	.global TEXT_START
	.global TEXT_END
	.global RODATA_START
	.global RODATA_END
	.global DATA_START
	.global DATA_END
	.global BSS_START
	.global BSS_END 
	.global STACK_TOP
	.global STACK_BOT
	.global HEAP_START
	.global HEAP_END
	.global HEAP_SIZE
	.global SYSCON_ADDR
	.global UART_ADDR

//MEMORY_START: .dword memory_start
//MEMORY_END: .dword memory_end

TEXT_START: .dword text_start
TEXT_END: .dword text_end
RODATA_START: .dword rodata_start
RODATA_END: .dword rodata_end
DATA_START: .dword data_start
DATA_END: .dword data_end
BSS_START: .dword bss_start
BSS_END: .dword bss_end

STACK_TOP: .dword stack_top
STACK_BOT: .dword stack_bot
HEAP_START: .dword heap_start
HEAP_END: .dword heap_end

SYSCON_ADDR: .dword 0x00100000
UART_ADDR: .dword 0x10000000



	.altmacro
	.macro save_gp i, basereg=t6
	sd x\i, ((\i)*8)(\basereg)
	.endm
	.macro load_gp i, basereg=t6
	ld x\i, ((\i)*8)(\basereg)
	.endm

	.section .init

	.option norvc
	
	.type start, @function
	.global start
start:
	.cfi_startproc

	/* if core not cpu0 skip this and wait for interrupt */
	csrr t0, mhartid
	bnez t0, loop_forever

	/* Reset satp */
	csrw satp, zero

	/* Set global pointer */
	.option push
	.option norelax /* dont optimize, sometimes assumes gp is already initialized */
	la gp, global_pointer
	.option pop

	/* Zero the BSS section */
	la t0, bss_start
	la t1, bss_end
bss_clear:
	sd zero, (t0)
	addi t0, t0, 8
	bleu t0, t1, bss_clear
	
	/* Setup stack */
	la sp, stack_top
	
	/* Jump to kinit */

	li t0, 0b11 << 11
	csrw mstatus, t0
	
	la t0, kinit
	csrw mepc, t0
	
	la ra, done_kinit
	
	mret

done_kinit:

	/* enable traps */
	la t0, andy_trap
	csrw mtvec, t0

	/* Switch to supervisor mode then jump to kmain */
	li t0, (0b01 << 11) | (1 << 5)
	csrw mstatus, t0
	
	la t0, kmain
	csrw mepc, t0

	la ra, loop_forever /* shouldn't return */
	
	mret
	
	.cfi_endproc

loop_forever:
	wfi
	j loop_forever

andy_trap:
	/* backup 31's value to scratch, swapping it with the trap stack address */
	csrrw t6, mscratch, t6

	/* use 31 to save 0 to 30 */
	.set i, 1
	.rept 30
	save_gp %i
	.set i, i+1
	.endr

	/* load 31's backup and save it */
	mv t5, t6
	csrr t6, mscratch
	save_gp 31, t5
	csrw mscratch, t5
	
	/* call into rust trap handler */
	mv sp, t5
	csrr a0, mepc
	csrr a1, mtval
	csrr a2, mcause
	csrr a3, mhartid
	csrr a4, mstatus
	call rust_andy_trap

	/* update mepc to rust return val */
	csrw mepc, a0
	
	/* restore stuff */
	csrr t6, mscratch
	
	.set i, 1
	.rept 31
	save_gp %i
	.set i, i+1
	.endr
	
	mret
	.end




