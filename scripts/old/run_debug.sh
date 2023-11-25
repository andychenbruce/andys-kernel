#!/usr/bin/env sh

../../../toolchain_bins/bin/riscv64-unknown-elf-gdb --se=../target/riscv64gc-unknown-none-elf/debug/kernel --eval-command="target remote localhost:1234"
