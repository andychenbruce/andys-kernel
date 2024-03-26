#!/usr/bin/env bash
set -e

cargo build --bin bootloader --target x86_64-unknown-uefi
cargo build --bin kernel --target x86_64-unknown-none
