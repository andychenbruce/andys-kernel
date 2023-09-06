ASFLAGS = -g
LDFLAGS = -Tlinker.ld -nostdlib -L./rust/target/riscv64gc-unknown-none-elf/debug -g
LDLIBS = -lchad_os

AS = riscv64-unknown-elf-as
LD = riscv64-unknown-elf-ld

BUILD_DIR = ./build

KERNEL_IMG = $(BUILD_DIR)/kernel.elf


RUN = qemu-system-riscv64 -machine virt -bios none -kernel $(KERNEL_IMG) -serial mon:stdio -nographic

OBJS = $(BUILD_DIR)/entry.o $(BUILD_DIR)/symbols.o

.PHONY: clean run debug


$(KERNEL_IMG): $(OBJS)
	cargo build
	$(LD) $(ASFLAGS) $^ $(LDFLAGS) $(LDLIBS) -o $@

$(BUILD_DIR)/entry.o: assembly/entry.S
	$(AS) $(ASFLAGS) -c $< -o $@

$(BUILD_DIR)/symbols.o: assembly/symbols.S
	$(AS) $(ASFLAGS) -c $< -o $@

run: $(KERNEL_IMG)
	$(RUN)

debug: kernel.elf
	$(RUN) -gdb tcp::1234 -S

clean:
	$(RM) $(OBJS) $(KERNEL_IMG)

