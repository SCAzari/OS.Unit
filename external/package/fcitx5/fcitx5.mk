################################################################################
#
# fcitx5 (toolkit spike, UNIT-OS-WP-0010)
#
################################################################################

# 5.1.19: the newest release that builds with Buildroot's wayland-protocols 1.45 (5.1.20 and
# later need 1.46).
FCITX5_VERSION = 5.1.19
# The release tarball, not GitHub's archive: it includes the yoga submodule (third_party/yoga).
FCITX5_SITE = https://download.fcitx-im.org/fcitx5/fcitx5
FCITX5_SOURCE = fcitx5-$(FCITX5_VERSION).tar.zst
FCITX5_LICENSE = LGPL-2.1+, Unicode-DFS-2016 (data)
FCITX5_LICENSE_FILES = LICENSES/LGPL-2.1-or-later.txt LICENSES/Unicode-DFS-2016.txt
FCITX5_INSTALL_STAGING = YES
FCITX5_DEPENDENCIES = host-pkgconf host-gettext extra-cmake-modules iso-codes json-for-modern-cpp \
	libxkbcommon xkeyboard-config expat cairo pango gdk-pixbuf librsvg libglib2 systemd \
	wayland wayland-protocols util-linux
FCITX5_CONF_OPTS = \
	-DENABLE_X11=OFF \
	-DENABLE_WAYLAND=ON \
	-DENABLE_DBUS=ON \
	-DUSE_SYSTEMD=ON \
	-DENABLE_KEYBOARD=ON \
	-DENABLE_ENCHANT=OFF \
	-DENABLE_EMOJI=OFF \
	-DENABLE_TEST=OFF \
	-DENABLE_TESTING_ADDONS=ON \
	-DENABLE_DOC=OFF \
	-DENABLE_XDGAUTOSTART=OFF \
	-DBUILD_SPELL_DICT=OFF \
	-DUSE_SYSTEM_PLASMA_WAYLAND_PROTOCOLS=OFF \
	-DENABLE_LIBUUID=ON

$(eval $(cmake-package))
