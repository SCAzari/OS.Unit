# Unit OS build entry point. Usage:
#   make                 configure (if needed) and build the default image
#   make UNIT_IMAGE=<name>   build another configuration from external/configs (without _defconfig)
#   make smoke           boot the emulation image in QEMU and run the smoke check
#   make harness         boot the emulation image in QEMU and run the test harness
#   make menuconfig      change the configuration; save with 'make savedefconfig'
UNIT_IMAGE ?= unit_qemu_x86_64
O ?= $(CURDIR)/output/$(UNIT_IMAGE)
BR := $(MAKE) MAKEOVERRIDES= -C $(CURDIR)/buildroot BR2_EXTERNAL=$(CURDIR)/external O=$(O)

all: $(O)/.config
	$(BR)

$(O)/.config: external/configs/$(UNIT_IMAGE)_defconfig
	$(BR) $(UNIT_IMAGE)_defconfig

menuconfig savedefconfig linux-menuconfig legal-info source: $(O)/.config
	$(BR) $@

smoke:
	scripts/qemu-smoke.sh $(O)/images

harness:
	scripts/qemu-harness.sh $(O)/images

clean:
	rm -rf $(O)

.PHONY: all smoke harness clean menuconfig savedefconfig linux-menuconfig legal-info source
