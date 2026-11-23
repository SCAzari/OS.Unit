################################################################################
#
# unit-compositor: Wayland compositor and window manager (UNIT-OS-WP-0009)
#
################################################################################

UNIT_COMPOSITOR_VERSION = 0.1.0
UNIT_COMPOSITOR_SITE = $(BR2_EXTERNAL_UNIT_PATH)/package/unit-compositor/src
UNIT_COMPOSITOR_SITE_METHOD = local
UNIT_COMPOSITOR_LICENSE = Proprietary (Unit; licence to be set under Design decision D-09); MIT (Smithay) and the licences of the vendored crates
UNIT_COMPOSITOR_DEPENDENCIES = host-pkgconf libdrm libinput libxkbcommon mesa3d seatd udev xkeyboard-config

# A local package is not vendored at download time, so fetch the crates pinned by Cargo.lock
# into Buildroot's Cargo home before the offline build. Buildroot's PATH puts the host rustc,
# which cargo runs, first.
define UNIT_COMPOSITOR_FETCH_CRATES
	cd $(@D) && \
	PATH=$(BR_PATH) CARGO_HOME=$(BR_CARGO_HOME) RUSTC=$(HOST_DIR)/bin/rustc \
		$(HOST_DIR)/bin/cargo fetch --locked --manifest-path Cargo.toml
endef
UNIT_COMPOSITOR_PRE_BUILD_HOOKS += UNIT_COMPOSITOR_FETCH_CRATES

define UNIT_COMPOSITOR_INSTALL_INIT_SYSTEMD
	$(INSTALL) -D -m 0644 $(UNIT_COMPOSITOR_PKGDIR)/unit-compositor.service \
		$(TARGET_DIR)/usr/lib/systemd/system/unit-compositor.service
	$(INSTALL) -D -m 0644 $(UNIT_COMPOSITOR_PKGDIR)/pam-unit-session \
		$(TARGET_DIR)/etc/pam.d/unit-session
endef

$(eval $(cargo-package))
