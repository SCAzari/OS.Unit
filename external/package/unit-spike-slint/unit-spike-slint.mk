################################################################################
#
# unit-spike-slint: spike screens in Slint (toolkit spike, UNIT-OS-WP-0010)
#
################################################################################

UNIT_SPIKE_SLINT_VERSION = 0.1.0
UNIT_SPIKE_SLINT_SITE = $(BR2_EXTERNAL_UNIT_PATH)/package/unit-spike-slint/src
UNIT_SPIKE_SLINT_SITE_METHOD = local
UNIT_SPIKE_SLINT_LICENSE = Proprietary (Unit; spike code, never released); Slint under GPL-3.0-only, royalty-free or paid licence (licence decision needed, D-09); the licences of the vendored crates
UNIT_SPIKE_SLINT_DEPENDENCIES = host-pkgconf libxkbcommon wayland fontconfig mesa3d

# A local package is not vendored at download time: fetch the crates pinned by Cargo.lock first.
define UNIT_SPIKE_SLINT_FETCH_CRATES
	cd $(@D) && \
	PATH=$(BR_PATH) CARGO_HOME=$(BR_CARGO_HOME) RUSTC=$(HOST_DIR)/bin/rustc \
		$(HOST_DIR)/bin/cargo fetch --locked --manifest-path Cargo.toml
endef
UNIT_SPIKE_SLINT_PRE_BUILD_HOOKS += UNIT_SPIKE_SLINT_FETCH_CRATES

$(eval $(cargo-package))
