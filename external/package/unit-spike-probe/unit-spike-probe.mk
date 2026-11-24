################################################################################
#
# unit-spike-probe: reference reader and scanner for the toolkit spike (UNIT-OS-WP-0010)
#
################################################################################

UNIT_SPIKE_PROBE_VERSION = 0.1.0
UNIT_SPIKE_PROBE_SITE = $(BR2_EXTERNAL_UNIT_PATH)/package/unit-spike-probe/src
UNIT_SPIKE_PROBE_SITE_METHOD = local
UNIT_SPIKE_PROBE_LICENSE = Proprietary (Unit; spike code, never released); the licences of the vendored crates

define UNIT_SPIKE_PROBE_FETCH_CRATES
	cd $(@D) && \
	PATH=$(BR_PATH) CARGO_HOME=$(BR_CARGO_HOME) RUSTC=$(HOST_DIR)/bin/rustc \
		$(HOST_DIR)/bin/cargo fetch --locked --manifest-path Cargo.toml
endef
UNIT_SPIKE_PROBE_PRE_BUILD_HOOKS += UNIT_SPIKE_PROBE_FETCH_CRATES

define UNIT_SPIKE_PROBE_INSTALL_SPEC
	$(INSTALL) -D -m 0644 $(BR2_EXTERNAL_UNIT_PATH)/../spike/screens.json \
		$(TARGET_DIR)/usr/share/unit-spike/screens.json
endef
UNIT_SPIKE_PROBE_POST_INSTALL_TARGET_HOOKS += UNIT_SPIKE_PROBE_INSTALL_SPEC

$(eval $(cargo-package))
