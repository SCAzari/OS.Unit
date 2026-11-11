################################################################################
#
# unit-release: Unit OS identity
#
################################################################################

UNIT_RELEASE_VERSION = 0.0.1
UNIT_RELEASE_SITE = $(BR2_EXTERNAL_UNIT_PATH)/package/unit-release/src
UNIT_RELEASE_SITE_METHOD = local
UNIT_RELEASE_LICENSE = Proprietary (Unit; licence to be set under Design decision D-09)
UNIT_RELEASE_BUILD_ID = $(shell git -C $(BR2_EXTERNAL_UNIT_PATH)/.. describe --always --dirty 2>/dev/null || echo unknown)

define UNIT_RELEASE_INSTALL_TARGET_CMDS
	sed -e 's/@VERSION@/$(UNIT_RELEASE_VERSION)/' -e 's/@BUILD_ID@/$(UNIT_RELEASE_BUILD_ID)/' \
		$(@D)/os-release.in > $(TARGET_DIR)/usr/lib/os-release
	ln -sf ../usr/lib/os-release $(TARGET_DIR)/etc/os-release
	echo "Unit OS $(UNIT_RELEASE_VERSION) ($(UNIT_RELEASE_BUILD_ID))" > $(TARGET_DIR)/etc/unit-release
endef

$(eval $(generic-package))
