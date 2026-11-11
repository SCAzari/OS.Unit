# Unit OS build entry point. Usage:
#   make                 configure (if needed) and build the default image
#   make CONFIG=<name>   build another configuration from external/configs (without _defconfig)
#   make smoke           boot the emulation image in QEMU and run the smoke check
#   make menuconfig      change the configuration; save with 'make savedefconfig'
CONFIG ?= unit_qemu_x86_64
O ?= $(CURDIR)/output/$(CONFIG)
BR := $(MAKE) -C $(CURDIR)/buildroot BR2_EXTERNAL=$(CURDIR)/external O=$(O)

all: $(O)/.config
	$(BR)

$(O)/.config: external/configs/$(CONFIG)_defconfig
	$(BR) $(CONFIG)_defconfig

menuconfig savedefconfig linux-menuconfig legal-info source: $(O)/.config
	$(BR) $@

smoke:
	scripts/qemu-smoke.sh $(O)/images

clean:
	rm -rf $(O)

.PHONY: all smoke clean menuconfig savedefconfig linux-menuconfig legal-info source
