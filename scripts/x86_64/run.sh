#!/usr/bin/env bash
set -e


rm --force --recursive ./esp
mkdir --parents ./esp/efi/boot
mkdir --parents ./esp/efi/kernel

cargo build --bin bootloader --target x86_64-unknown-uefi
cargo build --bin kernel --target x86_64-unknown-none

cp ../../target/x86_64-unknown-uefi/debug/bootloader.efi ./esp/efi/boot/bootx64.efi
cp ../../target/x86_64-unknown-none/debug/kernel ./esp/efi/kernel

printf "hihi\n你好\b" > ./esp/poo.txt
qemu-system-x86_64 \
    -enable-kvm \
    -nodefaults \
    -nographic \
    -chardev file,id=andy_out,path="/tmp/andy_log.txt" \
    -serial chardev:andy_out \
    -debugcon mon:stdio \
    -drive if=pflash,format=raw,readonly=on,file=$HOME/.guix-home/profile/share/firmware/ovmf_x64.bin \
    -drive format=raw,file=fat:rw:esp
