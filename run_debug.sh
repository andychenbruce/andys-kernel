#!/usr/bin/env sh

../poo/bin/riscv64-unknown-elf-gdb --se=./target/riscv64gc-unknown-none-elf/debug/chad_os --eval-command="target remote localhost:1234"
