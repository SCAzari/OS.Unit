################################################################################
#
# unit-harness: test harness and trace capture (UNIT-OS-WP-0002)
#
################################################################################

UNIT_HARNESS_VERSION = 0.1.0
UNIT_HARNESS_SITE = $(BR2_EXTERNAL_UNIT_PATH)/package/unit-harness/src
UNIT_HARNESS_SITE_METHOD = local
UNIT_HARNESS_LICENSE = Proprietary (Unit; licence to be set under Design decision D-09)

define UNIT_HARNESS_BUILD_CMDS
	$(TARGET_MAKE_ENV) $(MAKE) CC="$(TARGET_CC)" CFLAGS="$(TARGET_CFLAGS)" -C $(@D)
endef

define UNIT_HARNESS_INSTALL_TARGET_CMDS
	$(TARGET_MAKE_ENV) $(MAKE) -C $(@D) DESTDIR=$(TARGET_DIR) install
endef

$(eval $(generic-package))
