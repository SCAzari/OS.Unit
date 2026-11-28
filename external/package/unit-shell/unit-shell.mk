################################################################################
#
# unit-shell: the Unit Shell UI (UNIT-OS-WP-0009)
#
################################################################################

UNIT_SHELL_VERSION = 0.1.0
UNIT_SHELL_SITE = $(BR2_EXTERNAL_UNIT_PATH)/package/unit-shell/src
UNIT_SHELL_SITE_METHOD = local
UNIT_SHELL_LICENSE = Proprietary (Unit; licence to be set under Design decision D-09)
UNIT_SHELL_DEPENDENCIES = layer-shell-qt qt6base qt6declarative qt6svg qt6wayland
UNIT_SHELL_CONF_OPTS = -DBUILD_TESTING=OFF

# The compositor starts the Shell UI (Executive decision, 26 Nov 2026), through a start script
# that leaves it out on display-check boots (their screenshots are of the compositor alone), and
# the Shell UI check (make shell), which runs on boots with unit.shell_check=1.
define UNIT_SHELL_INSTALL_START
	$(INSTALL) -D -m 0755 $(UNIT_SHELL_PKGDIR)/unit-compositor-start \
		$(TARGET_DIR)/usr/libexec/unit/unit-compositor-start
	$(INSTALL) -D -m 0644 $(UNIT_SHELL_PKGDIR)/systemd/unit-compositor-shell.conf \
		$(TARGET_DIR)/usr/lib/systemd/system/unit-compositor.service.d/shell.conf
	$(INSTALL) -D -m 0755 $(UNIT_SHELL_PKGDIR)/check/unit-shell-check \
		$(TARGET_DIR)/usr/libexec/unit/unit-shell-check
	$(INSTALL) -D -m 0644 $(UNIT_SHELL_PKGDIR)/check/unit-shell-check.service \
		$(TARGET_DIR)/usr/lib/systemd/system/unit-shell-check.service
	$(INSTALL) -D -m 0644 $(UNIT_SHELL_PKGDIR)/check/51-unit-shell.preset \
		$(TARGET_DIR)/usr/lib/systemd/system-preset/51-unit-shell.preset
endef
UNIT_SHELL_POST_INSTALL_TARGET_HOOKS += UNIT_SHELL_INSTALL_START

$(eval $(cmake-package))
