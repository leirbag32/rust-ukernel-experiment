#!/bin/sh

# How to create the pos.img: qemu-img create -f raw pos.img 64M

qemu-system-aarch64 -M virt -cpu cortex-a72 -m 512M \
	-global virtio-mmio.force-legacy=false \
	-drive if=none,file=pos.img,format=raw,id=pos \
	-device virtio-blk-device,drive=pos \
       	-nographic -kernel target/aarch64-unknown-none/debug/device-os
#qemu-system-aarch64 -M virt -cpu cortex-a72 -m 512M -nographic -kernel target/aarch64-unknown-none/debug/device-os -d int,mmu,cpu_reset -D qemu.log
