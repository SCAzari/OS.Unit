################################################################################
#
# unit-firmware
#
################################################################################

UNIT_FIRMWARE_VERSION = 20261019
UNIT_FIRMWARE_SOURCE = linux-firmware-$(UNIT_FIRMWARE_VERSION).tar.xz
UNIT_FIRMWARE_SITE = https://cdn.kernel.org/pub/linux/kernel/firmware
UNIT_FIRMWARE_LICENSE = Intel redistributable firmware licences
UNIT_FIRMWARE_LICENSE_FILES = LICENSES/LICENSE.xe LICENSES/LICENSE.i915 LICENSES/LICENSE.intel WHENCE
UNIT_FIRMWARE_REDISTRIBUTE = YES

# Versions in this release (WHENCE): GuC 70.72.1, HuC 10.3.3,
# GSC 105.0.2.1397 SVN 1, DMC Xe3LPD release 36, IPU7 PTL 1.2.1.20251215.
# Licences: LICENSE.xe (xe/), LICENSE.i915 (i915/), LICENSE.intel (intel/ipu/).
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
	for l in LICENSE.xe LICENSE.i915 LICENSE.intel; do \
		$(INSTALL) -D -m 0644 $(@D)/LICENSES/$$l $(TARGET_DIR)/lib/firmware/LICENSES/$$l || exit 1; \
	done
endef

$(eval $(generic-package))
