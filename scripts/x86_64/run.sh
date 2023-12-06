#!/usr/bin/env bash

rm --force --recursive ./esp
mkdir --parents ./esp/efi/boot
cargo build --bin bootloader --target x86_64-unknown-uefi
cp ../../target/x86_64-unknown-uefi/debug/bootloader.efi ./esp/efi/boot/bootx64.efi
printf "hihi\n你好\b" > ./esp/poo.txt
qemu-system-x86_64 \
    -enable-kvm \
    -drive if=pflash,format=raw,readonly=on,file=/gnu/store/dk4m2z88bhfwj6m4s2jmz3nd2hbnc7q0-ovmf-20170116-1.13a50a6/share/firmware/ovmf_x64.bin \
    -drive format=raw,file=fat:rw:esp
