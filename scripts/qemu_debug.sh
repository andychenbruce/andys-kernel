#!/usr/bin/env sh

qemu-system-riscv64 -s -S -machine virt -bios none -serial mon:stdio -nographic -kernel ../target/riscv64gc-unknown-none-elf/debug/chad_os
