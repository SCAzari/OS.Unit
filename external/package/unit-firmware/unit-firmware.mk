################################################################################
#
# unit-firmware
#
################################################################################

UNIT_FIRMWARE_VERSION = 20261019
UNIT_FIRMWARE_SOURCE = linux-firmware-$(UNIT_FIRMWARE_VERSION).tar.xz
UNIT_FIRMWARE_SITE = https://cdn.kernel.org/pub/linux/kernel/firmware
UNIT_FIRMWARE_LICENSE = Intel redistributable firmware licences
UNIT_FIRMWARE_LICENSE_FILES = LICENSE.xe LICENSE.i915 WHENCE
UNIT_FIRMWARE_REDISTRIBUTE = YES

UNIT_FIRMWARE_FILES = \
	xe/ptl_guc_70.bin \
	xe/ptl_huc.bin \
	xe/ptl_gsc_1.bin \
	i915/xe3lpd_dmc.bin \
	intel/ipu/ipu7ptl_fw.bin

define UNIT_FIRMWARE_INSTALL_TARGET_CMDS
	for f in $(UNIT_FIRMWARE_FILES); do \
		$(INSTALL) -D -m 0644 $(@D)/$$f $(TARGET_DIR)/lib/firmware/$$f || exit 1; \
	done
	$(INSTALL) -D -m 0644 $(@D)/LICENSE.xe $(TARGET_DIR)/lib/firmware/LICENSE.xe
	$(INSTALL) -D -m 0644 $(@D)/LICENSE.i915 $(TARGET_DIR)/lib/firmware/LICENSE.i915
endef

$(eval $(generic-package))
