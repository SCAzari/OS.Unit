################################################################################
#
# unit-spike-qt: spike screens in Qt 6 QML (toolkit spike, UNIT-OS-WP-0010)
#
################################################################################

UNIT_SPIKE_QT_VERSION = 0.1.0
UNIT_SPIKE_QT_SITE = $(BR2_EXTERNAL_UNIT_PATH)/package/unit-spike-qt/src
UNIT_SPIKE_QT_SITE_METHOD = local
UNIT_SPIKE_QT_LICENSE = Proprietary (Unit; spike code, never released)
UNIT_SPIKE_QT_DEPENDENCIES = qt6base qt6declarative qt6wayland qt6shadertools

$(eval $(cmake-package))

# Qt detects its AT-SPI accessibility bridge when it is configured; Buildroot's qt6base does not
# depend on at-spi2-core, so make sure the bus library is built first.
ifeq ($(BR2_PACKAGE_AT_SPI2_CORE),y)
$(QT6BASE_TARGET_CONFIGURE): | at-spi2-core
QT6BASE_CONF_OPTS += -DFEATURE_accessibility_atspi_bridge=ON
endif
