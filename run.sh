#!/usr/bin/env bash
set -euo pipefail

# Path to OVMF firmware. Varies by distro, override with: OVMF_CODE=/path ./run.sh
#   Debian/Ubuntu: /usr/share/OVMF/OVMF_CODE.fd
#   Fedora:        /usr/share/edk2/ovmf/OVMF_CODE.fd
#   Arch:          /usr/share/edk2/x64/OVMF_CODE.4m.fd
OVMF_CODE="${OVMF_CODE:-/usr/share/edk2/x64/OVMF_CODE.4m.fd}"

cargo build --target x86_64-unknown-uefi --release -p htmlboot

# Fake ESP: QEMU exposes this folder as a FAT disk.
mkdir -p esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/htmlboot.efi esp/EFI/BOOT/BOOTX64.EFI

qemu-system-x86_64 \
  -machine q35 \
  -m 4G \
  -cpu host \
  -enable-kvm \
  -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
  -drive format=raw,file=fat:rw:esp \
  -device qemu-xhci,id=xhci \
  -device usb-mouse,bus=xhci.0 \
  -chardev stdio,id=char0,mux=on,logfile=debug.log,logappend=on \
  -serial chardev:char0 \
  -monitor chardev:char0
  -vga std
