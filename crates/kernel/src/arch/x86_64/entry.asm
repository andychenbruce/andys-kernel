
.section .bss
.align 16
.section .text
.global kentry
.type kentry, @function
kentry:
	mov al, 0x49
	out 0xe9, al
	
	//mov stack_top, esp

	//call kinit

	cli
1:	hlt
	jmp 1b

.size kentry, . - kentry
