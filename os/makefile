TARGET := riscv64gc-unknown-none-elf
KERNEL_ELF := target/$(TARGET)/release/os
KERNEL_BIN := $(KERNEL_ELF).bin
BOOTLOADER := ../bootloader/rustsbi-qemu.bin
KERNEL_ENTRY_PA := 0x80200000
QEMU_ARGS := -machine virt \
			 -nographic \
			 -bios $(BOOTLOADER) \
			 -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)

kernel:
	@cargo build --release

$(KERNEL_BIN): kernel
	@rust-objcopy --strip-all $(KERNEL_ELF) -O binary $@ 

build: $(KERNEL_BIN)

run: build
	@qemu-system-riscv64 $(QEMU_ARGS)

clean:
	@cargo clean

debug: build
	@tmux new-session -d \
		"qemu-system-riscv64 $(QEMU_ARGS) -s -S" && \
		tmux split-window -h "riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'" && \
		tmux -2 attach-session -d
