#!/usr/bin/env bash


qemu-system-x86_64 -enable-kvm \
    -drive if=pflash,format=raw,readonly=on,file=/gnu/store/dk4m2z88bhfwj6m4s2jmz3nd2hbnc7q0-ovmf-20170116-1.13a50a6/share/firmware/ovmf_x64.bin \
    -drive format=raw,file=fat:rw:esp
