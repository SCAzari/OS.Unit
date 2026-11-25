################################################################################
#
# unit-spike-slint: spike screens in Slint (toolkit spike, UNIT-OS-WP-0010)
#
################################################################################

UNIT_SPIKE_SLINT_VERSION = 0.1.0
UNIT_SPIKE_SLINT_SITE = $(BR2_EXTERNAL_UNIT_PATH)/package/unit-spike-slint/src
UNIT_SPIKE_SLINT_SITE_METHOD = local
UNIT_SPIKE_SLINT_LICENSE = Proprietary (Unit; spike code, never released); Slint under GPL-3.0-only, royalty-free or paid licence (licence decision needed, D-09); the licences of the vendored crates
UNIT_SPIKE_SLINT_DEPENDENCIES = host-pkgconf libxkbcommon wayland fontconfig mesa3d host-rust-spike

# Slint 1.18 needs Rust 1.92: build this package with the spike's Rust 1.97.1 (package
# rust-spike) instead of Buildroot's 1.88. Buildroot's cross-compiling settings are unchanged.
UNIT_SPIKE_SLINT_RUST = $(HOST_DIR)/opt/rust-spike/bin
UNIT_SPIKE_SLINT_CARGO_ENV = \
	PATH=$(UNIT_SPIKE_SLINT_RUST):$(BR_PATH) \
	RUSTC=$(UNIT_SPIKE_SLINT_RUST)/rustc \
	CARGO=$(UNIT_SPIKE_SLINT_RUST)/cargo

# A local package is not vendored at download time: fetch the crates pinned by Cargo.lock first.
define UNIT_SPIKE_SLINT_FETCH_CRATES
	cd $(@D) && \
	PATH=$(UNIT_SPIKE_SLINT_RUST):$(BR_PATH) CARGO_HOME=$(BR_CARGO_HOME) \
		RUSTC=$(UNIT_SPIKE_SLINT_RUST)/rustc \
		$(UNIT_SPIKE_SLINT_RUST)/cargo fetch --locked --manifest-path Cargo.toml
endef
UNIT_SPIKE_SLINT_PRE_BUILD_HOOKS += UNIT_SPIKE_SLINT_FETCH_CRATES

$(eval $(cargo-package))
