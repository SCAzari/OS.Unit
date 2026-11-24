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

# legal-info archives only a local package's own source folder, so the crates it is built from
# would be missing from the redistributable sources. Vendor the crates pinned by Cargo.lock and
# archive them beside the package's sources, as Buildroot does for downloaded cargo packages.
UNIT_COMPOSITOR_CRATES = $(UNIT_COMPOSITOR_BASENAME_RAW)-crates
define UNIT_COMPOSITOR_LEGAL_INFO_CRATES
	rm -rf $(UNIT_COMPOSITOR_DIR)/.legal-info-crates
	mkdir -p $(UNIT_COMPOSITOR_DIR)/.legal-info-crates $(BR_CARGO_HOME)
	cd $(UNIT_COMPOSITOR_DIR) && \
	PATH=$(BR_PATH) CARGO_HOME=$(BR_CARGO_HOME) RUSTC=$(HOST_DIR)/bin/rustc \
		flock $(BR_CARGO_HOME)/.br-lock \
		$(HOST_DIR)/bin/cargo vendor --locked --manifest-path Cargo.toml \
		$(UNIT_COMPOSITOR_DIR)/.legal-info-crates/VENDOR >/dev/null
	mkdir -p $(UNIT_COMPOSITOR_REDIST_SOURCES_DIR)
	. support/download/helpers; cd $(UNIT_COMPOSITOR_DIR); TAR=$(TAR) mk_tar_gz \
		$(UNIT_COMPOSITOR_DIR)/.legal-info-crates/ $(UNIT_COMPOSITOR_CRATES) @0 \
		$(UNIT_COMPOSITOR_REDIST_SOURCES_DIR)/$(UNIT_COMPOSITOR_CRATES).tar.gz
	rm -rf $(UNIT_COMPOSITOR_DIR)/.legal-info-crates
endef
UNIT_COMPOSITOR_POST_LEGAL_INFO_HOOKS += UNIT_COMPOSITOR_LEGAL_INFO_CRATES
# legal-info only extracts the package, so make sure cargo is there to vendor with.
unit-compositor-legal-info: host-rustc

define UNIT_COMPOSITOR_INSTALL_INIT_SYSTEMD
	$(INSTALL) -D -m 0644 $(UNIT_COMPOSITOR_PKGDIR)/unit-compositor.service \
		$(TARGET_DIR)/usr/lib/systemd/system/unit-compositor.service
	$(INSTALL) -D -m 0644 $(UNIT_COMPOSITOR_PKGDIR)/pam-unit-session \
		$(TARGET_DIR)/etc/pam.d/unit-session
endef

$(eval $(cargo-package))
