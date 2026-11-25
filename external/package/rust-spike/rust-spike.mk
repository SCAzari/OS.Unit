################################################################################
#
# rust-spike: a newer Rust for the toolkit spike's Slint build only (UNIT-OS-WP-0010)
#
# Slint 1.18 (the first with text runs for assistive technology) needs Rust 1.92; Buildroot
# 2026.02 provides 1.88. This installs Rust 1.97.1 (the version Buildroot 2026.08 ships) into its
# own folder, $(HOST_DIR)/opt/rust-spike. Only unit-spike-slint uses it; Buildroot's own Rust and
# every other package are unchanged. Spike branch only (Executive's decision, 25 Nov 2026).
#
################################################################################

RUST_SPIKE_VERSION = 1.97.1
RUST_SPIKE_SITE = https://static.rust-lang.org/dist
RUST_SPIKE_SOURCE = rust-$(RUST_SPIKE_VERSION)-x86_64-unknown-linux-gnu.tar.xz
RUST_SPIKE_LICENSE = Apache-2.0 or MIT
RUST_SPIKE_LICENSE_FILES = LICENSE-APACHE LICENSE-MIT

define HOST_RUST_SPIKE_INSTALL_CMDS
	cd $(@D) && ./install.sh --prefix=$(HOST_DIR)/opt/rust-spike \
		--components=rustc,cargo,rust-std-x86_64-unknown-linux-gnu \
		--disable-ldconfig
endef

$(eval $(host-generic-package))
