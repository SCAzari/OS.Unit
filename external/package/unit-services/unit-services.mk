################################################################################
#
# unit-services: Settings, Notification and Session services (UNIT-OS-WP-0009)
#
################################################################################

UNIT_SERVICES_VERSION = 0.1.0
UNIT_SERVICES_SITE = $(BR2_EXTERNAL_UNIT_PATH)/package/unit-services/src
UNIT_SERVICES_SITE_METHOD = local
UNIT_SERVICES_LICENSE = Proprietary (Unit; licence to be set under Design decision D-09); the licences of the vendored crates

# A local package has no download step, so Buildroot does not vendor its crates. Fetch the
# crates pinned by Cargo.lock into Buildroot's Cargo home (in the download folder) as part of
# `make source`, so that a later first build needs no network. Buildroot's PATH puts the host
# rustc, which cargo runs, first.
UNIT_SERVICES_CARGO = PATH=$(BR_PATH) CARGO_HOME=$(BR_CARGO_HOME) RUSTC=$(HOST_DIR)/bin/rustc \
	flock $(BR_CARGO_HOME)/.br-lock $(HOST_DIR)/bin/cargo

unit-services-fetch-crates: host-rustc
	mkdir -p $(BR_CARGO_HOME)
	cd $(UNIT_SERVICES_SITE) && \
		$(UNIT_SERVICES_CARGO) fetch --locked --manifest-path Cargo.toml
unit-services-source: unit-services-fetch-crates
.PHONY: unit-services-fetch-crates

# Before building, use the crates already fetched without the network; go online only if any
# are missing (a build without a prior `make source`).
define UNIT_SERVICES_FETCH_CRATES
	mkdir -p $(BR_CARGO_HOME)
	cd $(@D) && \
	{ $(UNIT_SERVICES_CARGO) fetch --locked --offline --manifest-path Cargo.toml || \
	  $(UNIT_SERVICES_CARGO) fetch --locked --manifest-path Cargo.toml; }
endef
UNIT_SERVICES_PRE_BUILD_HOOKS += UNIT_SERVICES_FETCH_CRATES

# legal-info archives only a local package's own source folder, so the crates it is built from
# would be missing from the redistributable sources. Vendor the crates pinned by Cargo.lock and
# archive them beside the package's sources, as Buildroot does for downloaded cargo packages.
UNIT_SERVICES_CRATES = $(UNIT_SERVICES_BASENAME_RAW)-crates
define UNIT_SERVICES_LEGAL_INFO_CRATES
	rm -rf $(UNIT_SERVICES_DIR)/.legal-info-crates
	mkdir -p $(UNIT_SERVICES_DIR)/.legal-info-crates $(BR_CARGO_HOME)
	cd $(UNIT_SERVICES_DIR) && \
	$(UNIT_SERVICES_CARGO) vendor --locked --offline --manifest-path Cargo.toml \
		$(UNIT_SERVICES_DIR)/.legal-info-crates/VENDOR >/dev/null
	mkdir -p $(UNIT_SERVICES_REDIST_SOURCES_DIR)
	. support/download/helpers; cd $(UNIT_SERVICES_DIR); TAR=$(TAR) mk_tar_gz \
		$(UNIT_SERVICES_DIR)/.legal-info-crates/ $(UNIT_SERVICES_CRATES) @0 \
		$(UNIT_SERVICES_REDIST_SOURCES_DIR)/$(UNIT_SERVICES_CRATES).tar.gz
	rm -rf $(UNIT_SERVICES_DIR)/.legal-info-crates
endef
UNIT_SERVICES_POST_LEGAL_INFO_HOOKS += UNIT_SERVICES_LEGAL_INFO_CRATES
# legal-info only extracts the package, so make sure the crates (and cargo) are there.
unit-services-legal-info: unit-services-fetch-crates

# The manifest row names the source archive of a downloaded cargo package
# (<name>-<version>-cargo4.tar.gz), which a local package does not have. Name the archives that
# legal-info actually saved for it: its own sources and the crates.
define UNIT_SERVICES_LEGAL_INFO_MANIFEST
	sed -i 's#^\("unit-services","$(UNIT_SERVICES_VERSION)","[^"]*","[^"]*",\)"[^"]*"#\1"$(UNIT_SERVICES_BASENAME_RAW).tar.gz $(UNIT_SERVICES_CRATES).tar.gz"#' \
		$(LEGAL_MANIFEST_CSV_TARGET)
	grep -q '^"unit-services",.*"$(UNIT_SERVICES_BASENAME_RAW).tar.gz $(UNIT_SERVICES_CRATES).tar.gz"' \
		$(LEGAL_MANIFEST_CSV_TARGET)
endef
UNIT_SERVICES_POST_LEGAL_INFO_HOOKS += UNIT_SERVICES_LEGAL_INFO_MANIFEST

# Systemd user services, started by D-Bus activation when first called. Buildroot's dbus-broker
# package installs the user bus service but not its socket, so the user session bus is set up
# here too (the standard dbus.socket user unit).
define UNIT_SERVICES_INSTALL_INIT_SYSTEMD
	$(INSTALL) -D -m 0644 $(UNIT_SERVICES_PKGDIR)/systemd/unit-settings.service \
		$(TARGET_DIR)/usr/lib/systemd/user/unit-settings.service
	$(INSTALL) -D -m 0644 $(UNIT_SERVICES_PKGDIR)/systemd/unit-notifications.service \
		$(TARGET_DIR)/usr/lib/systemd/user/unit-notifications.service
	$(INSTALL) -D -m 0644 $(UNIT_SERVICES_PKGDIR)/systemd/unit-session.service \
		$(TARGET_DIR)/usr/lib/systemd/user/unit-session.service
	$(INSTALL) -D -m 0644 $(UNIT_SERVICES_PKGDIR)/dbus/os.unit.Settings1.service \
		$(TARGET_DIR)/usr/share/dbus-1/services/os.unit.Settings1.service
	$(INSTALL) -D -m 0644 $(UNIT_SERVICES_PKGDIR)/dbus/org.freedesktop.Notifications.service \
		$(TARGET_DIR)/usr/share/dbus-1/services/org.freedesktop.Notifications.service
	$(INSTALL) -D -m 0644 $(UNIT_SERVICES_PKGDIR)/dbus/os.unit.Notifications1.service \
		$(TARGET_DIR)/usr/share/dbus-1/services/os.unit.Notifications1.service
	$(INSTALL) -D -m 0644 $(UNIT_SERVICES_PKGDIR)/dbus/os.unit.Session1.service \
		$(TARGET_DIR)/usr/share/dbus-1/services/os.unit.Session1.service
	$(INSTALL) -D -m 0644 $(UNIT_SERVICES_PKGDIR)/systemd/dbus.socket \
		$(TARGET_DIR)/usr/lib/systemd/user/dbus.socket
	mkdir -p $(TARGET_DIR)/usr/lib/systemd/user/sockets.target.wants
	ln -sf ../dbus.socket $(TARGET_DIR)/usr/lib/systemd/user/sockets.target.wants/dbus.socket
endef

$(eval $(cargo-package))
